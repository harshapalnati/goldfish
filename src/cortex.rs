//! # Agentic Memory Cortex
//!
//! A sophisticated memory system designed for AI agents with:
//! - Working Memory: Fast cache with TTL, pinning, and focus
//! - Episodic Memory: Persistent grouped experiences
//! - Importance Scoring: Exponential decay + query relevance
//! - Context Windows: Token-budgeted context for LLMs
//! - Memory Summaries: Consolidation of old memories

use crate::error::{MemoryError, Result};
use crate::search::{MemorySearch, SearchConfig, SearchMode};
use crate::types::{Association, Memory, MemoryId, MemorySearchResult, MemoryType, RelationType};
use crate::vector_search::{generate_embedding, VectorIndex, VectorSearchConfig};
use crate::MemoryStore;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ─── Working Memory ───────────────────────────────────────────────────────────

/// Working memory - fast cache for active context
/// What the agent is currently thinking about / needs to remember
#[derive(Debug, Clone)]
pub struct WorkingMemory {
    items: Vec<WorkingMemoryItem>,
    max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub memory_id: MemoryId,
    pub content: String,
    pub memory_type: MemoryType,
    pub accessed_at: DateTime<Utc>,
    pub attention_score: f32,
    /// If set, item auto-expires after this time
    pub expires_at: Option<DateTime<Utc>>,
    /// Pinned items survive decay and eviction
    pub pinned: bool,
}

impl WorkingMemory {
    pub fn new(max_items: usize) -> Self {
        Self {
            items: Vec::new(),
            max_items,
        }
    }

    /// Add or update an item in working memory with optional TTL
    pub fn remember(&mut self, memory: &Memory, ttl: Option<Duration>) {
        let expires_at = ttl.map(|d| Utc::now() + d);

        if let Some(item) = self.items.iter_mut().find(|i| i.memory_id == memory.id) {
            item.accessed_at = Utc::now();
            item.attention_score = (item.attention_score + 0.1).min(1.0);
            item.content = memory.content.clone();
            if let Some(exp) = expires_at {
                item.expires_at = Some(exp);
            }
        } else {
            self.items.push(WorkingMemoryItem {
                memory_id: memory.id.clone(),
                content: memory.content.clone(),
                memory_type: memory.memory_type,
                accessed_at: Utc::now(),
                attention_score: 0.5,
                expires_at,
                pinned: false,
            });
        }

        self.cleanup();
    }

    /// Focus on a specific memory - boosts its attention to maximum
    pub fn focus(&mut self, memory_id: &str) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.memory_id == memory_id) {
            item.attention_score = 1.0;
            item.accessed_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Pin a memory so it survives decay and eviction
    pub fn pin(&mut self, memory_id: &str) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.memory_id == memory_id) {
            item.pinned = true;
            true
        } else {
            false
        }
    }

    /// Unpin a memory
    pub fn unpin(&mut self, memory_id: &str) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.memory_id == memory_id) {
            item.pinned = false;
            true
        } else {
            false
        }
    }

    /// Get current context (what agent is thinking about)
    /// Returns pinned items first, then by attention score, filtering expired
    pub fn get_context(&self) -> Vec<&WorkingMemoryItem> {
        let now = Utc::now();
        let mut live: Vec<&WorkingMemoryItem> = self
            .items
            .iter()
            .filter(|i| {
                if let Some(exp) = i.expires_at {
                    exp > now
                } else {
                    true
                }
            })
            .collect();

        // Pinned first, then by attention score
        live.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.attention_score.partial_cmp(&a.attention_score).unwrap())
        });

        live
    }

    /// Get all working memory items (including expired)
    pub fn all(&self) -> &[WorkingMemoryItem] {
        &self.items
    }

    /// Clear working memory
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Decay attention scores (call periodically)
    /// Pinned items don't decay. Expired items are removed.
    pub fn decay(&mut self) {
        let now = Utc::now();

        // Remove expired non-pinned items
        self.items.retain(|i| {
            if i.pinned {
                return true;
            }
            if let Some(exp) = i.expires_at {
                if exp <= now {
                    return false;
                }
            }
            true
        });

        // Decay unpinned attention scores
        for item in &mut self.items {
            if !item.pinned {
                item.attention_score *= 0.95;
            }
        }

        // Remove items below threshold (but not pinned)
        self.items.retain(|i| i.pinned || i.attention_score > 0.1);
    }

    /// Cleanup: remove expired items and enforce capacity
    fn cleanup(&mut self) {
        let now = Utc::now();

        // Remove expired non-pinned items
        self.items.retain(|i| {
            if i.pinned {
                return true;
            }
            if let Some(exp) = i.expires_at {
                return exp > now;
            }
            true
        });

        // Sort: pinned first, then by attention
        self.items.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.attention_score.partial_cmp(&a.attention_score).unwrap())
        });

        // Trim to capacity (but don't evict pinned items)
        if self.items.len() > self.max_items {
            let pinned_count = self.items.iter().filter(|i| i.pinned).count();
            let keep = self.max_items.max(pinned_count);
            self.items.truncate(keep);
        }
    }

    /// Get item count
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ─── Episodic Memory ──────────────────────────────────────────────────────────

