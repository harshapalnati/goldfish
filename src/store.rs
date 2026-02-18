//! Memory graph storage using SQLite

use crate::confidence::VerificationStatus;
use crate::cortex::{Experience, MemorySummary};
use crate::error::{MemoryError, Result};
use crate::types::{Association, Memory, MemoryId, MemoryType, RelationType};

use sqlx::{Row, SqlitePool};
use std::sync::Arc;

/// Memory store for CRUD and graph operations
#[derive(Clone)]
pub struct MemoryStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl MemoryStore {
    /// Create a new memory store with the given SQLite pool
    pub fn new(pool: SqlitePool) -> Arc<Self> {
        Arc::new(Self { pool })
    }

    /// Get a reference to the SQLite pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Save a new memory
    pub async fn save(&self, memory: &Memory) -> Result<()> {
        let metadata_json = memory
            .metadata
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok());

        let confidence_json = serde_json::to_string(&memory.confidence).ok();

        sqlx::query(
            r#"
            INSERT INTO memories (
                id, content, memory_type, importance, created_at, updated_at,
                last_accessed_at, access_count, source, session_id, forgotten, metadata,
                confidence_score, confidence_data, verification_status
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&memory.id)
        .bind(&memory.content)
        .bind(memory.memory_type.to_string())
        .bind(memory.importance)
        .bind(memory.created_at)
        .bind(memory.updated_at)
        .bind(memory.last_accessed_at)
        .bind(memory.access_count)
        .bind(&memory.source)
        .bind(memory.session_id.as_ref())
        .bind(memory.forgotten)
        .bind(metadata_json)
        .bind(memory.confidence.score)
        .bind(confidence_json)
        .bind(memory.confidence.status.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load a memory by ID
    pub async fn load(&self, id: &str) -> Result<Option<Memory>> {
        let row = sqlx::query(
            r#"
            SELECT id, content, memory_type, importance, created_at, updated_at,
                   last_accessed_at, access_count, source, session_id, forgotten, metadata,
                   confidence_score, confidence_data, verification_status
            FROM memories
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| row_to_memory(&row)))
    }

    /// Update an existing memory
    pub async fn update(&self, memory: &Memory) -> Result<()> {
        let metadata_json = memory
            .metadata
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok());

        let confidence_json = serde_json::to_string(&memory.confidence).ok();

        sqlx::query(
            r#"
            UPDATE memories
            SET content = ?, memory_type = ?, importance = ?, updated_at = ?,
                last_accessed_at = ?, access_count = ?, source = ?, session_id = ?,
                forgotten = ?, metadata = ?, confidence_score = ?, confidence_data = ?,
                verification_status = ?
            WHERE id = ?
            "#,
        )
        .bind(&memory.content)
        .bind(memory.memory_type.to_string())
        .bind(memory.importance)
        .bind(memory.updated_at)
        .bind(memory.last_accessed_at)
        .bind(memory.access_count)
        .bind(&memory.source)
        .bind(memory.session_id.as_ref())
        .bind(memory.forgotten)
        .bind(metadata_json)
        .bind(memory.confidence.score)
        .bind(confidence_json)
        .bind(memory.confidence.status.to_string())
        .bind(&memory.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a memory permanently
    pub async fn delete(&self, id: &str) -> Result<()> {
        // First delete associations
        sqlx::query(
            "DELETE FROM associations WHERE source_id = ? OR target_id = ?"
        )
        .bind(id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        // Then delete the memory
        sqlx::query("DELETE FROM memories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Soft delete (forget) a memory
    pub async fn forget(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE memories SET forgotten = 1, updated_at = ? WHERE id = ? AND forgotten = 0"
        )
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Restore a forgotten memory
    pub async fn restore(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE memories SET forgotten = 0, updated_at = ? WHERE id = ? AND forgotten = 1"
        )
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record access to a memory
    pub async fn record_access(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE memories
            SET last_accessed_at = ?, access_count = access_count + 1
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create an association between memories
    pub async fn create_association(&self, association: &Association) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO associations (id, source_id, target_id, relation_type, weight, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(source_id, target_id, relation_type) DO UPDATE SET
                weight = excluded.weight
            "#,
        )
        .bind(&association.id)
        .bind(&association.source_id)
        .bind(&association.target_id)
        .bind(association.relation_type.to_string())
        .bind(association.weight)
        .bind(association.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all associations for a memory
    pub async fn get_associations(&self, memory_id: &str) -> Result<Vec<Association>> {
        let rows = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, weight, created_at
            FROM associations
            WHERE source_id = ? OR target_id = ?
            "#,
        )
        .bind(memory_id)
        .bind(memory_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_association).collect())
    }

    /// Get associations between a set of memories
    pub async fn get_associations_between(
        &self,
        memory_ids: &[String],
    ) -> Result<Vec<Association>> {
        if memory_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: String = memory_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query_str = format!(
            "SELECT id, source_id, target_id, relation_type, weight, created_at \
             FROM associations \
             WHERE source_id IN ({placeholders}) AND target_id IN ({placeholders})"
        );

        let mut query = sqlx::query(&query_str);
        for id in memory_ids {
            query = query.bind(id);
        }
        for id in memory_ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(row_to_association).collect())
    }

    /// Get neighbors in the graph (memories connected by associations)
    pub async fn get_neighbors(
        &self,
        memory_id: &str,
        depth: u32,
        exclude_ids: &[String],
    ) -> Result<(Vec<Memory>, Vec<Association>)> {
        let mut visited: std::collections::HashSet<String> =
            exclude_ids.iter().cloned().collect();
        visited.insert(memory_id.to_string());

        let mut all_associations = Vec::new();
        let mut frontier = vec![memory_id.to_string()];

        for _ in 0..depth {
            if frontier.is_empty() {
                break;
            }

            let mut next_frontier = Vec::new();
            for node_id in &frontier {
                let associations = self.get_associations(node_id).await?;
                for assoc in associations {
                    let neighbor_id = if assoc.source_id == *node_id {
                        &assoc.target_id
                    } else {
                        &assoc.source_id
                    };

                    if !visited.contains(neighbor_id) {
                        visited.insert(neighbor_id.clone());
                        next_frontier.push(neighbor_id.clone());
                    }
                    all_associations.push(assoc);
                }
            }
            frontier = next_frontier;
        }

        // Deduplicate associations
        let mut seen = std::collections::HashSet::new();
        all_associations.retain(|a| seen.insert(a.id.clone()));

        // Load neighbor memories
        let neighbor_ids: Vec<String> = visited
            .into_iter()
            .filter(|id| !exclude_ids.contains(id) && id != memory_id)
            .collect();

        let mut neighbors = Vec::new();
        for id in &neighbor_ids {
            if let Some(memory) = self.load(id).await? {
                if !memory.forgotten {
                    neighbors.push(memory);
                }
            }
        }

        Ok((neighbors, all_associations))
    }

    /// Get memories by type
    pub async fn get_by_type(
        &self,
        memory_type: MemoryType,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let rows = sqlx::query(
            r#"
            SELECT id, content, memory_type, importance, created_at, updated_at,
                   last_accessed_at, access_count, source, session_id, forgotten, metadata
            FROM memories
            WHERE memory_type = ? AND forgotten = 0
            ORDER BY importance DESC, updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(memory_type.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_memory).collect())
    }

    /// Get high-importance memories
    pub async fn get_high_importance(
        &self,
        threshold: f32,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let rows = sqlx::query(
            r#"
            SELECT id, content, memory_type, importance, created_at, updated_at,
                   last_accessed_at, access_count, source, session_id, forgotten, metadata
            FROM memories
            WHERE importance >= ? AND forgotten = 0
            ORDER BY importance DESC, updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_memory).collect())
    }

    /// Get memories sorted by various criteria
    pub async fn get_sorted(
        &self,
        sort: SortOrder,
        limit: i64,
        memory_type: Option<MemoryType>,
    ) -> Result<Vec<Memory>> {
        let order_clause = match sort {
            SortOrder::Recent => "ORDER BY created_at DESC",
            SortOrder::Updated => "ORDER BY updated_at DESC",
            SortOrder::Importance => "ORDER BY importance DESC, updated_at DESC",
            SortOrder::MostAccessed => "ORDER BY access_count DESC, created_at DESC",
            SortOrder::LastAccessed => "ORDER BY last_accessed_at DESC",
        };

        let (query_str, type_filter) = if let Some(ref memory_type) = memory_type {
            (
                format!(
                    "SELECT id, content, memory_type, importance, created_at, updated_at, \
                     last_accessed_at, access_count, source, session_id, forgotten, metadata \
                     FROM memories WHERE memory_type = ? AND forgotten = 0 {order_clause} LIMIT ?"
                ),
                Some(memory_type.to_string()),
            )
        } else {
            (
                format!(
                    "SELECT id, content, memory_type, importance, created_at, updated_at, \
                     last_accessed_at, access_count, source, session_id, forgotten, metadata \
                     FROM memories WHERE forgotten = 0 {order_clause} LIMIT ?"
                ),
                None,
            )
        };

        let rows = if let Some(type_str) = type_filter {
            sqlx::query(&query_str)
                .bind(type_str)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(&query_str)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        };

        Ok(rows.iter().map(row_to_memory).collect())
    }

    /// Get memories eligible for pruning
    pub async fn get_pruning_candidates(
        &self,
        importance_threshold: f32,
        min_age_days: i64,
    ) -> Result<Vec<Memory>> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(min_age_days);

        let rows = sqlx::query(
            r#"
            SELECT id, content, memory_type, importance, created_at, updated_at,
                   last_accessed_at, access_count, source, session_id, forgotten, metadata
            FROM memories
            WHERE importance < ?
              AND memory_type != 'identity'
              AND created_at < ?
              AND forgotten = 0
            ORDER BY importance ASC, created_at ASC
            "#,
        )
        .bind(importance_threshold)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_memory).collect())
    }

