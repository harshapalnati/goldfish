use crate::error::Result;
use async_trait::async_trait;
use serde_json::Value;

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

#[cfg(feature = "lancedb")]
pub mod lancedb {
    use super::*;
    use crate::error::MemoryError;
    use arrow_array::{
        types::Float32Type, Array, ArrayRef, FixedSizeListArray, Float32Array, Float64Array,
        RecordBatch, RecordBatchIterator, StringArray,
    };
    use arrow_schema::{DataType, Field, Schema};
    use ::lancedb::connect;
    use ::lancedb::query::{ExecutableQuery, QueryBase};
    use ::lancedb::table::Table;
    use futures::StreamExt;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug)]
    pub struct LanceDbVectorBackend {
        path: PathBuf,
        dimension: usize,
        table: RwLock<Option<Table>>,
    }

    impl LanceDbVectorBackend {
        pub fn new(path: impl AsRef<Path>, dimension: usize) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                dimension,
                table: RwLock::new(None),
            }
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

            table.add(batches)
                .execute()
                .await
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB upsert failed: {e}")))?;

            Ok(())
        }

        async fn delete(&self, id: &str) -> Result<()> {
            let table = self.get_or_init_table().await?;
            table.delete(&format!("id = '{id}'"))
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

            let mut stream = table
                .vector_search(vector)
                .map_err(|e| MemoryError::VectorDb(format!("LanceDB vector_search failed: {e}")))?
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

            hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            hits.truncate(limit);
            Ok(hits)
        }
    }
}