/// Experience - a grouping of related memories
/// Represents a "moment" or "experience" in the agent's history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub title: String,
    pub context: String,
    pub memory_ids: Vec<MemoryId>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub importance: f32,
}

impl Experience {
    pub fn new(title: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            context: context.into(),
            memory_ids: Vec::new(),
            started_at: Utc::now(),
            ended_at: None,
            importance: 0.5,
        }
    }

    pub fn add_memory(&mut self, memory_id: MemoryId) {
        if !self.memory_ids.contains(&memory_id) {
            self.memory_ids.push(memory_id);
        }
    }

    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
    }

    pub fn duration(&self) -> Duration {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        end - self.started_at
    }
}

// ─── Importance Scoring ───────────────────────────────────────────────────────

/// Configurable weights for importance calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceWeights {
    /// Weight for base importance (memory type default)
    pub base: f32,
    /// Weight for recency factor
    pub recency: f32,
    /// Weight for access frequency
    pub access_frequency: f32,
    /// Weight for memory type bonus
    pub type_bonus: f32,
    /// Weight for confidence score
    pub confidence: f32,
    /// Weight for query relevance (only used in calculate_with_query)
    pub relevance: f32,
    /// Decay rate lambda for exponential decay (higher = faster decay)
    pub decay_lambda: f32,
}

impl Default for ImportanceWeights {
    fn default() -> Self {
        Self {
            base: 0.30,
            recency: 0.20,
            access_frequency: 0.15,
            type_bonus: 0.15,
            confidence: 0.10,
            relevance: 0.10,
            decay_lambda: 0.01,
        }
    }
}

/// Importance calculator - determines what matters
pub struct ImportanceCalculator;

impl ImportanceCalculator {
    /// Calculate dynamic importance based on multiple factors
    pub fn calculate(memory: &Memory) -> f32 {
        Self::calculate_with_weights(memory, &ImportanceWeights::default())
    }

    /// Calculate with custom weights
    pub fn calculate_with_weights(memory: &Memory, weights: &ImportanceWeights) -> f32 {
        // Base importance from the memory itself
        let base = memory.importance;

        // Recency: exponential decay e^(-λt) where t is hours since last access
        let hours_since_access = (Utc::now() - memory.last_accessed_at).num_hours() as f32;
        let recency = (-weights.decay_lambda * hours_since_access).exp();

        // Access frequency: logarithmic scaling
        let access_freq = (memory.access_count as f32 + 1.0).ln() / 10.0;

        // Type-based bonus
        let type_bonus = match memory.memory_type {
            MemoryType::Identity => 0.5,
            MemoryType::Goal => 0.4,
            MemoryType::Decision => 0.3,
            MemoryType::Preference => 0.2,
            MemoryType::Todo => 0.2,
            MemoryType::Fact | MemoryType::Summary => 0.1,
            MemoryType::Event | MemoryType::Observation => 0.0,
        };

        // Confidence
        let confidence = memory.confidence.score;

        // Weighted combination
        let score = base * weights.base
            + recency * weights.recency
            + access_freq * weights.access_frequency
            + type_bonus * weights.type_bonus
            + confidence * weights.confidence;

        score.clamp(0.0, 1.0)
    }

    /// Calculate importance with query relevance factored in
    pub fn calculate_with_query(memory: &Memory, query: &str) -> f32 {
        let weights = ImportanceWeights::default();
        let base_score = Self::calculate_with_weights(memory, &weights);

        // Simple word overlap relevance
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = memory.content.to_lowercase();

        if query_words.is_empty() {
            return base_score;
        }

        let matching = query_words
            .iter()
            .filter(|w| content_lower.contains(**w))
            .count();
        let relevance = matching as f32 / query_words.len() as f32;

        // Blend base score with relevance
        let blended = base_score * (1.0 - weights.relevance) + relevance * weights.relevance;
        blended.clamp(0.0, 1.0)
    }

