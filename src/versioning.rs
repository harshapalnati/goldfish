//! # Memory Versioning
//!
//! Track memory evolution over time with full version history and rollback support.
//!
//! ## Features
//!
//! - **Version History**: Every change to a memory creates a new version
//! - **Diff Tracking**: See exactly what changed between versions
//! - **Rollback Support**: Revert to any previous version
//! - **Branching**: Create divergent versions for speculative scenarios
//! - **Conflict Detection**: Identify when concurrent changes occur
//!
//! ## Example
//!
//! ```rust,ignore
//! use goldfish::{MemorySystem, versioning};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let memory = MemorySystem::new("./data").await?;
//!
//!     // Create a memory
//!     let mut fact = Memory::new("The meeting is at 2pm", MemoryType::Fact);
//!     memory.save(&fact).await?;
//!
//!     // Update it (creates new version)
//!     fact.content = "The meeting is at 3pm".to_string();
//!     memory.save(&fact).await?;
//!
//!     // See version history
//!     let history = memory.get_version_history(&fact.id).await?;
//!
//!     // Rollback to previous version
//!     memory.rollback_to_version(&fact.id, history[0].version_number).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::{
    error::{MemoryError, Result},
    types::{Memory, MemoryId},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a memory version
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionId(pub String);

impl VersionId {
    /// Generate a new unique version ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for VersionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A snapshot of a memory at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersion {
    /// Unique version identifier
    pub version_id: VersionId,
    /// Which memory this is a version of
    pub memory_id: MemoryId,
    /// Sequential version number (1, 2, 3, ...)
    pub version_number: u32,
    /// The memory data at this version
    pub memory: Memory,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Who/what created this version
    pub author: VersionAuthor,
    /// Why this change was made
    pub change_reason: Option<String>,
    /// Previous version (None if first version)
    pub previous_version_id: Option<VersionId>,
    /// Diff from previous version (computed lazily)
    pub diff: Option<MemoryDiff>,
}

/// Author of a version change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionAuthor {
    /// The AI agent itself
    Agent,
    /// A human user
    User { id: String, name: String },
    /// An external system
    System { name: String },
    /// An automated process
    Automation { process: String },
    /// Unknown/legacy
    Unknown,
}

impl std::fmt::Display for VersionAuthor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agent => write!(f, "Agent"),
            Self::User { name, .. } => write!(f, "User:{}", name),
            Self::System { name } => write!(f, "System:{}", name),
            Self::Automation { process } => write!(f, "Auto:{}", process),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Difference between two memory versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDiff {
    /// Fields that changed
    pub changes: Vec<FieldChange>,
    /// Overall change type
    pub change_type: ChangeType,
    /// Summary of changes
    pub summary: String,
}

/// Type of change between versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// New memory created
    Created,
    /// Content changed
    Modified,
    /// Priority/importance changed
    Prioritized,
    /// Emotional valence changed
    EmotionalChange,
    /// Metadata changed
    MetadataChange,
    /// Memory soft-deleted
    SoftDeleted,
    /// Memory restored
    Restored,
    /// Tags changed
    Tagged,
    /// Associations changed
    Related,
    /// Multiple types of changes
    Complex,
}

/// A change to a specific field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Which field changed
    pub field: String,
    /// Old value
    pub old_value: serde_json::Value,
    /// New value
    pub new_value: serde_json::Value,
    /// Type of change
    pub change_kind: FieldChangeKind,
}

/// Kind of field change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldChangeKind {
    Added,
    Removed,
    Modified,
}

/// Configuration for versioning behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningConfig {
    /// Maximum number of versions to keep per memory
    pub max_versions_per_memory: usize,
    /// Whether to store full memory or just diffs
    pub storage_mode: StorageMode,
    /// Whether to automatically prune old versions
    pub auto_prune: bool,
    /// Versions older than this are candidates for pruning (in days)
    pub prune_threshold_days: i64,
    /// Whether to enable branching
    pub enable_branching: bool,
    /// Whether to detect and track conflicts
    pub track_conflicts: bool,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            max_versions_per_memory: 50,
            storage_mode: StorageMode::FullSnapshot,
            auto_prune: true,
            prune_threshold_days: 30,
            enable_branching: false,
            track_conflicts: true,
        }
    }
}

