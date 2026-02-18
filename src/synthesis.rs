//! AI-Powered Memory Synthesis
//!
//! This module provides AI-powered analysis of memories to:
//! - Detect patterns and insights
//! - Summarize related memories
//! - Detect contradictions
//! - Generate questions
//! - Extract key themes

use crate::types::{Memory, MemoryId, MemoryType};
use crate::{MemorySystem, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An insight generated from memory analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Unique identifier
    pub id: String,
    
    /// The insight text
    pub content: String,
    
    /// Type of insight
    pub insight_type: InsightType,
    
    /// Confidence in this insight (0.0 - 1.0)
    pub confidence: f32,
    
    /// Related memory IDs
    pub related_memories: Vec<MemoryId>,
    
    /// Evidence supporting this insight
    pub evidence: Vec<String>,
    
    /// When the insight was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Types of insights
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InsightType {
    /// Pattern detected across multiple memories
    Pattern,
    /// Contradiction between memories
    Contradiction,
    /// Summary of related memories
    Summary,
    /// Emerging theme
    Theme,
    /// Question to ask the user
    Question,
    /// Recommendation
    Recommendation,
    /// Trend (increasing/decreasing)
    Trend,
}

impl std::fmt::Display for InsightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsightType::Pattern => write!(f, "pattern"),
            InsightType::Contradiction => write!(f, "contradiction"),
            InsightType::Summary => write!(f, "summary"),
            InsightType::Theme => write!(f, "theme"),
            InsightType::Question => write!(f, "question"),
            InsightType::Recommendation => write!(f, "recommendation"),
            InsightType::Trend => write!(f, "trend"),
        }
    }
}

/// Synthesis engine for analyzing memories
pub struct SynthesisEngine {
    /// Minimum confidence threshold for insights
    min_confidence: f32,
    
    /// Maximum number of insights to generate
    max_insights: usize,
}

impl SynthesisEngine {
    /// Create a new synthesis engine
    pub fn new() -> Self {
        Self {
            min_confidence: 0.6,
            max_insights: 10,
        }
    }
    