    /// Should this memory be consolidated (summarized)?
    pub fn should_consolidate(memory: &Memory, threshold: f32) -> bool {
        let age_days = (Utc::now() - memory.created_at).num_days() as f32;
        let importance = Self::calculate(memory);

        age_days > 30.0 && importance < threshold
    }
}

// ─── Context Window ───────────────────────────────────────────────────────────

/// Context window builder for LLM consumption
/// Assembles the most relevant memories within a token budget
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Maximum token budget (approximate)
    pub max_tokens: usize,
    /// Include working memory items
    pub include_working_memory: bool,
    /// Include current experience
    pub include_experience: bool,
    /// Include high-importance memories
    pub include_important: bool,
    /// Maximum number of important memories to include
    pub max_important: usize,
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self {
            max_tokens: 2000,
            include_working_memory: true,
            include_experience: true,
            include_important: true,
            max_important: 10,
        }
    }
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..Default::default()
        }
    }

    /// Rough token estimation (~0.75 tokens per character)
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() as f64 * 0.75).ceil() as usize
    }

    /// Build context string from the cortex within token budget
    pub async fn build(&self, cortex: &MemoryCortex) -> Result<String> {
        let mut output = String::new();
        let mut remaining_tokens = self.max_tokens;

        // Layer 1: Pinned working memory (always included)
        if self.include_working_memory {
            let context_items = cortex.get_context().await;

            if !context_items.is_empty() {
                let mut section = String::from("## Active Context\n");

                // Pinned items first
                let pinned: Vec<_> = context_items.iter().filter(|i| i.pinned).collect();
                if !pinned.is_empty() {
                    section.push_str("### Pinned\n");
                    for item in &pinned {
                        let line = format!("- [{}] {}\n", item.memory_type, item.content);
                        section.push_str(&line);
                    }
                }

                // Other active items
                let active: Vec<_> = context_items.iter().filter(|i| !i.pinned).collect();
                if !active.is_empty() {
                    section.push_str("### Working Memory\n");
                    for item in &active {
                        let line = format!(
                            "- [{}] {} (attn: {:.2})\n",
                            item.memory_type, item.content, item.attention_score
                        );
                        section.push_str(&line);
                    }
                }

                let tokens = Self::estimate_tokens(&section);
                if tokens <= remaining_tokens {
                    output.push_str(&section);
                    remaining_tokens -= tokens;
                }
            }
        }

        // Layer 2: Current experience
        if self.include_experience {
            if let Some(ep) = cortex.get_current_experience().await {
                let section = format!(
                    "\n## Current Experience: {}\n{}\n- Memories in this experience: {}\n",
                    ep.title,
                    ep.context,
                    ep.memory_ids.len()
                );
                let tokens = Self::estimate_tokens(&section);
                if tokens <= remaining_tokens {
                    output.push_str(&section);
                    remaining_tokens -= tokens;
                }
            }
        }

        // Layer 3: High-importance memories
        if self.include_important && remaining_tokens > 100 {
            let important = cortex.get_important(self.max_important).await?;

            if !important.is_empty() {
                let mut section = String::from("\n## Important Memories\n");
                for mem in &important {
                    let line = format!(
                        "- [{}] {} (importance: {:.2})\n",
                        mem.memory_type, mem.content, mem.importance
                    );
                    let line_tokens = Self::estimate_tokens(&line);
                    if line_tokens > remaining_tokens {
                        break;
                    }
                    section.push_str(&line);
                    remaining_tokens -= line_tokens;
                }
                output.push_str(&section);
            }
        }

        Ok(output)
    }
}

// ─── Memory Summaries / Consolidation ─────────────────────────────────────────

/// A consolidated summary of multiple older memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub id: String,
    pub summary_text: String,
    pub original_memory_ids: Vec<MemoryId>,
    pub memory_type: MemoryType,
    pub created_at: DateTime<Utc>,
    pub importance: f32,
}

impl MemorySummary {
    pub fn new(
        summary_text: impl Into<String>,
        original_ids: Vec<MemoryId>,
        memory_type: MemoryType,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            summary_text: summary_text.into(),
            original_memory_ids: original_ids,
            memory_type,
            created_at: Utc::now(),
            importance: 0.5,
        }
    }
}

// ─── Memory Cortex ────────────────────────────────────────────────────────────