    /// Query memories with a custom SQL filter
    pub async fn query_with_filter(&self, filter: &str, limit: i64) -> Result<Vec<Memory>> {
        let query = format!(
            r#"
            SELECT id, content, memory_type, importance, created_at, updated_at,
                   last_accessed_at, access_count, source, session_id, forgotten, metadata,
                   confidence_score, confidence_data, verification_status
            FROM memories
            WHERE forgotten = 0 AND ({filter})
            ORDER BY created_at DESC
            LIMIT ?
            "#
        );

        let rows = sqlx::query(&query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(row_to_memory).collect())
    }

    /// Create an in-memory store for testing
    pub async fn connect_in_memory() -> Arc<Self> {
        use sqlx::sqlite::SqliteConnectOptions;

        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .create_if_missing(true);

        let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory SQLite");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        Arc::new(Self { pool })
    }

    // ─── Experience (Episodic Memory) CRUD ─────────────────────────────────

    /// Save a new experience
    pub async fn save_experience(&self, experience: &Experience) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO experiences (id, title, context, started_at, ended_at, importance)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&experience.id)
        .bind(&experience.title)
        .bind(&experience.context)
        .bind(experience.started_at)
        .bind(experience.ended_at)
        .bind(experience.importance)
        .execute(&self.pool)
        .await?;

        // Insert memory links
        for mem_id in &experience.memory_ids {
            let _ = self.add_memory_to_experience(&experience.id, mem_id).await;
        }

        Ok(())
    }

    /// Add a memory to an experience
    pub async fn add_memory_to_experience(
        &self,
        experience_id: &str,
        memory_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO experience_memories (experience_id, memory_id, added_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(experience_id)
        .bind(memory_id)
        .bind(chrono::Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update an experience (end time, importance)
    pub async fn update_experience(&self, experience: &Experience) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE experiences
            SET title = ?, context = ?, ended_at = ?, importance = ?
            WHERE id = ?
            "#,
        )
        .bind(&experience.title)
        .bind(&experience.context)
        .bind(experience.ended_at)
        .bind(experience.importance)
        .bind(&experience.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load an experience by ID with its memory IDs
    pub async fn load_experience(&self, id: &str) -> Result<Option<Experience>> {
        let row = sqlx::query(
            r#"
            SELECT id, title, context, started_at, ended_at, importance
            FROM experiences
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let exp_id: String = row.try_get("id").unwrap_or_default();
                let memory_ids = self.get_experience_memory_ids(&exp_id).await?;

                Ok(Some(Experience {
                    id: exp_id,
                    title: row.try_get("title").unwrap_or_default(),
                    context: row.try_get("context").unwrap_or_default(),
                    memory_ids,
                    started_at: row.try_get("started_at").unwrap_or_else(|_| chrono::Utc::now()),
                    ended_at: row.try_get("ended_at").ok(),
                    importance: row.try_get("importance").unwrap_or(0.5),
                }))
            }
            None => Ok(None),
        }
    }

    /// List experiences with pagination
    pub async fn list_experiences(&self, limit: i64, offset: i64) -> Result<Vec<Experience>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, context, started_at, ended_at, importance
            FROM experiences
            ORDER BY started_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut experiences = Vec::new();
        for row in &rows {
            let exp_id: String = row.try_get("id").unwrap_or_default();
            let memory_ids = self.get_experience_memory_ids(&exp_id).await?;

            experiences.push(Experience {
                id: exp_id,
                title: row.try_get("title").unwrap_or_default(),
                context: row.try_get("context").unwrap_or_default(),
                memory_ids,
                started_at: row.try_get("started_at").unwrap_or_else(|_| chrono::Utc::now()),
                ended_at: row.try_get("ended_at").ok(),
                importance: row.try_get("importance").unwrap_or(0.5),
            });
        }

        Ok(experiences)
    }

    /// Get memory IDs for an experience
    async fn get_experience_memory_ids(&self, experience_id: &str) -> Result<Vec<MemoryId>> {
        let rows = sqlx::query(
            "SELECT memory_id FROM experience_memories WHERE experience_id = ? ORDER BY added_at",
        )
        .bind(experience_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.try_get("memory_id").unwrap_or_default()).collect())
    }

    // ─── Memory Summary CRUD ──────────────────────────────────────────────

    /// Save a memory summary
    pub async fn save_summary(&self, summary: &MemorySummary) -> Result<()> {
        let original_ids_json = serde_json::to_string(&summary.original_memory_ids)
            .map_err(|e| MemoryError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO memory_summaries (id, summary_text, original_memory_ids, memory_type, created_at, importance)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&summary.id)
        .bind(&summary.summary_text)
        .bind(&original_ids_json)
        .bind(summary.memory_type.to_string())
        .bind(summary.created_at)
        .bind(summary.importance)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all memory summaries
    pub async fn get_summaries(&self) -> Result<Vec<MemorySummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, summary_text, original_memory_ids, memory_type, created_at, importance
            FROM memory_summaries
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::new();
        for row in &rows {
            let original_ids_json: String = row.try_get("original_memory_ids").unwrap_or_default();
            let original_memory_ids: Vec<MemoryId> =
                serde_json::from_str(&original_ids_json).unwrap_or_default();

            let mem_type_str: String = row.try_get("memory_type").unwrap_or_default();

            summaries.push(MemorySummary {
                id: row.try_get("id").unwrap_or_default(),
                summary_text: row.try_get("summary_text").unwrap_or_default(),
                original_memory_ids,
                memory_type: parse_memory_type(&mem_type_str),
                created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now()),
                importance: row.try_get("importance").unwrap_or(0.5),
            });
        }

        Ok(summaries)
    }
}