/// How version data is stored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    /// Store complete memory snapshot for each version
    FullSnapshot,
    /// Store only diffs (requires reconstructing from base)
    Differential,
    /// Hybrid: recent versions as full snapshots, older as diffs
    Hybrid,
}

/// A branch of memory versions (for speculative scenarios)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBranch {
    /// Unique branch identifier
    pub branch_id: String,
    /// Human-readable name
    pub name: String,
    /// Description of this branch
    pub description: Option<String>,
    /// Which version this branch was created from
    pub parent_version_id: VersionId,
    /// Memory this branch belongs to
    pub memory_id: MemoryId,
    /// All versions on this branch
    pub version_ids: Vec<VersionId>,
    /// When branch was created
    pub created_at: DateTime<Utc>,
    /// Whether this is the main/primary branch
    pub is_main: bool,
}

/// Detected conflict between concurrent changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The conflicting versions
    pub versions: Vec<VersionId>,
    /// Which memory has the conflict
    pub memory_id: MemoryId,
    /// When the conflict was detected
    pub detected_at: DateTime<Utc>,
    /// Description of the conflict
    pub description: String,
    /// Whether the conflict was resolved
    pub resolved: bool,
    /// How it was resolved
    pub resolution: Option<ConflictResolution>,
}

/// How a conflict was resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Used version A
    AcceptedVersionA,
    /// Used version B
    AcceptedVersionB,
    /// Merged both versions
    Merged,
    /// Created new combined version
    NewVersion,
    /// Manual intervention required
    Manual,
}

/// Statistics about memory versioning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersioningStats {
    /// Total number of versions across all memories
    pub total_versions: u64,
    /// Number of memories with version history
    pub memories_with_history: u64,
    /// Average versions per memory
    pub avg_versions_per_memory: f64,
    /// Number of branches
    pub total_branches: u64,
    /// Number of unresolved conflicts
    pub unresolved_conflicts: u64,
    /// Total conflicts (resolved + unresolved)
    pub total_conflicts: u64,
    /// Storage used by versions (in bytes)
    pub storage_bytes: u64,
}

/// Repository for version storage
#[async_trait]
pub trait VersionRepository: Send + Sync {
    /// Save a new version
    async fn save_version(&self, version: &MemoryVersion) -> Result<()>;
    
    /// Get a specific version
    async fn get_version(&self, version_id: &VersionId) -> Result<Option<MemoryVersion>>;
    
    /// Get all versions for a memory
    async fn get_memory_versions(&self, memory_id: &MemoryId) -> Result<Vec<MemoryVersion>>;
    
    /// Get the latest version for a memory
    async fn get_latest_version(&self, memory_id: &MemoryId) -> Result<Option<MemoryVersion>>;
    
    /// Delete old versions
    async fn prune_versions(&self, memory_id: &MemoryId, keep_count: usize) -> Result<u64>;
    
    /// Create a branch
    async fn create_branch(&self, branch: &MemoryBranch) -> Result<()>;
    
    /// Get branches for a memory
    async fn get_branches(&self, memory_id: &MemoryId) -> Result<Vec<MemoryBranch>>;
    
    /// Record a conflict
    async fn record_conflict(&self, conflict: &VersionConflict) -> Result<()>;
    
    /// Get unresolved conflicts
    async fn get_unresolved_conflicts(&self) -> Result<Vec<VersionConflict>>;
}

/// Main versioning engine
pub struct VersioningEngine {
    repository: Box<dyn VersionRepository>,
    config: VersioningConfig,
}

impl VersioningEngine {
    /// Create a new versioning engine
    pub fn new(repository: Box<dyn VersionRepository>, config: VersioningConfig) -> Self {
        Self { repository, config }
    }