/// Memory cortex - the main agentic memory system
pub struct MemoryCortex {
    store: Arc<MemoryStore>,
    search: MemorySearch,
    working_memory: RwLock<WorkingMemory>,
    current_experience: RwLock<Option<Experience>>,
    data_dir: std::path::PathBuf,
    vector_index: Option<Arc<VectorIndex>>,
}

impl MemoryCortex {
    pub async fn new(data_dir: impl Into<std::path::PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;

        // Initialize SQLite
        let sqlite_path = data_dir.join("memories.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&sqlite_path)
            .create_if_missing(true);

        let pool = sqlx::SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| MemoryError::Database(e.into()))?;

        let store = MemoryStore::new(pool);

        // Initialize search (BM25 index) - MemorySearch::with_dir expects Arc<MemoryStore>
        let search = MemorySearch::with_dir(Arc::clone(&store), &data_dir)?;
        search.reindex_all().await?;

        // Initialize vector index
        let vector_config = VectorSearchConfig {
            dimension: 384,
            index_path: data_dir.join("vectors"),
        };
        let vector_index = Arc::new(VectorIndex::new(vector_config));
        vector_index.init().await?;

        Ok(Self {
            store,
            search,
            working_memory: RwLock::new(WorkingMemory::new(20)),
            current_experience: RwLock::new(None),
            data_dir,
            vector_index: Some(vector_index),
        })
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    // ─── Core Memory Operations ───────────────────────────────────────────

    /// Remember something - adds to working memory and optionally to current episode
    pub async fn remember(&self, memory: &Memory) -> Result<()> {
        self.store.save(memory).await?;

        // Store vector embedding for semantic search
        if let Some(ref vector_index) = self.vector_index {
            let embedding = generate_embedding(&memory.content);
            vector_index.store(&memory.id, embedding).await?;
        }

        // Add to working memory
        let mut wm = self.working_memory.write().await;
        wm.remember(memory, None);

        // Add to current episode if active
        let mut episode = self.current_experience.write().await;
        if let Some(ep) = episode.as_mut() {
            ep.add_memory(memory.id.clone());
            // Persist episode-memory link
            let _ = self
                .store
                .add_memory_to_experience(&ep.id, &memory.id)
                .await;
        }

        Ok(())
    }

    /// Remember with a TTL (auto-expires from working memory)
    pub async fn remember_with_ttl(&self, memory: &Memory, ttl: Duration) -> Result<()> {
        self.store.save(memory).await?;

        let mut wm = self.working_memory.write().await;
        wm.remember(memory, Some(ttl));

        let mut episode = self.current_experience.write().await;
        if let Some(ep) = episode.as_mut() {
            ep.add_memory(memory.id.clone());
            let _ = self
                .store
                .add_memory_to_experience(&ep.id, &memory.id)
                .await;
        }

        Ok(())
    }

    /// Think about something - brings into working memory without saving
    pub async fn think_about(&self, memory_id: &str) -> Result<Option<Memory>> {
        let memory = self.store.load(memory_id).await?;

        if let Some(ref mem) = memory {
            let mut wm = self.working_memory.write().await;
            wm.remember(mem, None);

            // Update access count
            let mut m = mem.clone();
            m.access_count += 1;
            m.last_accessed_at = Utc::now();
            self.store.update(&m).await?;
        }

        Ok(memory)
    }

    /// Focus on a memory in working memory (boost attention to max)
    pub async fn focus(&self, memory_id: &str) -> bool {
        let mut wm = self.working_memory.write().await;
        wm.focus(memory_id)
    }

    /// Pin a memory in working memory
    pub async fn pin(&self, memory_id: &str) -> bool {
        let mut wm = self.working_memory.write().await;
        wm.pin(memory_id)
    }

    /// Unpin a memory in working memory
    pub async fn unpin(&self, memory_id: &str) -> bool {
        let mut wm = self.working_memory.write().await;
        wm.unpin(memory_id)
    }

    /// Get what agent is currently thinking about
    pub async fn get_context(&self) -> Vec<WorkingMemoryItem> {
        let wm = self.working_memory.read().await;
        wm.get_context().into_iter().cloned().collect()
    }

    // ─── Search & Recall ──────────────────────────────────────────────────

    /// Search memories with hybrid ranking (BM25 + importance + recency)
    /// Uses Tantivy for proper BM25 scoring instead of naive text matching
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>> {
        // Use proper BM25 search via Tantivy
        let mut config = SearchConfig::default();
        config.mode = SearchMode::FullText;
        config.max_results = limit * 3; // Get more candidates for re-ranking
        
        let mut results = self.search.search(query, &config).await?;
        
        // Apply importance + recency re-ranking
        let now = Utc::now();
        for result in &mut results {
            if result.memory.forgotten {
                result.score = 0.0;
                continue;
            }
            
            // Base BM25 score (already set by search)
            let bm25_score = result.score;
            
            // Importance boost (0.0 to 1.0)
            let importance_boost = result.memory.importance * 0.20;
            
            // Recency boost - newer memories get higher scores
            let hours_ago = (now - result.memory.last_accessed_at).num_hours().max(0) as f32;
            let recency_boost = 0.15 / (1.0 + hours_ago * 0.01);
            
            // Combine: 65% BM25, 20% importance, 15% recency
            result.score = bm25_score * 0.65 + importance_boost + recency_boost;
        }
        
        // Re-sort by combined score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        // Update ranks
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = i + 1;
        }
        
        Ok(results)
    }

