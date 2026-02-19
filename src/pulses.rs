//! Synaptic Pulses - Event system for memory changes
//!
//! This module implements a publish-subscribe system for memory events,
//! allowing agents to react to memory changes in real-time.
//!
//! Features:
//! - Subscribe to memory creation, updates, and deletion
//! - Real-time notifications
//! - Pattern-based filtering
//! - Multiple subscriber support
//!
//! Example:
//! ```rust,no_run
//! use goldfish::{MemorySystem, GoldfishPulses, Pulse};
//!
//! #[tokio::main]
//! async fn main() {
//!     let memory = MemorySystem::new("./data").await.unwrap();
//!     let pulses = memory.pulses();
//!
//!     // Subscribe to all new memories
//!     let mut subscriber = pulses.subscribe();
//!     tokio::spawn(async move {
//!         while let Ok(pulse) = subscriber.recv().await {
//!             match pulse {
//!                 Pulse::NewMemory { memory, .. } => println!("New: {}", memory.content),
//!                 _ => {}
//!             }
//!         }
//!     });
//! }
//! ```

use crate::types::{Association, Memory, MemoryId, MemoryType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing;

/// A pulse (event) in the synaptic system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Pulse {
    /// A new memory was created
    NewMemory {
        memory: Memory,
        timestamp: DateTime<Utc>,
    },

    /// A memory was updated
    MemoryUpdated {
        memory_id: MemoryId,
        old_content: Option<String>,
        new_content: String,
        changes: Vec<ChangeType>,
        timestamp: DateTime<Utc>,
    },

    /// A memory was accessed (read)
    MemoryAccessed {
        memory_id: MemoryId,
        access_count: i64,
        timestamp: DateTime<Utc>,
    },

    /// A memory was soft-deleted (forgotten)
    MemoryForgotten {
        memory_id: MemoryId,
        timestamp: DateTime<Utc>,
    },

    /// A memory was permanently deleted
    MemoryDeleted {
        memory_id: MemoryId,
        timestamp: DateTime<Utc>,
    },

    /// A new association was created
    AssociationCreated {
        association: Association,
        source_memory: Option<Memory>,
        target_memory: Option<Memory>,
        timestamp: DateTime<Utc>,
    },

    /// Memory confidence changed significantly
    ConfidenceChanged {
        memory_id: MemoryId,
        old_score: f32,
        new_score: f32,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// A contradiction was detected between memories
    ContradictionDetected {
        memory_id: MemoryId,
        conflicting_id: MemoryId,
        description: String,
        timestamp: DateTime<Utc>,
    },

    /// An insight was synthesized from multiple memories
    InsightGenerated {
        insight: String,
        related_memories: Vec<MemoryId>,
        confidence: f32,
        timestamp: DateTime<Utc>,
    },

    /// Maintenance operation completed
    MaintenanceCompleted {
        decayed: usize,
        pruned: usize,
        merged: usize,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// Search was performed
    SearchPerformed {
        query: String,
        results_count: usize,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// Batch operation completed
    BatchCompleted {
        operation: String,
        count: usize,
        success: bool,
        timestamp: DateTime<Utc>,
    },
}

impl Pulse {
    /// Get the timestamp of the pulse
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Pulse::NewMemory { timestamp, .. } => *timestamp,
            Pulse::MemoryUpdated { timestamp, .. } => *timestamp,
            Pulse::MemoryAccessed { timestamp, .. } => *timestamp,
            Pulse::MemoryForgotten { timestamp, .. } => *timestamp,
            Pulse::MemoryDeleted { timestamp, .. } => *timestamp,
            Pulse::AssociationCreated { timestamp, .. } => *timestamp,
            Pulse::ConfidenceChanged { timestamp, .. } => *timestamp,
            Pulse::ContradictionDetected { timestamp, .. } => *timestamp,
            Pulse::InsightGenerated { timestamp, .. } => *timestamp,
            Pulse::MaintenanceCompleted { timestamp, .. } => *timestamp,
            Pulse::SearchPerformed { timestamp, .. } => *timestamp,
            Pulse::BatchCompleted { timestamp, .. } => *timestamp,
        }
    }

    /// Get the memory ID if applicable
    pub fn memory_id(&self) -> Option<&str> {
        match self {
            Pulse::NewMemory { memory, .. } => Some(&memory.id),
            Pulse::MemoryUpdated { memory_id, .. } => Some(memory_id),
            Pulse::MemoryAccessed { memory_id, .. } => Some(memory_id),
            Pulse::MemoryForgotten { memory_id, .. } => Some(memory_id),
            Pulse::MemoryDeleted { memory_id, .. } => Some(memory_id),
            Pulse::ConfidenceChanged { memory_id, .. } => Some(memory_id),
            Pulse::ContradictionDetected { memory_id, .. } => Some(memory_id),
            _ => None,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            Pulse::NewMemory { memory, .. } => {
                format!("New {} memory created: {}", memory.memory_type, memory.id)
            }
            Pulse::MemoryUpdated {
                memory_id, changes, ..
            } => {
                format!("Memory {} updated: {:?}", memory_id, changes)
            }
            Pulse::MemoryAccessed {
                memory_id,
                access_count,
                ..
            } => {
                format!("Memory {} accessed (count: {})", memory_id, access_count)
            }
            Pulse::MemoryForgotten { memory_id, .. } => {
                format!("Memory {} forgotten", memory_id)
            }
            Pulse::MemoryDeleted { memory_id, .. } => {
                format!("Memory {} deleted", memory_id)
            }
            Pulse::AssociationCreated { association, .. } => {
                format!(
                    "Association created: {} -> {} ({:?})",
                    association.source_id, association.target_id, association.relation_type
                )
            }
            Pulse::ConfidenceChanged {
                memory_id,
                old_score,
                new_score,
                reason,
                ..
            } => {
                format!(
                    "Confidence for {} changed: {:.2} -> {:.2} ({})",
                    memory_id, old_score, new_score, reason
                )
            }
            Pulse::ContradictionDetected {
                memory_id,
                conflicting_id,
                ..
            } => {
                format!(
                    "Contradiction detected between {} and {}",
                    memory_id, conflicting_id
                )
            }
            Pulse::InsightGenerated { insight, .. } => {
                format!("Insight generated: {}", insight)
            }
            Pulse::MaintenanceCompleted {
                decayed,
                pruned,
                merged,
                ..
            } => {
                format!(
                    "Maintenance: {} decayed, {} pruned, {} merged",
                    decayed, pruned, merged
                )
            }
            Pulse::SearchPerformed {
                query,
                results_count,
                ..
            } => {
                format!("Search for '{}' returned {} results", query, results_count)
            }
            Pulse::BatchCompleted {
                operation,
                count,
                success,
                ..
            } => {
                format!(
                    "Batch {}: {} items (success: {})",
                    operation, count, success
                )
            }
        }
    }
}

/// Types of changes that can occur in a memory update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChangeType {
    Content,
    Importance,
    MemoryType,
    Confidence,
    Metadata,
    Source,
    Verification,
}

