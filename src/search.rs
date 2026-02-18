//! Memory search using Tantivy full-text search engine
//!
//! Provides fast, relevant search over the memory corpus with:
//! - Full-text indexing with BM25 scoring
//! - Fuzzy matching for typo tolerance
//! - Faceted search by memory type
//! - Importance-weighted result ranking

use crate::error::{MemoryError, Result};
use crate::types::{Memory, MemorySearchResult, MemoryType};
use crate::MemoryStore;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, QueryParser};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};

/// Search strategy
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Full-text search via Tantivy (default)
    #[default]
    FullText,
    /// Simple text contains (fallback)
    Text,
    /// Most recent memories
    Recent,
    /// Highest importance memories
    Important,
    /// Filter by type
    Typed,
}

/// Sort order for metadata-based searches
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSort {
    /// Most recent first
    #[default]
    Recent,
    /// Highest importance first
    Importance,
    /// Most accessed first
    MostAccessed,
    /// Last accessed first
    LastAccess,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub mode: SearchMode,
    pub memory_type: Option<MemoryType>,
    pub sort_by: SearchSort,
    pub max_results: usize,
    /// Enable fuzzy matching for typo tolerance
    pub fuzzy: bool,
    /// Boost recently accessed memories in scoring
    pub boost_recent: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            mode: SearchMode::FullText,
            memory_type: None,
            sort_by: SearchSort::Recent,
            max_results: 10,
            fuzzy: false,
            boost_recent: true,
        }
    }
}

/// Tantivy schema field handles
struct SchemaFields {
    id: Field,
    content: Field,
    memory_type: Field,
    source: Field,
    tags: Field,
    importance: Field,
}

/// Full-text memory search powered by Tantivy
pub struct MemorySearch {
    store: Arc<MemoryStore>,
    index: Index,
    reader: IndexReader,
    schema: Schema,
    fields: SchemaFields,
}

impl std::fmt::Debug for MemorySearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemorySearch")
            .field("store", &"<MemoryStore>")
            .field("index", &"<TantivyIndex>")
            .finish()
    }
}

impl Clone for MemorySearch {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            index: self.index.clone(),
            reader: self.index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()
                .expect("Failed to create index reader"),
            schema: self.schema.clone(),
            fields: SchemaFields {
                id: self.fields.id,
                content: self.fields.content,
                memory_type: self.fields.memory_type,
                source: self.fields.source,
                tags: self.fields.tags,
                importance: self.fields.importance,
            },
        }
    }
}

impl MemorySearch {
    /// Build the Tantivy schema for memory indexing
    fn build_schema() -> (Schema, SchemaFields) {
        let mut schema_builder = Schema::builder();

        let id = schema_builder.add_text_field("id", STRING | STORED);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let memory_type = schema_builder.add_text_field("memory_type", STRING | STORED);
        let source = schema_builder.add_text_field("source", STRING | STORED);
        let tags = schema_builder.add_text_field("tags", TEXT | STORED);
        let importance = schema_builder.add_f64_field("importance", FAST | STORED);

        let schema = schema_builder.build();
        let fields = SchemaFields {
            id,
            content,
            memory_type,
            source,
            tags,
            importance,
        };

        (schema, fields)
    }

    /// Create a new MemorySearch with Tantivy index at the given directory
    pub fn with_dir(store: Arc<MemoryStore>, index_dir: impl AsRef<Path>) -> Result<Self> {
        let (schema, fields) = Self::build_schema();
        let index_path = index_dir.as_ref().join("tantivy_index");
        std::fs::create_dir_all(&index_path)
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create index dir: {}", e)))?;

        let index = Index::create_in_dir(&index_path, schema.clone())
            .or_else(|_| Index::open_in_dir(&index_path))
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to open index: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create reader: {}", e)))?;

        Ok(Self {
            store,
            index,
            reader,
            schema,
            fields,
        })
    }

    /// Create a new MemorySearch with an in-memory Tantivy index (for testing)
    pub fn new(store: Arc<MemoryStore>) -> Self {
        let (schema, fields) = Self::build_schema();
        let index = Index::create_in_ram(schema.clone());

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .expect("Failed to create in-memory reader");

        Self {
            store,
            index,
            reader,
            schema,
            fields,
        }
    }

    /// Index a single memory into the Tantivy index
    pub fn index_memory(&self, memory: &Memory) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(15_000_000)
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create writer: {}", e)))?;

        // Delete any existing document with this ID
        let id_term = Term::from_field_text(self.fields.id, &memory.id);
        writer.delete_term(id_term);

        let tags_str = memory.tags.join(" ");
        let source_str = memory.source.as_deref().unwrap_or("");

