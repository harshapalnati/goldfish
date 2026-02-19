//! Memory confidence scoring system
//!
//! This module implements research-backed confidence scoring for memories,
//! based on cognitive science and AI uncertainty quantification research.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Confidence score for a memory (0.0 - 1.0)
///
/// Based on research:
/// - Koriat (2012): Self-Consistency Model of Subjective Confidence
/// - Cohen et al. (2021): Uncertainty in Deep Retrieval Models
/// - Shorinwa et al. (2024): Uncertainty Quantification in LLMs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryConfidence {
    /// Overall confidence score (0.0 - 1.0)
    ///
    /// This is a composite score calculated from all confidence factors.
    /// 1.0 = Maximum confidence, 0.0 = No confidence
    pub score: f32,

    /// Individual confidence factors
    pub factors: ConfidenceFactors,

    /// Current verification status
    pub status: VerificationStatus,

    /// History of confidence changes for audit trail
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub history: Vec<ConfidenceHistory>,

    /// When confidence was last updated
    pub updated_at: DateTime<Utc>,
}

impl MemoryConfidence {
    /// Create new confidence with default values
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            score: 0.5, // Start with neutral confidence
            factors: ConfidenceFactors::default(),
            status: VerificationStatus::Unverified,
            history: Vec::new(),
            updated_at: now,
        }
    }

    /// Create confidence with specific source reliability
    pub fn with_source_reliability(reliability: SourceReliability) -> Self {
        let mut confidence = Self::new();
        confidence.factors.source_reliability = reliability.score();
        confidence.factors.source_type = Some(reliability);
        confidence.recalculate();
        confidence
    }

    /// Recalculate overall confidence score from factors
    pub fn recalculate(&mut self) {
        let old_score = self.score;

        // Weighted combination of factors
        // Based on multicalibration research (Detommaso et al., 2024)
        let weights = ConfidenceWeights::default();

        self.score = (self.factors.source_reliability * weights.source_reliability
            + self.factors.consistency_score * weights.consistency
            + self.factors.retrieval_stability * weights.retrieval_stability
            + self.factors.user_verification * weights.user_verification
            + self.factors.corroboration_boost())
        .clamp(0.0, 1.0);

        // Record history if significant change
        if (self.score - old_score).abs() > 0.05 {
            self.history.push(ConfidenceHistory {
                timestamp: Utc::now(),
                old_score,
                new_score: self.score,
                reason: self.status.to_string(),
            });
        }

        self.updated_at = Utc::now();
    }

    /// Boost confidence when memory is corroborated by another source
    pub fn corroborate(&mut self, source: &str) {
        self.factors.corroboration_count += 1;
        self.factors.corroboration_sources.push(source.to_string());

        // Diminishing returns for multiple corroborations
        // Based on research on confidence calibration
        let boost = 0.1 / (self.factors.corroboration_count as f32).sqrt();
        self.factors.corroboration_score = (self.factors.corroboration_score + boost).min(1.0);

        self.recalculate();
    }

    /// Reduce confidence when contradiction is detected
    pub fn flag_contradiction(&mut self, conflicting_memory_id: &str) {
        self.factors
            .contradictions
            .push(conflicting_memory_id.to_string());
        self.factors.consistency_score *= 0.7; // Reduce consistency
        self.status = VerificationStatus::Contradicted;
        self.recalculate();
    }

    /// Mark as user-verified (highest confidence)
    pub fn verify(&mut self) {
        self.factors.user_verification = 1.0;
        self.status = VerificationStatus::UserConfirmed;
        self.recalculate();
    }

    /// Apply temporal decay to confidence
    ///
    /// Based on Ebbinghaus forgetting curve research
    pub fn decay(&mut self, days: i64) {
        if days <= 0 {
            return;
        }

        // Exponential decay with half-life of 30 days
        let decay_factor = 0.5_f32.powf(days as f32 / 30.0);

        // Only decay certain factors
        self.factors.retrieval_stability *= decay_factor;
        self.factors.consistency_score =
            (self.factors.consistency_score * 0.95 + 0.05 * decay_factor).min(1.0);

        self.recalculate();
    }

    /// Check if confidence meets threshold for reliable retrieval
    pub fn is_reliable(&self, threshold: f32) -> bool {
        self.score >= threshold
    }

    /// Get confidence tier (for display/organization)
    pub fn tier(&self) -> ConfidenceTier {
        match self.score {
            s if s >= 0.9 => ConfidenceTier::High,
            s if s >= 0.7 => ConfidenceTier::Medium,
            s if s >= 0.4 => ConfidenceTier::Low,
            _ => ConfidenceTier::Unreliable,
        }
    }

    /// Get explanation of confidence score
    pub fn explanation(&self) -> String {
        format!(
            "Confidence: {:.2} (Source: {:.2}, Consistency: {:.2}, Corroborated: {}x, Status: {})",
            self.score,
            self.factors.source_reliability,
            self.factors.consistency_score,
            self.factors.corroboration_count,
            self.status
        )
    }
}

