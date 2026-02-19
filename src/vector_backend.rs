use crate::error::Result;
use crate::vector_search::{VectorIndex, VectorSearchConfig};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VectorSearchHit {
    pub id: String,
    /// Higher is better.
    pub score: f32,
    pub payload: Option<Value>,
}

#[async_trait]
pub trait VectorBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn dimension(&self) -> usize;

    async fn upsert(&self, id: &str, vector: &[f32], payload: Option<Value>) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<VectorSearchHit>>;
}

/// Lightweight file-backed vector backend used as the default fallback.
pub struct FileVectorBackend {
    index: VectorIndex,
    path: PathBuf,
    dimension: usize,
}

impl FileVectorBackend {
    pub fn new(path: impl AsRef<Path>, dimension: usize) -> Self {
        let path = path.as_ref().to_path_buf();
        let index = VectorIndex::new(VectorSearchConfig {
            dimension,
            index_path: path.clone(),
        });
        Self {
            index,
            path,
            dimension,
        }
    }

    pub async fn ensure_ready(&self) -> Result<()> {
        self.index.init().await
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl VectorBackend for FileVectorBackend {
    fn name(&self) -> &'static str {
        "file"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn upsert(&self, id: &str, vector: &[f32], _payload: Option<Value>) -> Result<()> {
        self.index.store(&id.to_string(), vector.to_vec()).await
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.index.delete(&id.to_string()).await
    }

    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<VectorSearchHit>> {
        let results = self.index.search(vector, limit).await?;
        Ok(results
            .into_iter()
            .map(|(id, score)| VectorSearchHit {
                id,
                score,
                payload: None,
            })
            .collect())
    }
}

#[cfg(feature = "lancedb")]
pub mod lancedb {
    use super::*;
    use crate::error::MemoryError;
    use ::lancedb::connect;
    use ::lancedb::index::vector::{IvfFlatIndexBuilder, IvfPqIndexBuilder};
    use ::lancedb::index::{Index, IndexType};
    use ::lancedb::query::{ExecutableQuery, QueryBase};
    use ::lancedb::table::Table;
    use ::lancedb::DistanceType;
    use arrow_array::{
        types::Float32Type, Array, ArrayRef, FixedSizeListArray, Float32Array, Float64Array,
        RecordBatch, RecordBatchIterator, StringArray,
    };
    use arrow_schema::{DataType, Field, Schema};
    use futures::StreamExt;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AnnIndexKind {
        IvfPq,
        IvfFlat,
    }

    #[derive(Debug, Clone, Copy)]
    struct LanceDbAnnConfig {
        enabled: bool,
        kind: AnnIndexKind,
        distance_type: DistanceType,
        nprobes: usize,
        refine_factor: Option<u32>,
        min_rows_for_index: usize,
    }

    impl Default for LanceDbAnnConfig {
        fn default() -> Self {
            Self {
                enabled: true,
                kind: AnnIndexKind::IvfPq,
                distance_type: DistanceType::Cosine,
                nprobes: 24,
                refine_factor: Some(2),
                min_rows_for_index: 256,
            }
        }
    }

    impl LanceDbAnnConfig {
        fn from_env() -> Self {
            let mut cfg = Self::default();

            if let Ok(v) = std::env::var("GOLDFISH_LANCEDB_ANN") {
                let norm = v.trim().to_lowercase();
                match norm.as_str() {
                    "off" | "none" | "false" | "0" => cfg.enabled = false,
                    "ivfflat" | "ivf_flat" => {
                        cfg.enabled = true;
                        cfg.kind = AnnIndexKind::IvfFlat;
                    }
                    "ivfpq" | "ivf_pq" | "on" | "true" | "1" | "auto" => {
                        cfg.enabled = true;
                        cfg.kind = AnnIndexKind::IvfPq;
                    }
                    _ => {}
                }
            }

            if let Ok(v) = std::env::var("GOLDFISH_LANCEDB_DISTANCE") {
                let norm = v.trim().to_lowercase();
                cfg.distance_type = match norm.as_str() {
                    "l2" => DistanceType::L2,
                    "dot" | "dotproduct" | "dot_product" => DistanceType::Dot,
                    "cosine" => DistanceType::Cosine,
                    _ => cfg.distance_type,
                };
            }

            if let Ok(v) = std::env::var("GOLDFISH_LANCEDB_NPROBES") {
                if let Ok(parsed) = v.parse::<usize>() {
                    cfg.nprobes = parsed.max(1);
                }
            }

            if let Ok(v) = std::env::var("GOLDFISH_LANCEDB_REFINE_FACTOR") {
                if let Ok(parsed) = v.parse::<u32>() {
                    cfg.refine_factor = Some(parsed.max(1));
                }
            }

            if let Ok(v) = std::env::var("GOLDFISH_LANCEDB_ANN_MIN_ROWS") {
                if let Ok(parsed) = v.parse::<usize>() {
                    cfg.min_rows_for_index = parsed.max(1);
                }
            }

            cfg
        }
    }

    #[derive(Debug)]
    pub struct LanceDbVectorBackend {
        path: PathBuf,
        dimension: usize,
        table: RwLock<Option<Table>>,
        ann_cfg: LanceDbAnnConfig,
        ann_index_ready: RwLock<bool>,
    }

    impl LanceDbVectorBackend {
        pub fn new(path: impl AsRef<Path>, dimension: usize) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                dimension,
                table: RwLock::new(None),
                ann_cfg: LanceDbAnnConfig::from_env(),
                ann_index_ready: RwLock::new(false),
            }
        }

        pub async fn ensure_ready(&self) -> Result<()> {
            let _ = self.get_or_init_table().await?;
            Ok(())
        }

        async fn get_or_init_table(&self) -> Result<Table> {
            {
                let guard = self.table.read().await;
                if let Some(t) = guard.as_ref() {
                    return Ok(t.clone());
                }
            }

            let mut guard = self.table.write().await;
            if let Some(t) = guard.as_ref() {
                return Ok(t.clone());
            }

            std::fs::create_dir_all(&self.path)
                .map_err(|e| MemoryError::VectorDb(format!("Failed to create lancedb dir: {e}")))?;

            let uri = self.path.to_string_lossy().to_string();
            let db = connect(&uri)
                .execute()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("Failed to connect to lancedb: {e}")))?;