    /// Record a new version of a memory
    pub async fn record_version(
        &self,
        memory: &Memory,
        author: VersionAuthor,
        reason: Option<&str>,
    ) -> Result<MemoryVersion> {
        // Get latest version number
        let versions = self.repository.get_memory_versions(&memory.id).await?;
        let version_number = versions.len() as u32 + 1;
        let previous_version_id = versions.last().map(|v| v.version_id.clone());

        // Calculate diff if we have a previous version
        let diff = if let Some(ref prev_id) = previous_version_id {
            self.calculate_diff(prev_id, memory).await.ok()
        } else {
            None
        };

        let version = MemoryVersion {
            version_id: VersionId::new(),
            memory_id: memory.id.clone(),
            version_number,
            memory: memory.clone(),
            created_at: Utc::now(),
            author,
            change_reason: reason.map(String::from),
            previous_version_id,
            diff,
        };

        self.repository.save_version(&version).await?;

        // Auto-prune if enabled and threshold exceeded
        if self.config.auto_prune && versions.len() >= self.config.max_versions_per_memory {
            let to_prune = versions.len() - self.config.max_versions_per_memory + 1;
            self.repository.prune_versions(&memory.id, self.config.max_versions_per_memory - to_prune).await?;
        }

        Ok(version)
    }

    /// Get version history for a memory
    pub async fn get_history(&self, memory_id: &MemoryId) -> Result<Vec<MemoryVersion>> {
        let mut versions = self.repository.get_memory_versions(memory_id).await?;
        versions.sort_by(|a, b| a.version_number.cmp(&b.version_number));
        Ok(versions)
    }

    /// Rollback a memory to a specific version
    pub async fn rollback(&self, memory_id: &MemoryId, version_number: u32) -> Result<Memory> {
        let versions = self.get_history(memory_id).await?;
        
        let target = versions
            .iter()
            .find(|v| v.version_number == version_number)
            .ok_or_else(|| MemoryError::NotFound(format!("Version {} not found", version_number)))?;

        // Create rollback version
        let rollback_version = MemoryVersion {
            version_id: VersionId::new(),
            memory_id: memory_id.clone(),
            version_number: versions.len() as u32 + 1,
            memory: target.memory.clone(),
            created_at: Utc::now(),
            author: VersionAuthor::System { name: "rollback".to_string() },
            change_reason: Some(format!("Rollback to version {}", version_number)),
            previous_version_id: versions.last().map(|v| v.version_id.clone()),
            diff: None,
        };

        self.repository.save_version(&rollback_version).await?;

        Ok(target.memory.clone())
    }

    /// Compare two versions
    pub async fn compare_versions(&self, version_a: &VersionId, version_b: &VersionId) -> Result<MemoryDiff> {
        let ver_a = self.repository.get_version(version_a).await?
            .ok_or_else(|| MemoryError::NotFound("Version A not found".to_string()))?;
        let ver_b = self.repository.get_version(version_b).await?
            .ok_or_else(|| MemoryError::NotFound("Version B not found".to_string()))?;

        Ok(self.diff_memories(&ver_a.memory, &ver_b.memory))
    }

    /// Create a new branch from a version
    pub async fn create_branch(
        &self,
        version_id: &VersionId,
        name: impl Into<String>,
        description: Option<&str>,
    ) -> Result<MemoryBranch> {
        if !self.config.enable_branching {
            return Err(MemoryError::Configuration("Branching is disabled".to_string()));
        }

        let version = self.repository.get_version(version_id).await?
            .ok_or_else(|| MemoryError::NotFound("Version not found".to_string()))?;

        let branch = MemoryBranch {
            branch_id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.map(String::from),
            parent_version_id: version_id.clone(),
            memory_id: version.memory_id.clone(),
            version_ids: vec![version_id.clone()],
            created_at: Utc::now(),
            is_main: false,
        };

        self.repository.create_branch(&branch).await?;
        Ok(branch)
    }

