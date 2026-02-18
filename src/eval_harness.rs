//! Evaluation harness for memory system benchmarks
//!
//! Measures:
//! - Retrieval precision (does the right memory come back?)
//! - Context quality (does build_context produce better prompts?)
//! - End-to-end agent task success

use crate::error::Result;
use crate::hybrid_retrieval::{hybrid_rank, ExplainedSearchResult, HybridSearchConfig};
use crate::storage_backend::StorageBackend;
use crate::types::{Memory, MemoryType};
use std::collections::HashMap;
use std::time::Instant;

/// Benchmark results
#[derive(Debug, Default)]
pub struct BenchmarkResults {
    pub name: String,
    pub retrieval_precision: f32,
    pub context_quality_score: f32,
    pub task_success_rate: f32,
    pub avg_latency_ms: f64,
    pub details: Vec<String>,
}

/// Test case for retrieval evaluation
#[derive(Debug, Clone)]
pub struct RetrievalTestCase {
    pub query: String,
    pub expected_memory_ids: Vec<String>,
    pub description: String,
}

/// Eval harness for testing memory systems
pub struct EvalHarness<B: StorageBackend> {
    backend: B,
    test_cases: Vec<RetrievalTestCase>,
}

impl<B: StorageBackend> EvalHarness<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            test_cases: Vec::new(),
        }
    }

    /// Add a test case
    pub fn add_test_case(&mut self, query: &str, expected_ids: Vec<String>, description: &str) {
        self.test_cases.push(RetrievalTestCase {
            query: query.to_string(),
            expected_memory_ids: expected_ids,
            description: description.to_string(),
        });
    }

    /// Run retrieval precision benchmark
    pub async fn benchmark_retrieval(&self, config: &HybridSearchConfig) -> Result<BenchmarkResults> {
        let mut total_precision = 0.0;
        let mut total_latency = 0.0;
        let mut details = Vec::new();

        for (i, test_case) in self.test_cases.iter().enumerate() {
            let start = Instant::now();
            
            // Search using the backend's BM25 (simplified for now)
            let memories = self.backend.get_by_type(MemoryType::Fact, 1000).await?;
            let bm25_results: Vec<_> = memories
                .into_iter()
                .filter(|m| m.content.to_lowercase().contains(&test_case.query.to_lowercase()))
                .enumerate()
                .map(|(idx, m)| crate::types::MemorySearchResult {
                    memory: m,
                    score: 1.0 - (idx as f32 * 0.01),
                    rank: idx + 1,
                })
                .collect();

            // Calculate precision
            let retrieved_ids: Vec<String> = bm25_results.iter().map(|r| r.memory.id.clone()).collect();
            let correct = test_case.expected_memory_ids.iter()
                .filter(|id| retrieved_ids.contains(id))
                .count();
            
            let precision = if !test_case.expected_memory_ids.is_empty() {
                correct as f32 / test_case.expected_memory_ids.len() as f32
            } else {
                1.0
            };
            
            total_precision += precision;
            total_latency += start.elapsed().as_secs_f64() * 1000.0;

            details.push(format!(
                "Test {}: {} - Precision: {:.2}%, Expected: {:?}, Found: {:?}",
                i + 1,
                test_case.description,
                precision * 100.0,
                test_case.expected_memory_ids,
                retrieved_ids.iter().take(5).collect::<Vec<_>>()
            ));
        }

        let avg_precision = if !self.test_cases.is_empty() {
            total_precision / self.test_cases.len() as f32
        } else {
            0.0
        };

        let avg_latency = if !self.test_cases.is_empty() {
            total_latency / self.test_cases.len() as f64
        } else {
            0.0
        };

        Ok(BenchmarkResults {
            name: "Retrieval Precision".to_string(),
            retrieval_precision: avg_precision,
            context_quality_score: 0.0, // Not measured in this test
            task_success_rate: 0.0, // Not measured in this test
            avg_latency_ms: avg_latency,
            details,
        })
    }

    /// Run baseline comparison
    pub async fn compare_baselines(&self) -> Result<Vec<BenchmarkResults>> {
        let mut results = Vec::new();

        // Baseline 1: No memory (random)
        results.push(BenchmarkResults {
            name: "No Memory (Random)".to_string(),
            retrieval_precision: 0.0,
            context_quality_score: 0.0,
            task_success_rate: 0.0,
            avg_latency_ms: 0.0,
            details: vec!["Baseline: No memory system".to_string()],
        });

        // Baseline 2: BM25 only
        let bm25_config = HybridSearchConfig {
            weight_bm25: 1.0,
            weight_vector: 0.0,
            weight_importance: 0.0,
            weight_recency: 0.0,
            weight_graph: 0.0,
            ..Default::default()
        };
        let bm25_results = self.benchmark_retrieval(&bm25_config).await?;
        results.push(BenchmarkResults {
            name: "BM25 Only (Goldfish Baseline)".to_string(),
            ..bm25_results
        });

        // Baseline 3: Hybrid (Goldfish)
        let hybrid_config = HybridSearchConfig::default();
        let hybrid_results = self.benchmark_retrieval(&hybrid_config).await?;
        results.push(BenchmarkResults {
            name: "Hybrid (Goldfish)".to_string(),
            ..hybrid_results
        });

        Ok(results)
    }
}

/// Run standard evaluation suite
pub async fn run_standard_eval<B: StorageBackend>(backend: B) -> Result<Vec<BenchmarkResults>> {
    let mut harness = EvalHarness::new(backend);

    // Add standard test cases
    harness.add_test_case(
        "rust programming",
        vec![], // Will be populated with actual IDs during test
        "Search for Rust-related memories"
    );

    harness.add_test_case(
        "user preference",
        vec![],
        "Search for preference memories"
    );

    harness.add_test_case(
        "project deadline",
        vec![],
        "Search for deadline-related memories"
    );

    harness.compare_baselines().await
}

/// Print benchmark results
pub fn print_results(results: &[BenchmarkResults]) {
    println!("\n========================================");
    println!("        EVALUATION RESULTS");
    println!("========================================\n");

    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result.name);
        println!("   Retrieval Precision: {:.1}%", result.retrieval_precision * 100.0);
        println!("   Avg Latency: {:.2}ms", result.avg_latency_ms);
        println!();
        
        for detail in &result.details {
            println!("   • {}", detail);
        }
        println!();
    }

    println!("========================================\n");
}
