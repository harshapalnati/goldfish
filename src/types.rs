//! Memory types and graph structures

use crate::confidence::{MemoryConfidence, SourceReliability};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for memories
pub type MemoryId = String;

/// Unique identifier for sessions/conversations
pub type SessionId = String;

/// Memory structure representing a piece of knowledge
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    /// Unique identifier
    pub id: MemoryId,
    /// The memory content
    pub content: String,
    /// Type of memory (affects importance and behavior)
    pub memory_type: MemoryType,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Priority score (0.0 - 1.0)
    pub priority: f32,
    /// Emotional valence (-1.0 to 1.0)
    pub emotional_valence: f32,
    /// Tags associated with this memory
    pub tags: Vec<String>,
    /// When the memory was created
    pub created_at: DateTime<Utc>,
    /// When the memory was last updated
    pub updated_at: DateTime<Utc>,
    /// When the memory was last accessed
    pub last_accessed_at: DateTime<Utc>,
    /// Number of times accessed
    pub access_count: i64,
    /// Source of the memory (e.g., "conversation", "import", "system")
    pub source: Option<String>,
    /// Session/channel this memory belongs to
    pub session_id: Option<SessionId>,
    /// Whether this memory is forgotten (soft delete)
    pub forgotten: bool,
    /// Additional metadata (flexible key-value storage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Confidence scoring for this memory
    ///
    /// Based on research in cognitive science and AI uncertainty quantification:
    /// - Koriat (2012): Self-Consistency Model
    /// - Cohen et al. (2021): Uncertainty in Retrieval Models
    /// - Shorinwa et al. (2024): LLM Uncertainty Quantification
    pub confidence: MemoryConfidence,
}

impl Memory {
    /// Create a new memory with default values
    pub fn new(content: impl Into<String>, memory_type: MemoryType) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let importance = memory_type.default_importance();

        Self {
            id,
            content: content.into(),
            memory_type,
            importance,
            priority: importance,
            emotional_valence: 0.0,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            last_accessed_at: now,
            access_count: 0,
            source: None,
            session_id: None,
            forgotten: false,
            metadata: None,
            confidence: MemoryConfidence::new(),
        }
    }

    /// Set custom importance
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: impl Into<SessionId>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this memory should never decay
    pub fn is_permanent(&self) -> bool {
        self.memory_type == MemoryType::Identity || self.importance >= 0.95
    }

    /// Set confidence from source reliability
    pub fn with_confidence(mut self, reliability: SourceReliability) -> Self {
        self.confidence = MemoryConfidence::with_source_reliability(reliability);
        self
    }

    /// Verify this memory (boosts confidence)
    pub fn verify(&mut self) {
        self.confidence.verify();
        self.updated_at = Utc::now();
    }

    /// Corroborate this memory with another source
    pub fn corroborate(&mut self, source: &str) {
        self.confidence.corroborate(source);
        self.updated_at = Utc::now();
    }

    /// Check if memory has reliable confidence
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.confidence.is_reliable(threshold)
    }

    /// Get confidence tier
    pub fn confidence_tier(&self) -> crate::confidence::ConfidenceTier {
        self.confidence.tier()
    }
}

/// Types of memories with different default importance levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Objective fact
    Fact,
    /// User preference
    Preference,
    /// Decision that was made
    Decision,
    /// Core identity information (never decays)
    Identity,
    /// Something that happened
    Event,
    /// Pattern or observation
    Observation,
    /// Goal to achieve
    Goal,
    /// Actionable task
    Todo,
    /// Consolidated summary of older memories
    Summary,
}

impl MemoryType {
    /// All memory types
    pub const ALL: &[MemoryType] = &[
        MemoryType::Fact,
        MemoryType::Preference,
        MemoryType::Decision,
        MemoryType::Identity,
        MemoryType::Event,
        MemoryType::Observation,
        MemoryType::Goal,
        MemoryType::Todo,
        MemoryType::Summary,
    ];

