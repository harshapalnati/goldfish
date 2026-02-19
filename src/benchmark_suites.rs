//! Retrieval benchmark helpers inspired by RTEB-style evaluation.
//!
//! Provides reusable metrics:
//! - Recall@1/3/5
//! - MRR
//! - nDCG@k

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuery {
    pub query_id: String,
    pub query: String,
    #[serde(default)]
    pub relevant_ids: Vec<String>,
    #[serde(default)]
    pub relevance: HashMap<String, u32>,
}

impl BenchmarkQuery {
    pub fn relevance_map(&self) -> HashMap<String, u32> {
        let mut map = self.relevance.clone();
        for id in &self.relevant_ids {
            map.entry(id.clone()).or_insert(1);
        }
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryMetrics {
    pub query_id: String,
    pub recall_at_1: f32,
    pub recall_at_3: f32,
    pub recall_at_5: f32,
    pub mrr: f32,
    pub ndcg_at_k: f32,
    pub latency_ms: f64,
    pub retrieved_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrievalMetrics {
    pub evaluated_queries: usize,
    pub recall_at_1: f32,
    pub recall_at_3: f32,
    pub recall_at_5: f32,
    pub mrr: f32,
    pub ndcg_at_k: f32,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub suite_name: String,
    pub generated_at_utc: String,
    pub dataset: String,
    pub backend: String,
    pub top_k: usize,
    pub ndcg_k: usize,
    pub metrics: RetrievalMetrics,
    pub per_query: Vec<QueryMetrics>,
}

pub fn evaluate_query(
    query_id: impl Into<String>,
    retrieved_ids: Vec<String>,
    relevance: HashMap<String, u32>,
    latency_ms: f64,
    ndcg_k: usize,
) -> QueryMetrics {
    let relevant_set: HashSet<&str> = relevance
        .iter()
        .filter(|(_, score)| **score > 0)
        .map(|(id, _)| id.as_str())
        .collect();

    QueryMetrics {
        query_id: query_id.into(),
        recall_at_1: recall_at_k(&retrieved_ids, &relevant_set, 1),
        recall_at_3: recall_at_k(&retrieved_ids, &relevant_set, 3),
        recall_at_5: recall_at_k(&retrieved_ids, &relevant_set, 5),
        mrr: reciprocal_rank(&retrieved_ids, &relevant_set),
        ndcg_at_k: ndcg_at_k(&retrieved_ids, &relevance, ndcg_k),
        latency_ms,
        retrieved_ids,
    }
}

pub fn aggregate_metrics(per_query: &[QueryMetrics]) -> RetrievalMetrics {
    if per_query.is_empty() {
        return RetrievalMetrics::default();
    }

    let n = per_query.len() as f32;
    let mut latencies: Vec<f64> = per_query.iter().map(|m| m.latency_ms).collect();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95_idx = ((latencies.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(latencies.len() - 1);

    RetrievalMetrics {
        evaluated_queries: per_query.len(),
        recall_at_1: per_query.iter().map(|m| m.recall_at_1).sum::<f32>() / n,
        recall_at_3: per_query.iter().map(|m| m.recall_at_3).sum::<f32>() / n,
        recall_at_5: per_query.iter().map(|m| m.recall_at_5).sum::<f32>() / n,
        mrr: per_query.iter().map(|m| m.mrr).sum::<f32>() / n,
        ndcg_at_k: per_query.iter().map(|m| m.ndcg_at_k).sum::<f32>() / n,
        avg_latency_ms: per_query.iter().map(|m| m.latency_ms).sum::<f64>() / per_query.len() as f64,
        p95_latency_ms: latencies[p95_idx],
    }
}

fn recall_at_k(retrieved: &[String], relevant: &HashSet<&str>, k: usize) -> f32 {
    if relevant.is_empty() {
        return 0.0;
    }
    let found = retrieved
        .iter()
        .take(k)
        .filter(|id| relevant.contains(id.as_str()))
        .count();
    found as f32 / relevant.len() as f32
}

fn reciprocal_rank(retrieved: &[String], relevant: &HashSet<&str>) -> f32 {
    for (idx, id) in retrieved.iter().enumerate() {
        if relevant.contains(id.as_str()) {
            return 1.0 / (idx as f32 + 1.0);
        }
    }
    0.0
}

fn ndcg_at_k(retrieved: &[String], relevance: &HashMap<String, u32>, k: usize) -> f32 {
    if relevance.is_empty() || k == 0 {
        return 0.0;
    }

    let dcg = dcg_at_k(retrieved, relevance, k);
    let idcg = ideal_dcg_at_k(relevance, k);
    if idcg <= f32::EPSILON {
        0.0
    } else {
        dcg / idcg
    }
}

fn dcg_at_k(retrieved: &[String], relevance: &HashMap<String, u32>, k: usize) -> f32 {
    retrieved
        .iter()
        .take(k)
        .enumerate()
        .map(|(rank, id)| {
            let rel = *relevance.get(id).unwrap_or(&0) as f32;
            let gain = (2f32.powf(rel) - 1.0).max(0.0);
            let denom = (rank as f32 + 2.0).log2();
            if denom <= f32::EPSILON {
                0.0
            } else {
                gain / denom
            }
        })
        .sum()
}

fn ideal_dcg_at_k(relevance: &HashMap<String, u32>, k: usize) -> f32 {
    let mut grades: Vec<f32> = relevance
        .values()
        .copied()
        .map(|v| v as f32)
        .filter(|v| *v > 0.0)
        .collect();
    grades.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    grades
        .into_iter()
        .take(k)
        .enumerate()
        .map(|(rank, rel)| {
            let gain = (2f32.powf(rel) - 1.0).max(0.0);
            let denom = (rank as f32 + 2.0).log2();
            if denom <= f32::EPSILON {
                0.0
            } else {
                gain / denom
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_perfect_ranking() {
        let retrieved = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("a".to_string(), 3);
        relevance.insert("b".to_string(), 2);
        relevance.insert("c".to_string(), 1);

        let q = evaluate_query("q1", retrieved, relevance, 10.0, 3);
        assert!((q.recall_at_1 - (1.0 / 3.0)).abs() < 0.001);
        assert!((q.recall_at_3 - 1.0).abs() < 0.001);
        assert!((q.recall_at_5 - 1.0).abs() < 0.001);
        assert!((q.mrr - 1.0).abs() < 0.001);
        assert!((q.ndcg_at_k - 1.0).abs() < 0.001);
    }

    #[test]
    fn metrics_no_hits() {
        let retrieved = vec!["x".to_string(), "y".to_string()];
        let mut relevance = HashMap::new();
        relevance.insert("a".to_string(), 1);
        relevance.insert("b".to_string(), 1);

        let q = evaluate_query("q2", retrieved, relevance, 5.0, 5);
        assert_eq!(q.recall_at_1, 0.0);
        assert_eq!(q.recall_at_3, 0.0);
        assert_eq!(q.mrr, 0.0);
        assert_eq!(q.ndcg_at_k, 0.0);
    }
}
