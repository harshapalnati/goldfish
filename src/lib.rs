//! # Goldfish - Agentic Memory Cortex for AI Agents

pub mod cache;
pub mod confidence;
pub mod cortex;
pub mod embedding;
pub mod error;
pub mod eval_harness;
pub mod hybrid_retrieval;
pub mod maintenance;
pub mod pulses;
pub mod search;
pub mod semantic_eval;
pub mod storage_backend;
pub mod store;
pub mod synthesis;
pub mod temporal;
pub mod types;
pub mod vector_backend;
pub mod vector_search;
pub mod versioning;

pub use cache::{
    CacheConfig, CacheConfigBuilder, CacheKey, CacheManager, CacheStats, CachedMemoryOperations,
    L1Cache,
};
pub use confidence::{
    ConfidenceConfig, ConfidenceFactors, ConfidenceTier, MemoryConfidence, SourceReliability,
    VerificationStatus,
};
pub use cortex::{
    ContextWindow, Experience, ImportanceCalculator, ImportanceWeights, MemoryCortex,
    MemorySummary, WorkingMemory, WorkingMemoryItem,
};
pub use embedding::{CorpusStats, EmbeddingProvider, HashEmbeddingProvider};
pub use error::{MemoryError, Result};
pub use eval_harness::{
    compare_configurations, create_test_dataset, print_results, run_comprehensive_benchmark,
    BenchmarkResults, QueryResult, RetrievalTestCase,
};
pub use hybrid_retrieval::{ExplainedSearchResult, HybridSearchConfig, RetrievalExplanation};
pub use maintenance::{
    run_maintenance, MaintenanceConfig, MaintenanceConfigBuilder, MaintenanceReport,
};
pub use pulses::{
    pulse, ChangeType, GoldfishPulses, Pulse, PulseConfig, PulseFilter, PulseStats, PulseType,
};
pub use search::{MemorySearch, SearchConfig, SearchMode, SearchSort};
pub use storage_backend::StorageBackend;
pub use store::{MemoryStore, SortOrder};
pub use synthesis::{Insight, InsightType, SynthesisConfig, SynthesisEngine};
pub use temporal::{
    Episode, TemporalConfig, TemporalMode, TemporalPreset, TemporalQuery, TemporalSearchResult,
};
pub use types::{
    Association, CreateAssociationInput, CreateMemoryInput, Memory, MemoryId, MemorySearchResult,
    MemoryType, RelationType, SessionId,
};
pub use vector_backend::{VectorBackend, VectorSearchHit};
pub use vector_search::{generate_embedding, VectorIndex, VectorSearchConfig};
pub use versioning::{
    ChangeType as VersionChangeType, ConflictResolution, FieldChange, FieldChangeKind,
    MemoryBranch, MemoryDiff, MemoryVersion, StorageMode, VersionAuthor, VersionConflict,
    VersionId, VersionRepository, VersioningConfig, VersioningConfigBuilder, VersioningEngine,
    VersioningStats,
};

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;

/// Main memory system - SQLite only for simplicity
#[derive(Clone)]
pub struct MemorySystem {
    store: Arc<MemoryStore>,
    search: MemorySearch,
    data_dir: std::path::PathBuf,
    pulses: Arc<GoldfishPulses>,
    vector: Option<Arc<dyn VectorBackend>>,
    embedder: Option<Arc<dyn EmbeddingProvider>>,
}

impl std::fmt::Debug for MemorySystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemorySystem")
            .field("data_dir", &self.data_dir)
            .finish()
    }
}

impl MemorySystem {
    /// Create a new memory system (SQLite only)
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let sqlite_path = data_dir.join("memories.db");
        let options = SqliteConnectOptions::new()
            .filename(&sqlite_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| MemoryError::Database(e.into()))?;

        let store = MemoryStore::new(pool);
        let search = MemorySearch::with_dir(Arc::clone(&store), &data_dir)?;
        search.reindex_all().await?;
        let pulses = Arc::new(GoldfishPulses::default());