    /// Detect conflicts between concurrent modifications
    pub async fn detect_conflicts(
        &self,
        memory_id: &MemoryId,
        versions: &[VersionId],
    ) -> Result<Vec<VersionConflict>> {
        if !self.config.track_conflicts || versions.len() < 2 {
            return Ok(vec![]);
        }

        // Get all versions
        let mut mem_versions = vec![];
        for v_id in versions {
            if let Some(v) = self.repository.get_version(v_id).await? {
                mem_versions.push(v);
            }
        }

        // Sort by creation time
        mem_versions.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let mut conflicts = vec![];
        for window in mem_versions.windows(2) {
            let (older, newer) = (&window[0], &window[1]);
            
            // Check if changes overlap
            let diff = self.diff_memories(&older.memory, &newer.memory);
            if !diff.changes.is_empty() {
                // Check for overlapping changes
                let conflict = VersionConflict {
                    versions: vec![older.version_id.clone(), newer.version_id.clone()],
                    memory_id: memory_id.clone(),
                    detected_at: Utc::now(),
                    description: format!(
                        "Concurrent changes detected between versions {} and {}",
                        older.version_number,
                        newer.version_number
                    ),
                    resolved: false,
                    resolution: None,
                };
                conflicts.push(conflict);
            }
        }

        // Record conflicts
        for conflict in &conflicts {
            self.repository.record_conflict(conflict).await?;
        }

        Ok(conflicts)
    }

    /// Get versioning statistics
    pub async fn get_stats(&self) -> Result<VersioningStats> {
        // This would be implemented by the repository
        // For now, return defaults
        Ok(VersioningStats::default())
    }

    /// Calculate diff between a previous version and current memory
    async fn calculate_diff(&self, previous_version_id: &VersionId, current: &Memory) -> Result<MemoryDiff> {
        let previous = self.repository.get_version(previous_version_id).await?
            .ok_or_else(|| MemoryError::NotFound("Previous version not found".to_string()))?;
        
        Ok(self.diff_memories(&previous.memory, current))
    }

    /// Calculate diff between two memories
    fn diff_memories(&self, old: &Memory, new: &Memory) -> MemoryDiff {
        let mut changes = vec![];

        // Compare content
        if old.content != new.content {
            changes.push(FieldChange {
                field: "content".to_string(),
                old_value: serde_json::Value::String(old.content.clone()),
                new_value: serde_json::Value::String(new.content.clone()),
                change_kind: FieldChangeKind::Modified,
            });
        }

        // Compare memory type
        if old.memory_type != new.memory_type {
            changes.push(FieldChange {
                field: "memory_type".to_string(),
                old_value: serde_json::to_value(old.memory_type).unwrap_or_default(),
                new_value: serde_json::to_value(new.memory_type).unwrap_or_default(),
                change_kind: FieldChangeKind::Modified,
            });
        }

        // Compare priority
        if old.priority != new.priority {
            changes.push(FieldChange {
                field: "priority".to_string(),
                old_value: serde_json::to_value(old.priority).unwrap_or_default(),
                new_value: serde_json::to_value(new.priority).unwrap_or_default(),
                change_kind: FieldChangeKind::Modified,
            });
        }

        // Compare emotional valence
        if old.emotional_valence != new.emotional_valence {
            changes.push(FieldChange {
                field: "emotional_valence".to_string(),
                old_value: serde_json::to_value(old.emotional_valence).unwrap_or_default(),
                new_value: serde_json::to_value(new.emotional_valence).unwrap_or_default(),
                change_kind: FieldChangeKind::Modified,
            });
        }

        // Compare tags
        let old_tags: std::collections::HashSet<_> = old.tags.iter().collect();
        let new_tags: std::collections::HashSet<_> = new.tags.iter().collect();
        
        let added_tags: Vec<_> = new_tags.difference(&old_tags).cloned().collect();
        let removed_tags: Vec<_> = old_tags.difference(&new_tags).cloned().collect();

        if !added_tags.is_empty() {
            changes.push(FieldChange {
                field: "tags".to_string(),
                old_value: serde_json::Value::Array(vec![]),
                new_value: serde_json::to_value(added_tags).unwrap_or_default(),
                change_kind: FieldChangeKind::Added,
            });
        }

        if !removed_tags.is_empty() {
            changes.push(FieldChange {
                field: "tags".to_string(),
                old_value: serde_json::to_value(removed_tags).unwrap_or_default(),
                new_value: serde_json::Value::Array(vec![]),
                change_kind: FieldChangeKind::Removed,
            });
        }

        // Compare metadata
        let old_meta = old.metadata.as_ref().map(|m| {
            if let Some(obj) = m.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                std::collections::HashMap::new()
            }
        }).unwrap_or_default();
        
