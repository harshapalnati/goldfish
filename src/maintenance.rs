//! Memory maintenance: decay, prune, merge

use crate::error::Result;
use crate::types::MemoryType;
use crate::MemoryStore;

use std::sync::Arc;

/// Maintenance configuration
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    /// Importance below which memories are pruned
    pub prune_threshold: f32,
    /// Decay rate per day (0.0 - 1.0)
    pub decay_rate: f32,
    /// Minimum age in days before pruning
    pub min_age_days: i64,
    /// Similarity threshold for merging
    pub merge_similarity_threshold: f32,
    /// Whether to apply decay
    pub enable_decay: bool,
    /// Whether to prune memories
    pub enable_pruning: bool,
    /// Whether to merge similar memories
    pub enable_merging: bool,
    /// Whether to consolidate old memories into summaries
    pub enable_consolidation: bool,
    /// Minimum age in days before consolidation
    pub consolidation_age_days: i64,
    /// Importance threshold for consolidation
    pub consolidation_threshold: f32,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            prune_threshold: 0.1,
            decay_rate: 0.05,
            min_age_days: 30,
            merge_similarity_threshold: 0.95,
            enable_decay: true,
            enable_pruning: true,
            enable_merging: false,       // Disabled by default (expensive)
            enable_consolidation: false, // Disabled by default
            consolidation_age_days: 30,
            consolidation_threshold: 0.3,
        }
    }
}

/// Maintenance report
#[derive(Debug, Default)]
pub struct MaintenanceReport {
    /// Number of memories decayed
    pub decayed: usize,
    /// Number of memories pruned
    pub pruned: usize,
    /// Number of memories merged
    pub merged: usize,
    /// Number of memories consolidated into summaries
    pub consolidated: usize,
    /// Total memories checked
    pub checked: usize,
}

/// Run maintenance tasks
pub async fn run_maintenance(
    memory_store: &Arc<MemoryStore>,
    config: &MaintenanceConfig,
) -> Result<MaintenanceReport> {
    let mut report = MaintenanceReport::default();

    if config.enable_decay {
        report.decayed = apply_decay(memory_store, config.decay_rate).await?;
    }

    if config.enable_pruning {
        report.pruned = prune_memories(memory_store, config).await?;
    }

    if config.enable_merging {
        report.merged =
            merge_similar_memories(memory_store, config.merge_similarity_threshold).await?;
    }

    Ok(report)
}

/// Apply importance decay based on age and access patterns
async fn apply_decay(memory_store: &Arc<MemoryStore>, decay_rate: f32) -> Result<usize> {
    let mut decayed_count = 0;

    // Get all memories that can decay
    for mem_type in MemoryType::ALL.iter().filter(|t| t.can_decay()) {
        let memories = memory_store.get_by_type(*mem_type, 1000).await?;

        for mut memory in memories {
            let now = chrono::Utc::now();
            let days_old = (now - memory.updated_at).num_days();
            let days_since_access = (now - memory.last_accessed_at).num_days();

            // Calculate decay factors
            let age_decay = 1.0 - (days_old as f32 * decay_rate).min(0.5);
            let access_boost = if days_since_access < 7 {
                1.1 // Recent access boosts importance
            } else if days_since_access > 30 {
                0.9 // Long time since access reduces importance
            } else {
                1.0
            };

            let new_importance = memory.importance * age_decay * access_boost;

            // Only update if change is significant
            if (new_importance - memory.importance).abs() > 0.01 {
                memory.importance = new_importance.clamp(0.0, 1.0);
                memory.updated_at = now;
                memory_store.update(&memory).await?;
                decayed_count += 1;
            }
        }
    }

    tracing::debug!("Decayed {} memories", decayed_count);
    Ok(decayed_count)
}

/// Prune old, low-importance memories
async fn prune_memories(
    memory_store: &Arc<MemoryStore>,
    config: &MaintenanceConfig,
) -> Result<usize> {
    let candidates = memory_store
        .get_pruning_candidates(config.prune_threshold, config.min_age_days)
        .await?;

    let mut pruned_count = 0;

    for memory in candidates {
        // Soft delete (forget) rather than hard delete
        if memory_store.forget(&memory.id).await? {
            pruned_count += 1;
        }
    }

    tracing::debug!("Pruned (forgotten) {} memories", pruned_count);
    Ok(pruned_count)
}

/// Merge near-duplicate memories
async fn merge_similar_memories(
    _memory_store: &Arc<MemoryStore>,
    _similarity_threshold: f32,
) -> Result<usize> {
    // Placeholder for future implementation
    // Would need:
    // 1. Find pairs with high embedding similarity
    // 2. Keep the higher importance one
    // 3. Update associations to point to merged memory
    // 4. Delete the duplicate

    tracing::debug!("Memory merging not yet implemented");
    Ok(0)
}

/// Builder for maintenance config
pub struct MaintenanceConfigBuilder {
    config: MaintenanceConfig,
}

impl MaintenanceConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: MaintenanceConfig::default(),
        }
    }

    pub fn prune_threshold(mut self, threshold: f32) -> Self {
        self.config.prune_threshold = threshold;
        self
    }

    pub fn decay_rate(mut self, rate: f32) -> Self {
        self.config.decay_rate = rate;
        self
    }

    pub fn min_age_days(mut self, days: i64) -> Self {
        self.config.min_age_days = days;
        self
    }

    pub fn enable_decay(mut self, enable: bool) -> Self {
        self.config.enable_decay = enable;
        self
    }

    pub fn enable_pruning(mut self, enable: bool) -> Self {
        self.config.enable_pruning = enable;
        self
    }

    pub fn build(self) -> MaintenanceConfig {
        self.config
    }
}

impl Default for MaintenanceConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