impl Default for MemoryConfidence {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual factors contributing to confidence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceFactors {
    /// Reliability of the source (0.0 - 1.0)
    pub source_reliability: f32,

    /// Type of source (affects base reliability)
    pub source_type: Option<SourceReliability>,

    /// Self-consistency across retrievals (Koriat, 2012)
    pub consistency_score: f32,

    /// Stability of retrieval scores over time
    pub retrieval_stability: f32,

    /// User verification (1.0 = verified, 0.0 = not verified)
    pub user_verification: f32,

    /// Number of corroborating sources
    pub corroboration_count: u32,

    /// Score boost from corroboration
    pub corroboration_score: f32,

    /// List of corroborating source IDs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub corroboration_sources: Vec<String>,

    /// List of contradicting memory IDs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contradictions: Vec<String>,

    /// Evidence or sources for this memory
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence: Vec<String>,
}

impl ConfidenceFactors {
    /// Calculate corroboration boost (diminishing returns)
    fn corroboration_boost(&self) -> f32 {
        if self.corroboration_count == 0 {
            0.0
        } else {
            // Logarithmic scaling: 1 source = 0.1, 2 sources = 0.15, 3+ = ~0.18
            0.1 * (1.0 + (self.corroboration_count as f32).ln_1p())
        }
    }
}

impl Default for ConfidenceFactors {
    fn default() -> Self {
        Self {
            source_reliability: 0.5,
            source_type: None,
            consistency_score: 0.5,
            retrieval_stability: 0.5,
            user_verification: 0.0,
            corroboration_count: 0,
            corroboration_score: 0.0,
            corroboration_sources: Vec::new(),
            contradictions: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

/// Source reliability types
///
/// Based on research showing experts have better calibration
/// (Lichtenstein & Fischhoff, 1977)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SourceReliability {
    /// User explicitly verified
    UserVerified,
    /// Multiple authoritative sources agree
    AuthoritativeConsensus,
    /// Single authoritative source
    Authoritative,
    /// Multiple independent sources
    MultipleSources,
    /// LLM high confidence (>90%)
    LLMHighConfidence,
    /// LLM medium confidence
    LLMMediumConfidence,
    /// Single unverified source
    SingleSource,
    /// Heuristic or inferred
    Inferred,
    /// Uncertain origin
    Uncertain,
}

impl SourceReliability {
    /// Get base reliability score
    pub fn score(&self) -> f32 {
        match self {
            SourceReliability::UserVerified => 1.0,
            SourceReliability::AuthoritativeConsensus => 0.95,
            SourceReliability::Authoritative => 0.85,
            SourceReliability::MultipleSources => 0.8,
            SourceReliability::LLMHighConfidence => 0.75,
            SourceReliability::LLMMediumConfidence => 0.6,
            SourceReliability::SingleSource => 0.5,
            SourceReliability::Inferred => 0.4,
            SourceReliability::Uncertain => 0.3,
        }
    }
}

impl std::fmt::Display for SourceReliability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceReliability::UserVerified => write!(f, "user_verified"),
            SourceReliability::AuthoritativeConsensus => write!(f, "authoritative_consensus"),
            SourceReliability::Authoritative => write!(f, "authoritative"),
            SourceReliability::MultipleSources => write!(f, "multiple_sources"),
            SourceReliability::LLMHighConfidence => write!(f, "llm_high_confidence"),
            SourceReliability::LLMMediumConfidence => write!(f, "llm_medium_confidence"),
            SourceReliability::SingleSource => write!(f, "single_source"),
            SourceReliability::Inferred => write!(f, "inferred"),
            SourceReliability::Uncertain => write!(f, "uncertain"),
        }
    }
}

