use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
use crate::types::{Memory, MemorySearchResult, MemoryType};
use crate::vector_backend::VectorBackend;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    pub max_results: usize,
    pub bm25_limit: usize,
    pub vector_limit: usize,
    pub neighbor_depth: u32,

    pub weight_bm25: f32,
    pub weight_vector: f32,
    pub weight_importance: f32,
    pub weight_recency: f32,
    pub weight_graph: f32,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            bm25_limit: 25,
            vector_limit: 25,
            neighbor_depth: 1,
            weight_bm25: 0.35,
            weight_vector: 0.35,
            weight_importance: 0.1,
            weight_recency: 0.2,
            weight_graph: 0.15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrievalExplanation {
    pub bm25: Option<f32>,
    pub vector: Option<f32>,
    pub importance: f32,
    pub recency: f32,
    pub graph: f32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainedSearchResult {
    pub memory: Memory,
    pub score: f32,
    pub rank: usize,
    pub explanation: RetrievalExplanation,
}

#[derive(Debug, Default, Clone)]
struct ScoreParts {
    bm25_raw: Option<f32>,
    vector_raw: Option<f32>,
    graph_raw: f32,
}

fn recency_factor(last_accessed_at: chrono::DateTime<chrono::Utc>) -> f32 {
    let hours_ago = (chrono::Utc::now() - last_accessed_at).num_hours().max(0) as f32;
    1.0 / (1.0 + hours_ago * 0.01)
}

fn normalize_scores(values: &HashMap<String, f32>) -> HashMap<String, f32> {
    if values.is_empty() {
        return HashMap::new();
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for v in values.values() {
        min = min.min(*v);
        max = max.max(*v);
    }
    if (max - min).abs() < f32::EPSILON {
        return values.keys().map(|k| (k.clone(), 1.0)).collect();
    }
    values
        .iter()
        .map(|(k, v)| (k.clone(), (*v - min) / (max - min)))
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn hybrid_rank(
    query: &str,
    bm25_results: Vec<MemorySearchResult>,
    vector_backend: Option<&Arc<dyn VectorBackend>>,
    embedder: Option<&Arc<dyn EmbeddingProvider>>,
    load_memory: impl Fn(
            &str,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Memory>>> + Send>>
        + Send
        + Sync,
    get_neighbors: impl Fn(
            &str,
            u32,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<(Vec<Memory>, Vec<crate::types::Association>)>,
                    > + Send,
            >,
        > + Send
        + Sync,
    cfg: &HybridSearchConfig,
    filter_type: Option<MemoryType>,
) -> Result<Vec<ExplainedSearchResult>> {
    let mut parts: HashMap<String, ScoreParts> = HashMap::new();

    let mut bm25_map: HashMap<String, f32> = HashMap::new();
    for r in bm25_results.into_iter().take(cfg.bm25_limit) {
        if r.memory.forgotten {
            continue;
        }
        if let Some(mt) = filter_type {
            if r.memory.memory_type != mt {
                continue;
            }
        }
        bm25_map.insert(r.memory.id.clone(), r.score);
        parts.entry(r.memory.id.clone()).or_default().bm25_raw = Some(r.score);
    }

    let mut vector_map: HashMap<String, f32> = HashMap::new();
    if let (Some(vb), Some(emb)) = (vector_backend, embedder) {
        let embedded = emb
            .embed(&[query.to_string()])
            .await
            .map_err(|e| MemoryError::VectorDb(format!("Embedding failed: {e}")))?;
        let vec = embedded.first().ok_or_else(|| {
            MemoryError::VectorDb("Embedding provider returned no vectors".into())
        })?;

        let hits = vb.search(vec, cfg.vector_limit).await?;
        for h in hits {
            vector_map.insert(h.id.clone(), h.score);
            parts.entry(h.id).or_default().vector_raw = Some(h.score);
        }
    }

    // Graph expansion: pull neighbors of the strongest base candidates.
    let mut seed_ids: Vec<(String, f32)> = Vec::new();
    for (id, score) in bm25_map.iter() {
        seed_ids.push((id.clone(), *score));
    }
    for (id, score) in vector_map.iter() {
        seed_ids.push((id.clone(), *score));
    }
    seed_ids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut expanded: HashSet<String> = HashSet::new();
    for (seed_id, seed_score) in seed_ids.into_iter().take(10) {
        let (neighbors, assocs) = get_neighbors(&seed_id, cfg.neighbor_depth).await?;

        // Map target ids to relation multipliers.
        let mut rel_mult: HashMap<String, f32> = HashMap::new();
        for a in assocs {
            let other = if a.source_id == seed_id {
                a.target_id
            } else {
                a.source_id
            };
            rel_mult.insert(other, a.relation_type.score_multiplier() as f32);
        }

        for n in neighbors {
            if n.forgotten {
                continue;
            }
            if let Some(mt) = filter_type {
                if n.memory_type != mt {
                    continue;
                }
            }
            if expanded.insert(n.id.clone()) {
                let mult = rel_mult.get(&n.id).copied().unwrap_or(1.0);
                parts.entry(n.id.clone()).or_default().graph_raw += seed_score * mult;
            }
        }
    }

    let bm25_norm = normalize_scores(&bm25_map);
    let vector_norm = normalize_scores(&vector_map);
    let graph_values: HashMap<String, f32> = parts
        .iter()
        .filter(|(_, p)| p.graph_raw > 0.0)
        .map(|(id, p)| (id.clone(), p.graph_raw))
        .collect();
    let graph_norm = normalize_scores(&graph_values);

    let mut scored: Vec<(ExplainedSearchResult, f32)> = Vec::new();
    for (id, p) in parts {
        let Some(memory) = load_memory(&id).await? else {
            continue;
        };
        if memory.forgotten {
            continue;
        }
        if let Some(mt) = filter_type {
            if memory.memory_type != mt {
                continue;
            }
        }

        let bm25 = p.bm25_raw.and_then(|_| bm25_norm.get(&id).copied());
        let vector = p.vector_raw.and_then(|_| vector_norm.get(&id).copied());
        let graph = graph_norm.get(&id).copied().unwrap_or(0.0);

        let importance = memory.importance.clamp(0.0, 1.0);
        let recency = recency_factor(memory.last_accessed_at).clamp(0.0, 1.0);

        let mut explanation = RetrievalExplanation {
            bm25,
            vector,
            importance,
            recency,
            graph,
            notes: Vec::new(),
        };

        if bm25.is_some() {
            explanation
                .notes
                .push("Matched BM25 full-text search".to_string());
        }
        if vector.is_some() {
            explanation
                .notes
                .push("Matched semantic vector search".to_string());
        }
        if graph > 0.0 {
            explanation
                .notes
                .push("Included via graph neighborhood expansion".to_string());
        }

        let score = cfg.weight_bm25 * bm25.unwrap_or(0.0)
            + cfg.weight_vector * vector.unwrap_or(0.0)
            + cfg.weight_importance * importance
            + cfg.weight_recency * recency
            + cfg.weight_graph * graph;

        scored.push((
            ExplainedSearchResult {
                memory,
                score,
                rank: 0,
                explanation,
            },
            score,
        ));
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(cfg.max_results);

    for (i, (r, _)) in scored.iter_mut().enumerate() {
        r.rank = i + 1;
    }

    Ok(scored.into_iter().map(|(r, _)| r).collect())
}
