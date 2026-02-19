//! Storage backend trait definitions

use crate::error::Result;
use crate::types::{Association, Memory, MemoryId, MemoryType, RelationType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Query parameters for storage search
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub text_query: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub min_importance: Option<f32>,
    pub max_results: usize,
    pub offset: usize,
}

/// Temporal filter for time-based queries
#[derive(Debug, Clone)]
pub struct TemporalFilter {
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub limit: usize,
}

/// Storage backend trait - abstracts over different storage implementations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Save a memory
    async fn save(&self, memory: &Memory) -> Result<()>;
    
    /// Load a memory by ID
    async fn load(&self, id: &str) -> Result<Option<Memory>>;
    
    /// Update an existing memory
    async fn update(&self, memory: &Memory) -> Result<()>;
    
    /// Delete a memory permanently
    async fn delete(&self, id: &str) -> Result<bool>;
    
    /// Soft delete (forget) a memory
    async fn forget(&self, id: &str) -> Result<bool>;
    
    /// Restore a forgotten memory
    async fn restore(&self, id: &str) -> Result<bool>;
    
    /// Search memories
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Memory>>;
    
    /// Get memories by type
    async fn get_by_type(&self, memory_type: MemoryType, limit: i64) -> Result<Vec<Memory>>;
    
    /// Get high importance memories
    async fn get_high_importance(&self, threshold: f32, limit: i64) -> Result<Vec<Memory>>;
    
    /// Query with custom SQL filter (SQLite-specific, may not work on all backends)
    async fn query_with_filter(&self, filter: &str, limit: i64) -> Result<Vec<Memory>>;
    
    /// Create an association between memories
    async fn create_association(&self, association: &Association) -> Result<()>;
    
    /// Get associations for a memory
    async fn get_associations(&self, memory_id: &str) -> Result<Vec<Association>>;
    
    /// Get neighboring memories in the graph
    async fn get_neighbors(
        &self,
        memory_id: &str,
        depth: u32,
        visited: &[String],
    ) -> Result<(Vec<Memory>, Vec<Association>)>;
    
    /// Temporal query - get memories within time range
    async fn temporal_query(&self, filter: &TemporalFilter) -> Result<Vec<Memory>>;
    
    /// Get storage statistics
    async fn stats(&self) -> Result<StorageStats>;
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_memories: usize,
    pub active_memories: usize,
    pub forgotten_memories: usize,
    pub total_associations: usize,
    pub by_type: Vec<(MemoryType, usize)>,
}