        Ok(Self {
            store,
            search,
            data_dir,
            pulses,
            vector: None,
            embedder: None,
        })
    }

    /// Save a memory
    pub async fn save(&self, memory: &Memory) -> Result<()> {
        self.store.save(memory).await?;
        self.search.index_memory(memory)?;

        if let (Some(vector), Some(embedder)) = (&self.vector, &self.embedder) {
            let vectors = embedder
                .embed(std::slice::from_ref(&memory.content))
                .await
                .map_err(|e| MemoryError::VectorDb(format!("Embedding failed: {e}")))?;
            if let Some(v) = vectors.first() {
                vector.upsert(&memory.id, v, None).await?;
            }
        }

        Ok(())
    }

    /// Load a memory by ID
    pub async fn load(&self, id: &str) -> Result<Option<Memory>> {
        self.store.load(id).await
    }

    /// Update a memory
    pub async fn update(&self, memory: &Memory) -> Result<()> {
        self.store.update(memory).await?;
        self.search.index_memory(memory)?;

        if let (Some(vector), Some(embedder)) = (&self.vector, &self.embedder) {
            let vectors = embedder
                .embed(std::slice::from_ref(&memory.content))
                .await
                .map_err(|e| MemoryError::VectorDb(format!("Embedding failed: {e}")))?;
            if let Some(v) = vectors.first() {
                vector.upsert(&memory.id, v, None).await?;
            }
        }

        Ok(())
    }

    /// Delete a memory
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.store.delete(id).await?;
        self.search.delete_memory(id)?;

        if let Some(vector) = &self.vector {
            vector.delete(id).await?;
        }

        Ok(())
    }

    /// Soft delete (forget) a memory
    pub async fn forget(&self, id: &str) -> Result<bool> {
        self.store.forget(id).await
    }

    /// Restore a forgotten memory
    pub async fn restore(&self, id: &str) -> Result<bool> {
        self.store.restore(id).await
    }

    /// Search memories (simple text match for now)
    pub async fn search(&self, query: &str) -> Result<Vec<MemorySearchResult>> {
        self.search.search(query, &SearchConfig::default()).await
    }

    /// Search with custom configuration
    pub async fn search_with_config(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<MemorySearchResult>> {
        self.search.search(query, config).await
    }

    /// Get memories by type
    pub async fn get_by_type(&self, memory_type: MemoryType, limit: i64) -> Result<Vec<Memory>> {
        self.store.get_by_type(memory_type, limit).await
    }

    /// Get high-importance memories
    pub async fn get_high_importance(&self, threshold: f32, limit: i64) -> Result<Vec<Memory>> {
        self.store.get_high_importance(threshold, limit).await
    }

    /// Create an association between memories
    pub async fn associate(
        &self,
        source_id: &str,
        target_id: &str,
        relation_type: RelationType,
    ) -> Result<()> {
        let association = Association::new(source_id, target_id, relation_type);
        self.store.create_association(&association).await
    }

    /// Get associations for a memory
    pub async fn get_associations(&self, memory_id: &str) -> Result<Vec<Association>> {
        self.store.get_associations(memory_id).await
    }

    /// Get memory neighbors in the graph
    pub async fn get_neighbors(
        &self,
        memory_id: &str,
        depth: u32,
    ) -> Result<(Vec<Memory>, Vec<Association>)> {
        self.store.get_neighbors(memory_id, depth, &[]).await
    }

    /// Run maintenance tasks
    pub async fn run_maintenance(&self, config: &MaintenanceConfig) -> Result<MaintenanceReport> {
        maintenance::run_maintenance(&self.store, config).await
    }

    /// Get the underlying store
    pub fn store(&self) -> &MemoryStore {
        &self.store
    }

    /// Get the search interface
    pub fn search_interface(&self) -> &MemorySearch {
        &self.search
    }

    /// Get the pulses system for subscribing to events
    pub fn pulses(&self) -> &GoldfishPulses {
        &self.pulses
    }

    /// Attach a vector backend and embedding provider to enable hybrid retrieval.
    ///
    /// This does not change the existing API surface; it only enables the additional
    /// `hybrid_search` method and keeps vectors up-to-date on save/update/delete.
    pub fn with_vector_backend(
        mut self,
        vector: Arc<dyn VectorBackend>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.vector = Some(vector);
        self.embedder = Some(embedder);
        self
    }

    /// Hybrid retrieval: BM25 (Tantivy) + vector + recency + importance + graph neighborhood.
    pub async fn hybrid_search(
        &self,
        query: &str,
        cfg: &HybridSearchConfig,
        filter_type: Option<MemoryType>,
    ) -> Result<Vec<ExplainedSearchResult>> {
        let bm25_cfg = SearchConfig {
            mode: SearchMode::FullText,
            max_results: cfg.bm25_limit.max(cfg.max_results),
            memory_type: filter_type,
            ..SearchConfig::default()
        };

        let bm25 = self.search.search(query, &bm25_cfg).await?;

        hybrid_retrieval::hybrid_rank(
            query,
            bm25,
            self.vector.as_ref(),
            self.embedder.as_ref(),
            |id| {
                let store = Arc::clone(&self.store);
                let id = id.to_string();
                Box::pin(async move { store.load(&id).await })
            },
            |id, depth| {
                let store = Arc::clone(&self.store);
                let id = id.to_string();
                Box::pin(async move { store.get_neighbors(&id, depth, &[]).await })
            },
            cfg,
            filter_type,
        )
        .await
    }

    /// Search memories by time range
    pub async fn search_temporal(
        &self,
        _query: &str,
        temporal: &temporal::TemporalQuery,
    ) -> Result<Vec<MemorySearchResult>> {
        let time_filter = temporal.to_sql_filter();
        let memories = self.store.query_with_filter(&time_filter, 1000).await?;

        let results: Vec<MemorySearchResult> = memories
            .into_iter()
            .enumerate()
            .map(|(i, memory)| MemorySearchResult {
                memory,
                score: 1.0 - (i as f32 / 100.0),
                rank: i + 1,
            })
            .collect();

        Ok(results)
    }

    /// Get memories from today
    pub async fn get_today(&self) -> Result<Vec<Memory>> {
        let today = chrono::Utc::now().date_naive();
        let filter = format!("date(created_at) = '{}'", today);
        self.store.query_with_filter(&filter, 100).await
    }

    /// Get memories from yesterday
    pub async fn get_yesterday(&self) -> Result<Vec<Memory>> {
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1)).date_naive();
        let filter = format!("date(created_at) = '{}'", yesterday);
        self.store.query_with_filter(&filter, 100).await
    }

    /// Get memories from last N days
    pub async fn get_last_days(&self, n: i64) -> Result<Vec<Memory>> {
        let days_ago = (chrono::Utc::now() - chrono::Duration::days(n)).date_naive();
        let filter = format!("date(created_at) >= '{}'", days_ago);
        self.store.query_with_filter(&filter, 1000).await
    }
}
