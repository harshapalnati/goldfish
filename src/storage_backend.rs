use crate::cortex::{Experience, MemorySummary};
use crate::error::Result;
use crate::store::MemoryStore;
use crate::temporal::TemporalQuery;
use crate::types::{Association, Memory, MemoryType};
use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;

    async fn save_memory(&self, memory: &Memory) -> Result<()>;
    async fn load_memory(&self, id: &str) -> Result<Option<Memory>>;
    async fn update_memory(&self, memory: &Memory) -> Result<()>;
    async fn delete_memory(&self, id: &str) -> Result<()>;
    async fn forget_memory(&self, id: &str) -> Result<bool>;
    async fn restore_memory(&self, id: &str) -> Result<bool>;

    async fn get_by_type(&self, memory_type: MemoryType, limit: i64) -> Result<Vec<Memory>>;
    async fn query_temporal(&self, query: &TemporalQuery, limit: i64) -> Result<Vec<Memory>>;

    async fn create_association(&self, association: &Association) -> Result<()>;
    async fn get_associations(&self, memory_id: &str) -> Result<Vec<Association>>;
    async fn get_neighbors(
        &self,
        memory_id: &str,
        depth: u32,
        exclude_ids: &[String],
    ) -> Result<(Vec<Memory>, Vec<Association>)>;

    async fn save_experience(&self, experience: &Experience) -> Result<()>;
    async fn update_experience(&self, experience: &Experience) -> Result<()>;
    async fn list_experiences(&self, limit: i64, offset: i64) -> Result<Vec<Experience>>;
    async fn get_experience(&self, id: &str) -> Result<Option<Experience>>;
    async fn add_memory_to_experience(&self, experience_id: &str, memory_id: &str) -> Result<()>;

    async fn save_summary(&self, summary: &MemorySummary) -> Result<()>;
    async fn get_summaries(&self) -> Result<Vec<MemorySummary>>;
}

#[async_trait]
impl StorageBackend for MemoryStore {
    fn backend_name(&self) -> &'static str {
        "sqlite"
    }

    async fn save_memory(&self, memory: &Memory) -> Result<()> {
        self.save(memory).await
    }

    async fn load_memory(&self, id: &str) -> Result<Option<Memory>> {
        self.load(id).await
    }

    async fn update_memory(&self, memory: &Memory) -> Result<()> {
        self.update(memory).await
    }

    async fn delete_memory(&self, id: &str) -> Result<()> {
        self.delete(id).await
    }

    async fn forget_memory(&self, id: &str) -> Result<bool> {
        self.forget(id).await
    }

    async fn restore_memory(&self, id: &str) -> Result<bool> {
        self.restore(id).await
    }

    async fn get_by_type(&self, memory_type: MemoryType, limit: i64) -> Result<Vec<Memory>> {
        self.get_by_type(memory_type, limit).await
    }

    async fn query_temporal(&self, query: &TemporalQuery, limit: i64) -> Result<Vec<Memory>> {
        let filter = query.to_sql_filter();
        self.query_with_filter(&filter, limit).await
    }

    async fn create_association(&self, association: &Association) -> Result<()> {
        self.create_association(association).await
    }

    async fn get_associations(&self, memory_id: &str) -> Result<Vec<Association>> {
        self.get_associations(memory_id).await
    }

    async fn get_neighbors(
        &self,
        memory_id: &str,
        depth: u32,
        exclude_ids: &[String],
    ) -> Result<(Vec<Memory>, Vec<Association>)> {
        self.get_neighbors(memory_id, depth, exclude_ids).await
    }

    async fn save_experience(&self, experience: &Experience) -> Result<()> {
        self.save_experience(experience).await
    }

    async fn update_experience(&self, experience: &Experience) -> Result<()> {
        self.update_experience(experience).await
    }

    async fn list_experiences(&self, limit: i64, offset: i64) -> Result<Vec<Experience>> {
        self.list_experiences(limit, offset).await
    }

    async fn get_experience(&self, id: &str) -> Result<Option<Experience>> {
        self.get_experience(id).await
    }

    async fn add_memory_to_experience(&self, experience_id: &str, memory_id: &str) -> Result<()> {
        self.add_memory_to_experience(experience_id, memory_id)
            .await
    }

    async fn save_summary(&self, summary: &MemorySummary) -> Result<()> {
        self.save_summary(summary).await
    }

    async fn get_summaries(&self) -> Result<Vec<MemorySummary>> {
        self.get_summaries().await
    }
}