        let new_meta = new.metadata.as_ref().map(|m| {
            if let Some(obj) = m.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                std::collections::HashMap::new()
            }
        }).unwrap_or_default();

        for (key, new_val) in &new_meta {
            if let Some(old_val) = old_meta.get(key) {
                if old_val != new_val {
                    changes.push(FieldChange {
                        field: format!("metadata.{}", key),
                        old_value: old_val.clone(),
                        new_value: new_val.clone(),
                        change_kind: FieldChangeKind::Modified,
                    });
                }
            } else {
                changes.push(FieldChange {
                    field: format!("metadata.{}", key),
                    old_value: serde_json::Value::Null,
                    new_value: new_val.clone(),
                    change_kind: FieldChangeKind::Added,
                });
            }
        }

        for (key, old_val) in &old_meta {
            if !new_meta.contains_key(key) {
                changes.push(FieldChange {
                    field: format!("metadata.{}", key),
                    old_value: old_val.clone(),
                    new_value: serde_json::Value::Null,
                    change_kind: FieldChangeKind::Removed,
                });
            }
        }

        // Determine change type
        let change_type = if changes.is_empty() {
            ChangeType::Created
        } else if changes.len() == 1 && changes[0].field == "content" {
            ChangeType::Modified
        } else if changes.iter().any(|c| c.field == "priority") {
            ChangeType::Prioritized
        } else if changes.iter().any(|c| c.field == "emotional_valence") {
            ChangeType::EmotionalChange
        } else if changes.iter().any(|c| c.field.starts_with("metadata")) {
            ChangeType::MetadataChange
        } else if changes.iter().any(|c| c.field == "tags") {
            ChangeType::Tagged
        } else {
            ChangeType::Complex
        };

        let summary = format!(
            "{} field(s) changed: {}",
            changes.len(),
            changes.iter().map(|c| c.field.as_str()).collect::<Vec<_>>().join(", ")
        );

        MemoryDiff {
            changes,
            change_type,
            summary,
        }
    }
}

/// Configuration builder
#[derive(Debug, Default)]
pub struct VersioningConfigBuilder {
    config: VersioningConfig,
}

impl VersioningConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max versions per memory
    pub fn max_versions(mut self, count: usize) -> Self {
        self.config.max_versions_per_memory = count;
        self
    }

    /// Set storage mode
    pub fn storage_mode(mut self, mode: StorageMode) -> Self {
        self.config.storage_mode = mode;
        self
    }

    /// Enable auto-pruning
    pub fn auto_prune(mut self, enable: bool) -> Self {
        self.config.auto_prune = enable;
        self
    }

    /// Set prune threshold
    pub fn prune_threshold(mut self, days: i64) -> Self {
        self.config.prune_threshold_days = days;
        self
    }

    /// Enable branching
    pub fn enable_branching(mut self, enable: bool) -> Self {
        self.config.enable_branching = enable;
        self
    }

    /// Enable conflict tracking
    pub fn track_conflicts(mut self, enable: bool) -> Self {
        self.config.track_conflicts = enable;
        self
    }

    /// Build configuration
    pub fn build(self) -> VersioningConfig {
        self.config
    }
}

/// Utility functions for versioning
pub mod utils {
    use super::*;