    /// Get important memories (what matters now)
    pub async fn get_important(&self, limit: usize) -> Result<Vec<Memory>> {
        let mut all_memories = Vec::new();
        for mem_type in MemoryType::ALL {
            let memories = self.store.get_by_type(*mem_type, 1000).await?;
            all_memories.extend(memories);
        }

        let mut scored: Vec<(Memory, f32)> = all_memories
            .into_iter()
            .filter(|m| !m.forgotten)
            .map(|m| {
                let score = ImportanceCalculator::calculate(&m);
                (m, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        Ok(scored.into_iter().take(limit).map(|(m, _)| m).collect())
    }

    // ─── Episodic Memory ──────────────────────────────────────────────────

    /// Start a new episode (experience/context) - persisted to DB
    pub async fn start_episode(&self, title: &str, context: &str) -> Result<String> {
        let experience = Experience::new(title, context);
        let id = experience.id.clone();

        // Persist to DB
        self.store.save_experience(&experience).await?;

        let mut episode = self.current_experience.write().await;
        *episode = Some(experience);

        Ok(id)
    }

    /// End current episode - updates DB with end time and computed importance
    pub async fn end_episode(&self) -> Result<Option<Experience>> {
        let mut episode = self.current_experience.write().await;
        if let Some(ep) = episode.as_mut() {
            ep.end();

            // Compute importance from constituent memories
            let mut total_importance = 0.0;
            let mut count = 0;
            for mem_id in &ep.memory_ids {
                if let Ok(Some(mem)) = self.store.load(mem_id).await {
                    total_importance += ImportanceCalculator::calculate(&mem);
                    count += 1;
                }
            }
            if count > 0 {
                ep.importance = total_importance / count as f32;
            }

            // Update in DB
            self.store.update_experience(ep).await?;
        }
        Ok(episode.take())
    }

    /// Get current episode
    pub async fn get_current_experience(&self) -> Option<Experience> {
        let episode = self.current_experience.read().await;
        episode.clone()
    }

    /// List episodes with pagination (from DB)
    pub async fn list_episodes(&self, limit: i64, offset: i64) -> Result<Vec<Experience>> {
        self.store.list_experiences(limit, offset).await
    }

    /// Get a specific episode by ID (from DB)
    pub async fn get_episode(&self, id: &str) -> Result<Option<Experience>> {
        self.store.load_experience(id).await
    }

    /// Get recent episodes
    pub async fn get_recent_episodes(&self, limit: usize) -> Result<Vec<Experience>> {
        self.store.list_experiences(limit as i64, 0).await
    }

    // ─── Graph Operations ─────────────────────────────────────────────────

    /// Get related memories (graph traversal)
    pub async fn get_related(&self, memory_id: &str, depth: u32) -> Result<Vec<Memory>> {
        let (neighbors, _) = self.store.get_neighbors(memory_id, depth, &[]).await?;
        Ok(neighbors)
    }

    /// Get memories from a specific time
    pub async fn get_memories_since(&self, days_ago: i64) -> Result<Vec<Memory>> {
        let since = Utc::now() - Duration::days(days_ago);
        let filter = format!("created_at >= '{}'", since.format("%Y-%m-%d"));
        self.store.query_with_filter(&filter, 1000).await
    }

    // ─── Convenience Methods ──────────────────────────────────────────────

    /// Make a decision and remember it
    pub async fn decide(&self, decision: &str, context: &str, options: &[&str]) -> Result<Memory> {
        let memory = Memory::new(
            format!(
                "Decision: {} - Context: {} - Options: {:?}",
                decision, context, options
            ),
            MemoryType::Decision,
        )
        .with_importance(0.9);

        self.remember(&memory).await?;

        Ok(memory)
    }

    /// Store a preference
    pub async fn prefer(&self, preference: &str, importance: f32) -> Result<Memory> {
        let memory = Memory::new(preference, MemoryType::Preference).with_importance(importance);

        self.remember(&memory).await?;

        Ok(memory)
    }

    /// Set a goal
    pub async fn goal(&self, goal: &str) -> Result<Memory> {
        let memory = Memory::new(goal, MemoryType::Goal).with_importance(0.95);

        self.remember(&memory).await?;

        Ok(memory)
    }

    /// Get pending goals
    pub async fn get_goals(&self) -> Result<Vec<Memory>> {
        self.store.get_by_type(MemoryType::Goal, 100).await
    }

    /// Create association between memories
    pub async fn link(&self, from_id: &str, to_id: &str, relation: RelationType) -> Result<()> {
        let assoc = Association::new(from_id, to_id, relation);
        self.store.create_association(&assoc).await
    }

    // ─── Working Memory Management ────────────────────────────────────────

    /// Working memory decay (call periodically)
    pub async fn decay(&self) {
        let mut wm = self.working_memory.write().await;
        wm.decay();
    }

    /// Clear working memory (for new context)
    pub async fn clear_context(&self) {
        let mut wm = self.working_memory.write().await;
        wm.clear();
    }

    // ─── Context Window ───────────────────────────────────────────────────

    /// Build a context window for LLM consumption
    pub async fn build_context(&self, config: &ContextWindow) -> Result<String> {
        config.build(self).await
    }

    /// Full memory dump for context window (legacy API, delegates to ContextWindow)
    pub async fn get_full_context(&self, _max_memories: usize) -> Result<String> {
        let config = ContextWindow::default();
        config.build(self).await
    }

    // ─── Memory Consolidation ─────────────────────────────────────────────

    /// Consolidate old, low-importance memories into summaries
    /// Returns the number of memories consolidated
    pub async fn consolidate(&self, threshold: f32, max_age_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(max_age_days);
        let filter = format!(
            "created_at < '{}' AND importance < {} AND forgotten = 0",
            cutoff.format("%Y-%m-%d"),
            threshold
        );

        let candidates = self.store.query_with_filter(&filter, 1000).await?;

        if candidates.is_empty() {
            return Ok(0);
        }

        // Group by type
        let mut by_type: HashMap<MemoryType, Vec<Memory>> = HashMap::new();
        for mem in &candidates {
            by_type
                .entry(mem.memory_type)
                .or_default()
                .push(mem.clone());
        }

        let mut consolidated_count = 0;

        for (mem_type, memories) in &by_type {
            if memories.len() < 2 {
                continue;
            }

            // Build summary text from all memories in this group
            let mut summary_parts: Vec<String> = Vec::new();
            let mut original_ids: Vec<MemoryId> = Vec::new();

            for mem in memories {
                // Avoid duplicating very similar content
                let content_trimmed = mem.content.trim();
                if !summary_parts.iter().any(|s| s == content_trimmed) {
                    summary_parts.push(content_trimmed.to_string());
                }
                original_ids.push(mem.id.clone());
            }

            let summary_text = format!(
                "Consolidated {} {} memories: {}",
                memories.len(),
                mem_type,
                summary_parts.join("; ")
            );

            // Create the summary memory
            let summary_memory = Memory::new(&summary_text, MemoryType::Summary)
                .with_importance(0.5)
                .with_metadata(serde_json::json!({
                    "original_ids": original_ids,
                    "consolidated_from_type": mem_type.to_string(),
                    "original_count": memories.len(),
                }));

            self.store.save(&summary_memory).await?;

            // Save summary record
            let summary = MemorySummary::new(&summary_text, original_ids.clone(), *mem_type);
            self.store.save_summary(&summary).await?;

            // Soft-delete originals
            for mem_id in &original_ids {
                self.store.forget(mem_id).await?;
            }

            consolidated_count += memories.len();
        }

        tracing::info!(
            "Consolidated {} memories into summaries",
            consolidated_count
        );
        Ok(consolidated_count)
    }

    /// Get all memory summaries
    pub async fn get_summaries(&self) -> Result<Vec<MemorySummary>> {
        self.store.get_summaries().await
    }
}