    /// Get default importance for this type
    pub fn default_importance(&self) -> f32 {
        match self {
            MemoryType::Identity => 1.0,
            MemoryType::Goal => 0.9,
            MemoryType::Decision => 0.8,
            MemoryType::Todo => 0.8,
            MemoryType::Preference => 0.7,
            MemoryType::Fact => 0.6,
            MemoryType::Event => 0.4,
            MemoryType::Observation => 0.3,
            MemoryType::Summary => 0.5,
        }
    }

    /// Check if this type should decay over time
    pub fn can_decay(&self) -> bool {
        !matches!(self, MemoryType::Identity)
    }

    /// Check if this type is a summary/consolidation
    pub fn is_summary(&self) -> bool {
        matches!(self, MemoryType::Summary)
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Fact => write!(f, "fact"),
            MemoryType::Preference => write!(f, "preference"),
            MemoryType::Decision => write!(f, "decision"),
            MemoryType::Identity => write!(f, "identity"),
            MemoryType::Event => write!(f, "event"),
            MemoryType::Observation => write!(f, "observation"),
            MemoryType::Goal => write!(f, "goal"),
            MemoryType::Todo => write!(f, "todo"),
            MemoryType::Summary => write!(f, "summary"),
        }
    }
}

/// Association between two memories (graph edge)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Association {
    /// Unique identifier
    pub id: String,
    /// Source memory ID
    pub source_id: MemoryId,
    /// Target memory ID
    pub target_id: MemoryId,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Weight of the association (0.0 - 1.0)
    pub weight: f32,
    /// When the association was created
    pub created_at: DateTime<Utc>,
}

impl Association {
    /// Create a new association
    pub fn new(
        source_id: impl Into<MemoryId>,
        target_id: impl Into<MemoryId>,
        relation_type: RelationType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.into(),
            target_id: target_id.into(),
            relation_type,
            weight: 0.5,
            created_at: Utc::now(),
        }
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
}

/// Types of relationships between memories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// General semantic connection
    RelatedTo,
    /// Newer version of same information
    Updates,
    /// Conflicting information
    Contradicts,
    /// Causal relationship (target caused source)
    CausedBy,
    /// Result relationship (source results from target)
    ResultOf,
    /// Hierarchical relationship (source is part of target)
    PartOf,
}

impl RelationType {
    /// Multiplier for search scoring based on relation type
    pub fn score_multiplier(&self) -> f64 {
        match self {
            RelationType::Updates => 1.5,
            RelationType::CausedBy | RelationType::ResultOf => 1.3,
            RelationType::RelatedTo => 1.0,
            RelationType::Contradicts => 0.5,
            RelationType::PartOf => 0.8,
        }
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::RelatedTo => write!(f, "related_to"),
            RelationType::Updates => write!(f, "updates"),
            RelationType::Contradicts => write!(f, "contradicts"),
            RelationType::CausedBy => write!(f, "caused_by"),
            RelationType::ResultOf => write!(f, "result_of"),
            RelationType::PartOf => write!(f, "part_of"),
        }
    }
}

/// Search result combining memory with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    /// The memory
    pub memory: Memory,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
    /// Rank in results (1-based)
    pub rank: usize,
}

/// Input for creating a memory
#[derive(Debug, Clone)]
pub struct CreateMemoryInput {
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: Option<f32>,
    pub source: Option<String>,
    pub session_id: Option<SessionId>,
    pub metadata: Option<serde_json::Value>,
}

impl CreateMemoryInput {
    pub fn new(content: impl Into<String>, memory_type: MemoryType) -> Self {
        Self {
            content: content.into(),
            memory_type,
            importance: None,
            source: None,
            session_id: None,
            metadata: None,
        }
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = Some(importance);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<SessionId>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Input for creating an association
#[derive(Debug, Clone)]
pub struct CreateAssociationInput {
    pub target_id: MemoryId,
    pub relation_type: RelationType,
    pub weight: f32,
}