    /// Get a human-readable description of a version
    pub fn describe_version(version: &MemoryVersion) -> String {
        let time_ago = chrono::Utc::now() - version.created_at;
        let time_str = if time_ago.num_hours() < 1 {
            format!("{} minutes ago", time_ago.num_minutes())
        } else if time_ago.num_hours() < 24 {
            format!("{} hours ago", time_ago.num_hours())
        } else {
            format!("{} days ago", time_ago.num_days())
        };

        format!(
            "Version {} by {} ({}) - {}",
            version.version_number,
            version.author,
            time_str,
            version.change_reason.as_deref().unwrap_or("No description")
        )
    }

    /// Format a diff for display
    pub fn format_diff(diff: &MemoryDiff) -> String {
        let mut output = format!("Changes ({}):\n", diff.change_type);
        
        for change in &diff.changes {
            let prefix = match change.change_kind {
                FieldChangeKind::Added => "+",
                FieldChangeKind::Removed => "-",
                FieldChangeKind::Modified => "~",
            };
            
            output.push_str(&format!(
                "  {} {}: {:?} → {:?}\n",
                prefix,
                change.field,
                change.old_value,
                change.new_value
            ));
        }
        
        output
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Modified => write!(f, "Modified"),
            Self::Prioritized => write!(f, "Prioritized"),
            Self::EmotionalChange => write!(f, "EmotionalChange"),
            Self::MetadataChange => write!(f, "MetadataChange"),
            Self::SoftDeleted => write!(f, "SoftDeleted"),
            Self::Restored => write!(f, "Restored"),
            Self::Tagged => write!(f, "Tagged"),
            Self::Related => write!(f, "Related"),
            Self::Complex => write!(f, "Complex"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryType;

    fn create_test_memory(content: &str) -> Memory {
        Memory::new(content, MemoryType::Fact)
    }

    #[test]
    fn test_version_id_generation() {
        let id1 = VersionId::new();
        let id2 = VersionId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_diff_content_change() {
        let old = create_test_memory("Original content");
        let mut new = old.clone();
        new.content = "Updated content".to_string();

        let config = VersioningConfig::default();
        let engine = VersioningEngine::new(Box::new(DummyRepository), config);
        let diff = engine.diff_memories(&old, &new);

        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].field, "content");
        assert_eq!(diff.change_type, ChangeType::Modified);
    }

    #[test]
    fn test_diff_tag_change() {
        let old = create_test_memory("Content");
        let mut new = old.clone();
        new.tags = vec!["tag1".to_string(), "tag2".to_string()];

        let config = VersioningConfig::default();
        let engine = VersioningEngine::new(Box::new(DummyRepository), config);
        let diff = engine.diff_memories(&old, &new);

        assert!(diff.changes.iter().any(|c| c.field == "tags"));
    }

    // Dummy repository for testing
    struct DummyRepository;
    
    #[async_trait::async_trait]
    impl VersionRepository for DummyRepository {
        async fn save_version(&self, _version: &MemoryVersion) -> Result<()> { Ok(()) }
        async fn get_version(&self, _version_id: &VersionId) -> Result<Option<MemoryVersion>> { Ok(None) }
        async fn get_memory_versions(&self, _memory_id: &MemoryId) -> Result<Vec<MemoryVersion>> { Ok(vec![]) }
        async fn get_latest_version(&self, _memory_id: &MemoryId) -> Result<Option<MemoryVersion>> { Ok(None) }
        async fn prune_versions(&self, _memory_id: &MemoryId, _keep_count: usize) -> Result<u64> { Ok(0) }
        async fn create_branch(&self, _branch: &MemoryBranch) -> Result<()> { Ok(()) }
        async fn get_branches(&self, _memory_id: &MemoryId) -> Result<Vec<MemoryBranch>> { Ok(vec![]) }
        async fn record_conflict(&self, _conflict: &VersionConflict) -> Result<()> { Ok(()) }
        async fn get_unresolved_conflicts(&self) -> Result<Vec<VersionConflict>> { Ok(vec![]) }
    }
}