            let table_name = "goldfish_vectors";
            let table = match db.open_table(table_name).execute().await {
                Ok(t) => t,
                Err(_) => {
                    let schema = Arc::new(Schema::new(vec![
                        Field::new("id", DataType::Utf8, false),
                        Field::new(
                            "vector",
                            DataType::FixedSizeList(
                                Arc::new(Field::new("item", DataType::Float32, true)),
                                self.dimension as i32,
                            ),
                            false,
                        ),
                        Field::new("payload", DataType::Utf8, true),
                    ]));

                    db.create_empty_table(table_name, schema)
                        .execute()
                        .await
                        .map_err(|e| {
                            MemoryError::VectorDb(format!("Failed to create lancedb table: {e}"))
                        })?
                }
            };

            *guard = Some(table.clone());
            Ok(table)
        }

        async fn ensure_ann_index(&self, table: &Table) -> Result<()> {
            if !self.ann_cfg.enabled {
                return Ok(());
            }

            {
                let guard = self.ann_index_ready.read().await;
                if *guard {
                    return Ok(());
                }
            }

            let mut guard = self.ann_index_ready.write().await;
            if *guard {
                return Ok(());
            }

            let existing = table
                .list_indices()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB list_indices failed: {e}")))?;

            let has_vector_ann = existing.iter().any(|idx| {
                idx.columns.iter().any(|c| c == "vector")
                    && matches!(
                        idx.index_type,
                        IndexType::IvfFlat
                            | IndexType::IvfPq
                            | IndexType::IvfRq
                            | IndexType::IvfHnswPq
                            | IndexType::IvfHnswSq
                    )
            });
            if has_vector_ann {
                *guard = true;
                return Ok(());
            }

            let row_count = table
                .count_rows(None)
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB count_rows failed: {e}")))?;
            if row_count < self.ann_cfg.min_rows_for_index {
                return Ok(());
            }

            let index = match self.ann_cfg.kind {
                AnnIndexKind::IvfPq => {
                    Index::IvfPq(IvfPqIndexBuilder::default().distance_type(self.ann_cfg.distance_type))
                }
                AnnIndexKind::IvfFlat => {
                    Index::IvfFlat(
                        IvfFlatIndexBuilder::default().distance_type(self.ann_cfg.distance_type),
                    )
                }
            };

            table
                .create_index(&["vector"], index)
                .name("goldfish_vector_ann".to_string())
                .replace(false)
                .execute()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB ANN index creation failed: {e}")))?;

            *guard = true;
            Ok(())
        }

        fn batch_for_upsert(
            &self,
            id: &str,
            vector: &[f32],
            payload: Option<Value>,
        ) -> Result<RecordBatch> {
            if vector.len() != self.dimension {
                return Err(MemoryError::VectorDb(format!(
                    "Vector dimension mismatch: got {}, expected {}",
                    vector.len(),
                    self.dimension
                )));
            }

            let ids: ArrayRef = Arc::new(StringArray::from(vec![id.to_string()]));

            let list = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                std::iter::once(Some(vector.iter().map(|v| Some(*v)).collect::<Vec<_>>())),
                self.dimension as i32,
            );
            let vectors: ArrayRef = Arc::new(list);

            let payload_str = payload.map(|p| p.to_string());
            let payloads: ArrayRef = Arc::new(StringArray::from(vec![payload_str]));

            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new(
                    "vector",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        self.dimension as i32,
                    ),
                    false,
                ),
                Field::new("payload", DataType::Utf8, true),
            ]));

            RecordBatch::try_new(schema, vec![ids, vectors, payloads])
                .map_err(|e| MemoryError::VectorDb(format!("Failed to build record batch: {e}")))
        }
    }

    #[async_trait]
    impl VectorBackend for LanceDbVectorBackend {
        fn name(&self) -> &'static str {
            "lancedb"
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        async fn upsert(&self, id: &str, vector: &[f32], payload: Option<Value>) -> Result<()> {
            let table = self.get_or_init_table().await?;

            // Best-effort delete first to avoid duplicates without requiring a primary key.
            let _ = self.delete(id).await;

            let batch = self.batch_for_upsert(id, vector, payload)?;
            let schema = batch.schema();
            let batches = Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));

            table
                .add(batches)
                .execute()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB upsert failed: {e}")))?;

            Ok(())
        }

        async fn delete(&self, id: &str) -> Result<()> {
            let table = self.get_or_init_table().await?;
            let escaped = id.replace('\'', "''");
            table
                .delete(&format!("id = '{escaped}'"))
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB delete failed: {e}")))?;
            Ok(())
        }

        async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<VectorSearchHit>> {
            if vector.len() != self.dimension {
                return Err(MemoryError::VectorDb(format!(
                    "Vector dimension mismatch: got {}, expected {}",
                    vector.len(),
                    self.dimension
                )));
            }

            let table = self.get_or_init_table().await?;
            self.ensure_ann_index(&table).await?;

            let mut query = table
                .vector_search(vector)
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB vector_search failed: {e}")))?;

            if self.ann_cfg.enabled {
                query = query
                    .distance_type(self.ann_cfg.distance_type)
                    .nprobes(self.ann_cfg.nprobes);
                if let Some(refine) = self.ann_cfg.refine_factor {
                    query = query.refine_factor(refine);
                }
            }

            let mut stream = query
                .limit(limit)
                .execute()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB search failed: {e}")))?;

            let mut hits = Vec::new();
            while let Some(batch) = stream.next().await {
                let batch = batch
                    .map_err(|e| MemoryError::VectorDb(format!("LanceDB stream error: {e}")))?;
                if batch.num_rows() == 0 {
                    continue;
                }

                // Columns: id, vector, payload plus LanceDB-added distance/_score fields.
                // We only require `id`. If distance is present, convert to similarity-ish score.
                let id_col = batch
                    .column_by_name("id")
                    .ok_or_else(|| {
                        MemoryError::VectorDb("LanceDB results missing 'id' column".into())
                    })?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| {
                        MemoryError::VectorDb("LanceDB 'id' column type mismatch".into())
                    })?;

                let payload_col = match batch.column_by_name("payload") {
                    Some(c) => c.as_any().downcast_ref::<StringArray>(),
                    None => None,
                };

                let distance_f32 = match batch.column_by_name("_distance") {
                    Some(c) => c.as_any().downcast_ref::<Float32Array>(),
                    None => None,
                };
                let distance_f64 = match batch.column_by_name("_distance") {
                    Some(c) => c.as_any().downcast_ref::<Float64Array>(),
                    None => None,
                };

                for row in 0..batch.num_rows() {
                    let id = id_col.value(row).to_string();

                    let payload = payload_col.and_then(|p| {
                        if p.is_null(row) {
                            return None;
                        }
                        let s = p.value(row);
                        if s.is_empty() {
                            None
                        } else {
                            serde_json::from_str::<Value>(s).ok()
                        }
                    });

                    let score = if let Some(dist) = distance_f32 {
                        let d = dist.value(row).max(0.0);
                        1.0 / (1.0 + d)
                    } else if let Some(dist) = distance_f64 {
                        let d = dist.value(row).max(0.0) as f32;
                        1.0 / (1.0 + d)
                    } else {
                        1.0
                    };

                    hits.push(VectorSearchHit { id, score, payload });
                }
            }

            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            hits.truncate(limit);
            Ok(hits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn file_backend_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let backend = FileVectorBackend::new(dir.path().join("vectors"), 8);
        backend.ensure_ready().await.expect("init");

        backend
            .upsert("m1", &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], None)
            .await
            .expect("upsert");
        backend
            .upsert("m2", &[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], None)
            .await
            .expect("upsert");

        let hits = backend
            .search(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 2)
            .await
            .expect("search");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "m1");

        backend.delete("m1").await.expect("delete");
        let hits = backend
            .search(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 2)
            .await
            .expect("search after delete");
        assert!(hits.iter().all(|h| h.id != "m1"));
    }

    #[cfg(feature = "lancedb")]
    #[tokio::test]
    async fn lancedb_backend_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let backend = crate::vector_backend::lancedb::LanceDbVectorBackend::new(dir.path(), 8);
        backend.ensure_ready().await.expect("lancedb init");

        backend
            .upsert("m1", &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], None)
            .await
            .expect("lancedb upsert");
        backend
            .upsert("m2", &[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], None)
            .await
            .expect("lancedb upsert");

        let hits = backend
            .search(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 2)
            .await
            .expect("lancedb search");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "m1");
    }
}