/// Sort order for queries
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    /// Most recent first
    #[default]
    Recent,
    /// Most recently updated first
    Updated,
    /// Highest importance first
    Importance,
    /// Most accessed first
    MostAccessed,
    /// Last accessed first
    LastAccessed,
}

/// Helper: Convert database row to Memory
fn row_to_memory(row: &sqlx::sqlite::SqliteRow) -> Memory {
    use sqlx::Row;
    use crate::confidence::MemoryConfidence;

    let mem_type_str: String = row.try_get("memory_type").unwrap_or_default();
    let memory_type = parse_memory_type(&mem_type_str);

    let metadata_json: Option<String> = row.try_get("metadata").ok();
    let metadata = metadata_json.and_then(|s| serde_json::from_str(&s).ok());

    // Parse confidence data
    let confidence_data: Option<String> = row.try_get("confidence_data").ok();
    let mut confidence: MemoryConfidence = confidence_data
        .and_then(|s| serde_json::from_str::<MemoryConfidence>(&s).ok())
        .unwrap_or_default();
    
    // Override with stored score if present
    if let Ok(score) = row.try_get::<f32, _>("confidence_score") {
        confidence.score = score;
    }
    
    // Parse verification status
    if let Ok(status_str) = row.try_get::<String, _>("verification_status") {
        confidence.status = parse_verification_status(&status_str);
    }

    Memory {
        id: row.try_get("id").unwrap_or_default(),
        content: row.try_get("content").unwrap_or_default(),
        memory_type,
        importance: row.try_get("importance").unwrap_or(0.5),
        priority: row.try_get("importance").unwrap_or(0.5),
        emotional_valence: 0.0,
        tags: Vec::new(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: row.try_get("updated_at").unwrap_or_else(|_| chrono::Utc::now()),
        last_accessed_at: row
            .try_get("last_accessed_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
        access_count: row.try_get("access_count").unwrap_or(0),
        source: row.try_get("source").ok(),
        session_id: row.try_get("session_id").ok(),
        forgotten: row.try_get::<bool, _>("forgotten").unwrap_or(false),
        metadata,
        confidence,
    }
}

/// Helper: Parse verification status from string
fn parse_verification_status(s: &str) -> VerificationStatus {
    match s {
        "unverified" => VerificationStatus::Unverified,
        "tentative" => VerificationStatus::Tentative,
        "corroborated" => VerificationStatus::Corroborated,
        "user_confirmed" => VerificationStatus::UserConfirmed,
        "contradicted" => VerificationStatus::Contradicted,
        "superseded" => VerificationStatus::Superseded,
        _ => VerificationStatus::Unverified,
    }
}

/// Helper: Parse memory type from string
fn parse_memory_type(s: &str) -> MemoryType {
    match s {
        "fact" => MemoryType::Fact,
        "preference" => MemoryType::Preference,
        "decision" => MemoryType::Decision,
        "identity" => MemoryType::Identity,
        "event" => MemoryType::Event,
        "observation" => MemoryType::Observation,
        "goal" => MemoryType::Goal,
        "todo" => MemoryType::Todo,
        "summary" => MemoryType::Summary,
        _ => MemoryType::Fact,
    }
}

/// Helper: Convert database row to Association
fn row_to_association(row: &sqlx::sqlite::SqliteRow) -> Association {
    use sqlx::Row;

    let relation_type_str: String = row.try_get("relation_type").unwrap_or_default();
    let relation_type = parse_relation_type(&relation_type_str);

    Association {
        id: row.try_get("id").unwrap_or_default(),
        source_id: row.try_get("source_id").unwrap_or_default(),
        target_id: row.try_get("target_id").unwrap_or_default(),
        relation_type,
        weight: row.try_get("weight").unwrap_or(0.5),
        created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now()),
    }
}

/// Helper: Parse relation type from string
fn parse_relation_type(s: &str) -> RelationType {
    match s {
        "related_to" => RelationType::RelatedTo,
        "updates" => RelationType::Updates,
        "contradicts" => RelationType::Contradicts,
        "caused_by" => RelationType::CausedBy,
        "result_of" => RelationType::ResultOf,
        "part_of" => RelationType::PartOf,
        _ => RelationType::RelatedTo,
    }
}