/// Filter for subscribing to specific pulse types
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PulseFilter {
    /// Only receive pulses for specific memory types
    pub memory_types: Option<Vec<MemoryType>>,

    /// Only receive pulses with confidence above threshold
    pub min_confidence: Option<f32>,

    /// Only receive pulses for specific pulse types
    pub pulse_types: Option<Vec<PulseType>>,

    /// Pattern to match in content (regex)
    pub content_pattern: Option<String>,

    /// Maximum age of pulses to receive (in seconds)
    pub max_age_seconds: Option<u64>,
}

impl PulseFilter {
    /// Create a new filter that accepts all pulses
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by memory type
    pub fn with_memory_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_types = Some(vec![mem_type]);
        self
    }

    /// Filter by minimum confidence
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    /// Filter by pulse type
    pub fn with_pulse_type(mut self, pulse_type: PulseType) -> Self {
        self.pulse_types = Some(vec![pulse_type]);
        self
    }

    /// Check if a pulse matches this filter
    pub fn matches(&self, pulse: &Pulse) -> bool {
        // Check pulse type filter
        if let Some(ref types) = self.pulse_types {
            let pulse_type: PulseType = pulse.into();
            if !types.contains(&pulse_type) {
                return false;
            }
        }

        // Check memory type filter
        if let Some(ref mem_types) = self.memory_types {
            match pulse {
                Pulse::NewMemory { memory, .. } => {
                    if !mem_types.contains(&memory.memory_type) {
                        return false;
                    }
                }
                _ => return false, // Non-memory pulses don't match memory type filter
            }
        }

        // Check confidence filter
        if let Some(min_conf) = self.min_confidence {
            if let Pulse::NewMemory { memory, .. } = pulse {
                if memory.confidence.score < min_conf {
                    return false;
                }
            }
        }

        // Check max age filter
        if let Some(max_age) = self.max_age_seconds {
            let age = Utc::now() - pulse.timestamp();
            if age.num_seconds() > max_age as i64 {
                return false;
            }
        }

        // Check content pattern
        if let Some(ref pattern) = self.content_pattern {
            // Simple substring match for now, could use regex
            let content = match pulse {
                Pulse::NewMemory { memory, .. } => Some(memory.content.as_str()),
                Pulse::MemoryUpdated { new_content, .. } => Some(new_content.as_str()),
                Pulse::InsightGenerated { insight, .. } => Some(insight.as_str()),
                _ => None,
            };

            if let Some(content) = content {
                if !content.contains(pattern) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Types of pulses (for filtering)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PulseType {
    NewMemory,
    MemoryUpdated,
    MemoryAccessed,
    MemoryForgotten,
    MemoryDeleted,
    AssociationCreated,
    ConfidenceChanged,
    ContradictionDetected,
    InsightGenerated,
    MaintenanceCompleted,
    SearchPerformed,
    BatchCompleted,
}

impl From<&Pulse> for PulseType {
    fn from(pulse: &Pulse) -> Self {
        match pulse {
            Pulse::NewMemory { .. } => PulseType::NewMemory,
            Pulse::MemoryUpdated { .. } => PulseType::MemoryUpdated,
            Pulse::MemoryAccessed { .. } => PulseType::MemoryAccessed,
            Pulse::MemoryForgotten { .. } => PulseType::MemoryForgotten,
            Pulse::MemoryDeleted { .. } => PulseType::MemoryDeleted,
            Pulse::AssociationCreated { .. } => PulseType::AssociationCreated,
            Pulse::ConfidenceChanged { .. } => PulseType::ConfidenceChanged,
            Pulse::ContradictionDetected { .. } => PulseType::ContradictionDetected,
            Pulse::InsightGenerated { .. } => PulseType::InsightGenerated,
            Pulse::MaintenanceCompleted { .. } => PulseType::MaintenanceCompleted,
            Pulse::SearchPerformed { .. } => PulseType::SearchPerformed,
            Pulse::BatchCompleted { .. } => PulseType::BatchCompleted,
        }
    }
}

/// Synaptic Pulses - Event bus for memory system
#[derive(Debug, Clone)]
pub struct GoldfishPulses {
    /// Broadcast channel sender
    sender: broadcast::Sender<Pulse>,

    /// Configuration
    config: PulseConfig,

    /// Stats for monitoring
    stats: Arc<RwLock<PulseStats>>,
}

impl GoldfishPulses {
    /// Create a new pulse system
    pub fn new(config: PulseConfig) -> Self {
        let (sender, _) = broadcast::channel(config.channel_capacity);

        Self {
            sender,
            config,
            stats: Arc::new(RwLock::new(PulseStats::default())),
        }
    }

    /// Subscribe to all pulses
    pub fn subscribe(&self) -> broadcast::Receiver<Pulse> {
        self.sender.subscribe()
    }

    /// Subscribe with a filter
    pub fn subscribe_filtered(&self, filter: PulseFilter) -> FilteredSubscriber {
        let receiver = self.sender.subscribe();
        FilteredSubscriber::new(receiver, filter)
    }

    /// Emit a pulse
    pub async fn emit(&self, pulse: Pulse) {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_emitted += 1;
            stats.last_emitted = Some(pulse.timestamp());

            let pulse_type: PulseType = (&pulse).into();
            *stats.by_type.entry(pulse_type).or_insert(0) += 1;
        }

        // Send to all subscribers
        if let Err(e) = self.sender.send(pulse) {
            tracing::warn!("Failed to emit pulse: {}", e);
        }
    }

    /// Get current stats
    pub async fn stats(&self) -> PulseStats {
        self.stats.read().await.clone()
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the pulse system configuration
    pub fn config(&self) -> &PulseConfig {
        &self.config
    }
}

impl Default for GoldfishPulses {
    fn default() -> Self {
        Self::new(PulseConfig::default())
    }
}

/// Filtered subscriber that only yields matching pulses
pub struct FilteredSubscriber {
    receiver: broadcast::Receiver<Pulse>,
    filter: PulseFilter,
}

impl FilteredSubscriber {
    /// Create a new filtered subscriber
    fn new(receiver: broadcast::Receiver<Pulse>, filter: PulseFilter) -> Self {
        Self { receiver, filter }
    }

    /// Receive the next matching pulse
    pub async fn recv(&mut self) -> Option<Pulse> {
        loop {
            match self.receiver.recv().await {
                Ok(pulse) => {
                    if self.filter.matches(&pulse) {
                        return Some(pulse);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Option<Pulse> {
        loop {
            match self.receiver.try_recv() {
                Ok(pulse) => {
                    if self.filter.matches(&pulse) {
                        return Some(pulse);
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

/// Configuration for pulse system
#[derive(Debug, Clone)]
pub struct PulseConfig {
    /// Channel capacity (buffer size)
    pub channel_capacity: usize,

    /// Enable pulse persistence
    pub persist_pulses: bool,

    /// Maximum pulses to persist
    pub max_persisted: usize,

    /// Log all pulses
    pub log_pulses: bool,
}

impl Default for PulseConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 1000,
            persist_pulses: false,
            max_persisted: 10000,
            log_pulses: true,
        }
    }
}

/// Statistics for pulse monitoring
#[derive(Debug, Clone, Default)]
pub struct PulseStats {
    pub total_emitted: u64,
    pub last_emitted: Option<DateTime<Utc>>,
    pub by_type: std::collections::HashMap<PulseType, u64>,
}

/// Helper functions for creating common pulses
pub mod pulse {
    use super::*;

    /// Create a new memory pulse
    pub fn new_memory(memory: Memory) -> Pulse {
        Pulse::NewMemory {
            memory,
            timestamp: Utc::now(),
        }
    }

    /// Create a memory updated pulse
    pub fn memory_updated(
        memory_id: MemoryId,
        old_content: Option<String>,
        new_content: String,
        changes: Vec<ChangeType>,
    ) -> Pulse {
        Pulse::MemoryUpdated {
            memory_id,
            old_content,
            new_content,
            changes,
            timestamp: Utc::now(),
        }
    }

    /// Create a confidence changed pulse
    pub fn confidence_changed(
        memory_id: MemoryId,
        old_score: f32,
        new_score: f32,
        reason: impl Into<String>,
    ) -> Pulse {
        Pulse::ConfidenceChanged {
            memory_id,
            old_score,
            new_score,
            reason: reason.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a maintenance completed pulse
    pub fn maintenance_completed(
        decayed: usize,
        pruned: usize,
        merged: usize,
        duration_ms: u64,
    ) -> Pulse {
        Pulse::MaintenanceCompleted {
            decayed,
            pruned,
            merged,
            duration_ms,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Memory, MemoryType};

    #[tokio::test]
    async fn test_pulse_emission() {
        let pulses = GoldfishPulses::default();
        let mut subscriber = pulses.subscribe();

        let memory = Memory::new("Test", MemoryType::Fact);
        let pulse = pulse::new_memory(memory);

        pulses.emit(pulse.clone()).await;

        let received = subscriber.recv().await.unwrap();
        assert_eq!(received, pulse);
    }

    #[test]
    fn test_filter_matches() {
        let filter = PulseFilter::new()
            .with_memory_type(MemoryType::Fact)
            .with_min_confidence(0.7);

        let mut memory = Memory::new("Test", MemoryType::Fact);
        memory.confidence.score = 0.8;
        let pulse = pulse::new_memory(memory);

        assert!(filter.matches(&pulse));
    }

    #[test]
    fn test_filter_rejects_wrong_type() {
        let filter = PulseFilter::new().with_memory_type(MemoryType::Goal);

        let memory = Memory::new("Test", MemoryType::Fact);
        let pulse = pulse::new_memory(memory);

        assert!(!filter.matches(&pulse));
    }
}