        writer
            .add_document(doc!(
                self.fields.id => memory.id.as_str(),
                self.fields.content => memory.content.as_str(),
                self.fields.memory_type => memory.memory_type.to_string(),
                self.fields.source => source_str,
                self.fields.tags => tags_str.as_str(),
                self.fields.importance => memory.importance as f64,
            ))
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to add document: {}", e)))?;

        writer
            .commit()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to commit: {}", e)))?;

        // Reload the reader to pick up changes
        self.reader
            .reload()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to reload reader: {}", e)))?;

        Ok(())
    }

    /// Delete a memory document from the Tantivy index by ID
    pub fn delete_memory(&self, id: &str) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(15_000_000)
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create writer: {}", e)))?;

        let id_term = Term::from_field_text(self.fields.id, id);
        writer.delete_term(id_term);

        writer
            .commit()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to commit: {}", e)))?;

        self.reader
            .reload()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to reload reader: {}", e)))?;

        Ok(())
    }

    /// Reindex all memories from the store
    pub async fn reindex_all(&self) -> Result<usize> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create writer: {}", e)))?;

        // Clear existing index
        writer.delete_all_documents()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to clear index: {}", e)))?;

        let mut count = 0;

        for mem_type in MemoryType::ALL {
            let memories = self.store.get_by_type(*mem_type, 10_000).await?;
            for memory in &memories {
                let tags_str = memory.tags.join(" ");
                let source_str = memory.source.as_deref().unwrap_or("");

                writer
                    .add_document(doc!(
                        self.fields.id => memory.id.as_str(),
                        self.fields.content => memory.content.as_str(),
                        self.fields.memory_type => memory.memory_type.to_string(),
                        self.fields.source => source_str,
                        self.fields.tags => tags_str.as_str(),
                        self.fields.importance => memory.importance as f64,
                    ))
                    .map_err(|e| {
                        MemoryError::SearchIndex(format!("Failed to add document: {}", e))
                    })?;

                count += 1;
            }
        }

        writer
            .commit()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to commit: {}", e)))?;

        self.reader
            .reload()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to reload reader: {}", e)))?;

        tracing::info!("Reindexed {} memories", count);
        Ok(count)
    }

    /// Remove a memory from the search index
    pub fn remove_memory(&self, memory_id: &str) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(15_000_000)
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to create writer: {}", e)))?;

        let id_term = Term::from_field_text(self.fields.id, memory_id);
        writer.delete_term(id_term);

        writer
            .commit()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to commit: {}", e)))?;

        self.reader
            .reload()
            .map_err(|e| MemoryError::SearchIndex(format!("Failed to reload reader: {}", e)))?;

        Ok(())
    }

    /// Search memories using Tantivy full-text search
    pub async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<MemorySearchResult>> {
        match config.mode {
            SearchMode::FullText => self.search_fulltext(query, config).await,
            SearchMode::Text => self.search_text_fallback(query, config).await,
            SearchMode::Recent => self.search_metadata(config).await,
            SearchMode::Important => self.search_metadata(config).await,
            SearchMode::Typed => self.search_metadata(config).await,
        }
    }

    /// Full-text search using Tantivy
    async fn search_fulltext(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<MemorySearchResult>> {
        let searcher = self.reader.searcher();

        let scored_ids = if config.fuzzy {
            // Fuzzy search: build fuzzy term queries for each word
            let words: Vec<&str> = query.split_whitespace().collect();
            let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

            for word in &words {
                let term = Term::from_field_text(self.fields.content, word);
                let fuzzy = FuzzyTermQuery::new(term, 1, true);
                subqueries.push((Occur::Should, Box::new(fuzzy)));
            }

            let combined = BooleanQuery::new(subqueries);

            let top_docs = searcher
                .search(&combined, &TopDocs::with_limit(config.max_results))
                .map_err(|e| MemoryError::SearchIndex(format!("Fuzzy search failed: {}", e)))?;

            top_docs
                .into_iter()
                .filter_map(|(score, doc_address)| {
                    let doc: tantivy::TantivyDocument = searcher.doc(doc_address).ok()?;
                    let id = doc.get_first(self.fields.id)?.as_str()?.to_string();
                    Some((id, score))
                })
                .collect::<Vec<_>>()
        } else {
            // Standard query parser search
            let query_parser =
                QueryParser::for_index(&self.index, vec![self.fields.content, self.fields.tags]);

            let parsed_query = query_parser
                .parse_query(query)
                .map_err(|e| MemoryError::SearchIndex(format!("Query parse failed: {}", e)))?;

            // Optional: filter by memory type
            let final_query: Box<dyn tantivy::query::Query> =
                if let Some(ref mem_type) = config.memory_type {
                    let type_term =
                        Term::from_field_text(self.fields.memory_type, &mem_type.to_string());
                    let type_query = tantivy::query::TermQuery::new(
                        type_term,
                        IndexRecordOption::Basic,
                    );
                    Box::new(BooleanQuery::new(vec![
                        (Occur::Must, parsed_query),
                        (Occur::Must, Box::new(type_query)),
                    ]))
                } else {
                    parsed_query
                };

            let top_docs = searcher
                .search(&final_query, &TopDocs::with_limit(config.max_results))
                .map_err(|e| MemoryError::SearchIndex(format!("Search failed: {}", e)))?;

            top_docs
                .into_iter()
                .filter_map(|(score, doc_address)| {
                    let doc: tantivy::TantivyDocument = searcher.doc(doc_address).ok()?;
                    let id = doc.get_first(self.fields.id)?.as_str()?.to_string();
                    Some((id, score))
                })
                .collect::<Vec<_>>()
        };

        // Load full memories from store and build results
        let mut results = Vec::new();
        for (rank, (id, tantivy_score)) in scored_ids.into_iter().enumerate() {
            if let Ok(Some(memory)) = self.store.load(&id).await {
                // Skip forgotten memories
                if memory.forgotten {
                    continue;
                }

                // Combine Tantivy BM25 score with importance
                let mut score = tantivy_score;

                // Boost by importance
                score *= 1.0 + memory.importance * 0.5;

                // Recency boost
                if config.boost_recent {
                    let hours_ago =
                        (chrono::Utc::now() - memory.last_accessed_at).num_hours() as f32;
                    let recency = 1.0 / (1.0 + hours_ago * 0.01);
                    score *= 1.0 + recency * 0.3;
                }

                results.push(MemorySearchResult {
                    memory,
                    score,
                    rank: rank + 1,
                });
            }
        }

        // Re-sort by combined score and update ranks
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        Ok(results)
    }

    /// Fallback: simple text contains matching (for when Tantivy is unavailable)
    async fn search_text_fallback(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<MemorySearchResult>> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for mem_type in MemoryType::ALL {
            // Filter by memory_type if specified
            if let Some(ref filter_type) = config.memory_type {
                if filter_type != mem_type {
                    continue;
                }
            }

            let memories = self.store.get_by_type(*mem_type, 1000).await?;

            for memory in memories {
                if memory.forgotten {
                    continue;
                }

                let content_lower = memory.content.to_lowercase();
                if content_lower.contains(&query_lower) {
                    let score = memory.importance;
                    results.push(MemorySearchResult {
                        memory,
                        score,
                        rank: results.len() + 1,
                    });
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(config.max_results);

        for (i, r) in results.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        Ok(results)
    }

    /// Metadata-based search (recent, important, by type)
    async fn search_metadata(&self, config: &SearchConfig) -> Result<Vec<MemorySearchResult>> {
        let mut all_memories = Vec::new();

        let types_to_search = if let Some(ref t) = config.memory_type {
            vec![*t]
        } else {
            MemoryType::ALL.to_vec()
        };

        for mem_type in types_to_search {
            let memories = self.store.get_by_type(mem_type, 1000).await?;
            all_memories.extend(memories);
        }

        // Filter out forgotten
        all_memories.retain(|m| !m.forgotten);

        // Sort by the requested mode
        match config.sort_by {
            SearchSort::Importance => {
                all_memories.sort_by(|a, b| {
                    b.importance
                        .partial_cmp(&a.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SearchSort::MostAccessed => {
                all_memories.sort_by(|a, b| b.access_count.cmp(&a.access_count));
            }
            SearchSort::LastAccess => {
                all_memories.sort_by(|a, b| b.last_accessed_at.cmp(&a.last_accessed_at));
            }
            SearchSort::Recent => {
                all_memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }

        all_memories.truncate(config.max_results);

        let results = all_memories
            .into_iter()
            .enumerate()
            .map(|(i, memory)| MemorySearchResult {
                score: memory.importance,
                memory,
                rank: i + 1,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::{MemorySystem, Memory, MemoryType};

    #[tokio::test]
    async fn fulltext_search_finds_saved_memory() {
        let dir = tempfile::tempdir().unwrap();
        let memory_system = MemorySystem::new(dir.path()).await.unwrap();

        let memory = Memory::new("Rust is memory-safe", MemoryType::Fact);
        memory_system.save(&memory).await.unwrap();

        let results = memory_system.search("memory-safe").await.unwrap();
        assert!(results.iter().any(|r| r.memory.id == memory.id));
    }
}