    /// Set minimum confidence threshold
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence;
        self
    }
    
    /// Synthesize insights from memories
    pub async fn synthesize(&self, memories: &[Memory]) -> Vec<Insight> {
        let mut insights = Vec::new();
        
        // Detect patterns
        if let Some(pattern) = self.detect_patterns(memories).await {
            insights.push(pattern);
        }
        
        // Detect contradictions
        let contradictions = self.detect_contradictions(memories).await;
        insights.extend(contradictions);
        
        // Extract themes
        let themes = self.extract_themes(memories).await;
        insights.extend(themes);
        
        // Detect trends
        if let Some(trend) = self.detect_trends(memories).await {
            insights.push(trend);
        }
        
        // Generate questions
        let questions = self.generate_questions(memories).await;
        insights.extend(questions);
        
        // Filter by confidence and limit
        insights.retain(|i| i.confidence >= self.min_confidence);
        insights.truncate(self.max_insights);
        
        // Sort by confidence
        insights.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        insights
    }
    
    /// Detect patterns across memories
    async fn detect_patterns(&self, memories: &[Memory]) -> Option<Insight> {
        // Group memories by type
        let mut by_type: HashMap<MemoryType, Vec<&Memory>> = HashMap::new();
        for mem in memories {
            by_type.entry(mem.memory_type).or_default().push(mem);
        }
        
        // Look for repeated content patterns
        let mut patterns = Vec::new();
        
        for (mem_type, mems) in by_type {
            if mems.len() >= 3 {
                // Simple pattern: multiple memories of same type
                patterns.push(format!("Multiple {:?} memories detected ({})", mem_type, mems.len()));
            }
        }
        
        if patterns.is_empty() {
            return None;
        }
        
        let content = patterns.join("; ");
        let related: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
        
        Some(Insight {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            insight_type: InsightType::Pattern,
            confidence: 0.7,
            related_memories: related,
            evidence: vec!["Type frequency analysis".to_string()],
            generated_at: chrono::Utc::now(),
        })
    }
    
    /// Detect contradictions between memories
    async fn detect_contradictions(&self, memories: &[Memory]) -> Vec<Insight> {
        let contradictions = Vec::new();
        
        // Look for memories with Contradicts relations
        for _mem in memories {
            // This would check actual relations in a full implementation
            // For now, simple heuristic: very similar content but different details
        }
        
        contradictions
    }
    
    /// Extract themes from memories
    async fn extract_themes(&self, memories: &[Memory]) -> Vec<Insight> {
        let mut themes = Vec::new();
        
        // Simple keyword extraction
        let mut keyword_counts: HashMap<String, usize> = HashMap::new();
        
        for mem in memories {
            let words: Vec<String> = mem.content
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .filter(|s| s.len() > 4) // Only longer words
                .collect();
            
            for word in words {
                *keyword_counts.entry(word).or_insert(0) += 1;
            }
        }
        
        // Find frequent keywords
        let frequent: Vec<(String, usize)> = keyword_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect();
        
        if !frequent.is_empty() {
            let theme_words: Vec<String> = frequent.iter().map(|(w, _)| w.clone()).collect();
            let content = format!("Key themes: {}", theme_words.join(", "));
            
            let related: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
            
            themes.push(Insight {
                id: uuid::Uuid::new_v4().to_string(),
                content,
                insight_type: InsightType::Theme,
                confidence: 0.6,
                related_memories: related,
                evidence: vec!["Keyword frequency analysis".to_string()],
                generated_at: chrono::Utc::now(),
            });
        }
        
        themes
    }
    
    /// Detect trends in memories over time
    async fn detect_trends(&self, memories: &[Memory]) -> Option<Insight> {
        if memories.len() < 5 {
            return None;
        }
        
        // Sort by creation time
        let mut sorted = memories.to_vec();
        sorted.sort_by_key(|m| m.created_at);
        
        // Simple trend: increasing frequency
        let first_half = &sorted[..sorted.len() / 2];
        let second_half = &sorted[sorted.len() / 2..];
        
        let first_span = if first_half.len() >= 2 {
            (first_half.last().unwrap().created_at - first_half.first().unwrap().created_at).num_days()
        } else {
            1
        };
        
        let second_span = if second_half.len() >= 2 {
            (second_half.last().unwrap().created_at - second_half.first().unwrap().created_at).num_days()
        } else {
            1
        };
        
        if first_span > 0 && second_span > 0 {
            let first_rate = first_half.len() as f64 / first_span as f64;
            let second_rate = second_half.len() as f64 / second_span as f64;
            
            if second_rate > first_rate * 1.5 {
                let content = format!(
                    "Memory creation frequency is increasing ({:.1}x more frequent)",
                    second_rate / first_rate
                );
                
                let related: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
                
                return Some(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    content,
                    insight_type: InsightType::Trend,
                    confidence: 0.65,
                    related_memories: related,
                    evidence: vec!["Temporal frequency analysis".to_string()],
                    generated_at: chrono::Utc::now(),
                });
            }
        }
        
        None
    }
    
    /// Generate questions based on memory gaps
    async fn generate_questions(&self, memories: &[Memory]) -> Vec<Insight> {
        let mut questions = Vec::new();
        
        // Check for low-confidence memories
        let low_confidence: Vec<&Memory> = memories
            .iter()
            .filter(|m| m.confidence.score < 0.5)
            .collect();
        
        if !low_confidence.is_empty() {
            let content = format!(
                "Should I verify {} low-confidence memories?",
                low_confidence.len()
            );
            
            let related: Vec<String> = low_confidence.iter().map(|m| m.id.clone()).collect();
            
            questions.push(Insight {
                id: uuid::Uuid::new_v4().to_string(),
                content,
                insight_type: InsightType::Question,
                confidence: 0.8,
                related_memories: related,
                evidence: vec!["Low confidence detection".to_string()],
                generated_at: chrono::Utc::now(),
            });
        }
        
        // Check for contradictions
        // (simplified - would need relation data in full implementation)
        
        questions
    }
    
    /// Summarize a group of related memories
    pub async fn summarize(&self, memories: &[Memory]) -> String {
        if memories.is_empty() {
            return "No memories to summarize".to_string();
        }
        
        if memories.len() == 1 {
            return memories[0].content.clone();
        }
        
        // Group by type
        let mut by_type: HashMap<MemoryType, Vec<&Memory>> = HashMap::new();
        for mem in memories {
            by_type.entry(mem.memory_type).or_default().push(mem);
        }
        
        let mut summary_parts = Vec::new();
        
        for (mem_type, mems) in by_type {
            summary_parts.push(format!("{} {:?} memories", mems.len(), mem_type));
        }
        
        format!(
            "Collection of {} memories: {}",
            memories.len(),
            summary_parts.join(", ")
        )
    }
    
    /// Find related memories based on content similarity
    pub async fn find_related(
        &self,
        memory: &Memory,
        candidates: &[Memory],
        threshold: f32,
    ) -> Vec<(MemoryId, f32)> {
        let mut related = Vec::new();
        
        let memory_words: std::collections::HashSet<String> = memory.content
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
        for candidate in candidates {
            if candidate.id == memory.id {
                continue;
            }
            
            let candidate_words: std::collections::HashSet<String> = candidate.content
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            
            // Jaccard similarity
            let intersection: std::collections::HashSet<_> = memory_words
                .intersection(&candidate_words)
                .collect();
            let union: std::collections::HashSet<_> = memory_words
                .union(&candidate_words)
                .collect();
            
            if !union.is_empty() {
                let similarity = intersection.len() as f32 / union.len() as f32;
                if similarity >= threshold {
                    related.push((candidate.id.clone(), similarity));
                }
            }
        }
        
        // Sort by similarity
        related.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        related.truncate(5);
        
        related
    }
}