/// Verification status of a memory
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VerificationStatus {
    /// Never verified
    Unverified,
    /// Single source, not confirmed
    Tentative,
    /// Multiple sources corroborate
    Corroborated,
    /// User explicitly confirmed
    UserConfirmed,
    /// Conflicts detected with other memories
    Contradicted,
    /// Deprecated by newer information
    Superseded,
}

impl VerificationStatus {
    /// Get base confidence modifier
    pub fn confidence_modifier(&self) -> f32 {
        match self {
            VerificationStatus::Unverified => 0.5,
            VerificationStatus::Tentative => 0.6,
            VerificationStatus::Corroborated => 0.8,
            VerificationStatus::UserConfirmed => 1.0,
            VerificationStatus::Contradicted => 0.3,
            VerificationStatus::Superseded => 0.2,
        }
    }
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Unverified => write!(f, "unverified"),
            VerificationStatus::Tentative => write!(f, "tentative"),
            VerificationStatus::Corroborated => write!(f, "corroborated"),
            VerificationStatus::UserConfirmed => write!(f, "user_confirmed"),
            VerificationStatus::Contradicted => write!(f, "contradicted"),
            VerificationStatus::Superseded => write!(f, "superseded"),
        }
    }
}

/// Confidence tiers for organization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConfidenceTier {
    /// High confidence (>= 0.9)
    High,
    /// Medium confidence (0.7 - 0.9)
    Medium,
    /// Low confidence (0.4 - 0.7)
    Low,
    /// Unreliable (< 0.4)
    Unreliable,
}

impl std::fmt::Display for ConfidenceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceTier::High => write!(f, "high"),
            ConfidenceTier::Medium => write!(f, "medium"),
            ConfidenceTier::Low => write!(f, "low"),
            ConfidenceTier::Unreliable => write!(f, "unreliable"),
        }
    }
}

/// Weights for confidence factor combination
///
/// Based on multicalibration research
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceWeights {
    pub source_reliability: f32,
    pub consistency: f32,
    pub retrieval_stability: f32,
    pub user_verification: f32,
}

impl Default for ConfidenceWeights {
    fn default() -> Self {
        Self {
            source_reliability: 0.35,
            consistency: 0.25,
            retrieval_stability: 0.20,
            user_verification: 0.20,
        }
    }
}

/// History entry for confidence changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceHistory {
    pub timestamp: DateTime<Utc>,
    pub old_score: f32,
    pub new_score: f32,
    pub reason: String,
}

/// Configuration for confidence scoring behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    /// Minimum confidence for reliable retrieval
    pub min_reliable_confidence: f32,

    /// Enable automatic confidence decay
    pub enable_decay: bool,

    /// Days between decay applications
    pub decay_interval_days: i64,

    /// Enable contradiction detection
    pub enable_contradiction_detection: bool,

    /// Similarity threshold for contradiction (0.0 - 1.0)
    pub contradiction_similarity_threshold: f32,

    /// Enable auto-corroboration for similar memories
    pub enable_auto_corroboration: bool,

    /// Similarity threshold for auto-corroboration
    pub auto_corroboration_threshold: f32,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            min_reliable_confidence: 0.6,
            enable_decay: true,
            decay_interval_days: 7,
            enable_contradiction_detection: true,
            contradiction_similarity_threshold: 0.85,
            enable_auto_corroboration: true,
            auto_corroboration_threshold: 0.92,
        }
    }
}