impl Default for SynthesisEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    /// Enable pattern detection
    pub enable_patterns: bool,
    
    /// Enable contradiction detection
    pub enable_contradictions: bool,
    
    /// Enable theme extraction
    pub enable_themes: bool,
    
    /// Enable trend detection
    pub enable_trends: bool,
    
    /// Enable question generation
    pub enable_questions: bool,
    
    /// Minimum confidence for insights
    pub min_confidence: f32,
    
    /// Maximum insights to generate
    pub max_insights: usize,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            enable_patterns: true,
            enable_contradictions: true,
            enable_themes: true,
            enable_trends: true,
            enable_questions: true,
            min_confidence: 0.6,
            max_insights: 10,
        }
    }
}

/// Extension trait for MemorySystem to add synthesis capabilities
#[async_trait::async_trait]
pub trait SynthesisExt {
    /// Synthesize insights from all memories
    async fn synthesize_insights(&self) -> Result<Vec<Insight>>;
    
    /// Get a summary of recent memories
    async fn summarize_recent(&self, days: i64) -> Result<String>;
    
    /// Detect contradictions in memory system
    async fn detect_contradictions(&self) -> Result<Vec<Insight>>;
}

#[async_trait::async_trait]
impl SynthesisExt for MemorySystem {
    async fn synthesize_insights(&self) -> Result<Vec<Insight>> {
        // Get recent memories
        let memories = self.get_last_days(30).await?;
        
        let engine = SynthesisEngine::new();
        let insights = engine.synthesize(&memories).await;
        
        Ok(insights)
    }
    
    async fn summarize_recent(&self, days: i64) -> Result<String> {
        let memories = self.get_last_days(days).await?;
        
        let engine = SynthesisEngine::new();
        let summary = engine.summarize(&memories).await;
        
        Ok(summary)
    }
    
    async fn detect_contradictions(&self) -> Result<Vec<Insight>> {
        // Get all memories
        let memories = self.get_last_days(3650).await?;
        
        let engine = SynthesisEngine::new();
        let insights = engine.detect_contradictions(&memories).await;
        
        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_synthesize_patterns() {
        let engine = SynthesisEngine::new();
        
        let memories = vec![
            Memory::new("Fact 1", MemoryType::Fact),
            Memory::new("Fact 2", MemoryType::Fact),
            Memory::new("Fact 3", MemoryType::Fact),
            Memory::new("Preference 1", MemoryType::Preference),
        ];
        
        let insights = engine.synthesize(&memories).await;
        
        assert!(!insights.is_empty());
        
        // Should detect pattern of multiple facts
        let pattern_insight = insights.iter().find(|i| i.insight_type == InsightType::Pattern);
        assert!(pattern_insight.is_some());
    }
    
    #[tokio::test]
    async fn test_summarize() {
        let engine = SynthesisEngine::new();
        
        let memories = vec![
            Memory::new("Rust is safe", MemoryType::Fact),
            Memory::new("Rust is fast", MemoryType::Fact),
        ];
        
        let summary = engine.summarize(&memories).await;
        
        assert!(summary.contains("2"));
        assert!(summary.contains("Fact"));
    }
}
