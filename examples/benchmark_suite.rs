//! Benchmark harness for retrieval metrics:
//! Recall@1/3/5, MRR, nDCG@k.
//!
//! Run:
//!   cargo run --example benchmark_suite
//!   cargo run --example benchmark_suite --features lancedb -- --vector-backend lancedb

use anyhow::{Context, Result};
use clap::Parser;
use goldfish::{
    aggregate_metrics, evaluate_query, BenchmarkQuery, BenchmarkReport, Memory, MemoryCortex,
    MemoryType,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Parser)]
struct Args {
    /// JSONL file with benchmark memory records.
    #[arg(long, default_value = "benchmark_suites/datasets/sample_memories.jsonl")]
    memories: PathBuf,

    /// JSONL file with benchmark query + relevance records.
    #[arg(long, default_value = "benchmark_suites/datasets/sample_queries.jsonl")]
    queries: PathBuf,

    /// Output folder for benchmark reports.
    #[arg(long, default_value = "benchmark_suites/results")]
    results_dir: PathBuf,

    /// Optional output file name. If omitted, a timestamped file is used.
    #[arg(long)]
    output: Option<String>,

    /// Benchmark working data directory used by MemoryCortex.
    #[arg(long, default_value = "./benchmark_cortex_data")]
    data_dir: PathBuf,

    /// Number of retrieved memories per query.
    #[arg(long, default_value_t = 10)]
    top_k: usize,

    /// Cutoff for nDCG.
    #[arg(long, default_value_t = 10)]
    ndcg_k: usize,

    /// Vector backend: auto, file, or lancedb.
    #[arg(long, default_value = "auto")]
    vector_backend: String,

    /// Reset benchmark data directory before running.
    #[arg(long, default_value_t = true)]
    reset_data: bool,
}

#[derive(Debug, Deserialize)]
struct BenchmarkMemory {
    id: String,
    content: String,
    memory_type: String,
    importance: Option<f32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.vector_backend != "auto" {
        std::env::set_var("GOLDFISH_VECTOR_BACKEND", args.vector_backend.clone());
    }

    if args.reset_data && args.data_dir.exists() {
        fs::remove_dir_all(&args.data_dir).with_context(|| {
            format!(
                "failed removing benchmark data dir '{}'",
                args.data_dir.display()
            )
        })?;
    }

    let memories: Vec<BenchmarkMemory> = load_jsonl(&args.memories)?;
    let queries: Vec<BenchmarkQuery> = load_jsonl(&args.queries)?;

    let cortex = MemoryCortex::new(&args.data_dir).await?;
    let backend_name = cortex.vector_backend_name().to_string();

    for record in memories {
        let mut m = Memory::new(record.content, parse_memory_type(&record.memory_type)?);
        m.id = record.id;
        if let Some(importance) = record.importance {
            m.importance = importance.clamp(0.0, 1.0);
        }
        cortex.remember(&m).await?;
    }

    let mut per_query = Vec::new();
    for q in &queries {
        let start = Instant::now();
        let hits = cortex.recall(&q.query, args.top_k).await?;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        let retrieved_ids = hits.into_iter().map(|h| h.memory.id).collect::<Vec<_>>();
        per_query.push(evaluate_query(
            q.query_id.clone(),
            retrieved_ids,
            q.relevance_map(),
            latency_ms,
            args.ndcg_k,
        ));
    }

    let metrics = aggregate_metrics(&per_query);

    fs::create_dir_all(&args.results_dir).with_context(|| {
        format!(
            "failed creating results dir '{}'",
            args.results_dir.display()
        )
    })?;

    let output_file = args.output.unwrap_or_else(|| {
        format!(
            "retrieval_benchmark_{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        )
    });
    let output_path = args.results_dir.join(output_file);

    let report = BenchmarkReport {
        suite_name: "RTEB-style retrieval suite".to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339(),
        dataset: args.queries.display().to_string(),
        backend: backend_name.clone(),
        top_k: args.top_k,
        ndcg_k: args.ndcg_k,
        metrics,
        per_query,
    };

    let json = serde_json::to_string_pretty(&report)?;
    fs::write(&output_path, json)
        .with_context(|| format!("failed writing report '{}'", output_path.display()))?;

    println!("Benchmark completed.");
    println!("Vector backend: {}", backend_name);
    println!("Saved report: {}", output_path.display());
    println!(
        "Recall@1 {:.3} | Recall@3 {:.3} | Recall@5 {:.3} | MRR {:.3} | nDCG@{} {:.3}",
        report.metrics.recall_at_1,
        report.metrics.recall_at_3,
        report.metrics.recall_at_5,
        report.metrics.mrr,
        args.ndcg_k,
        report.metrics.ndcg_at_k
    );
    println!(
        "Latency avg {:.3} ms | p95 {:.3} ms",
        report.metrics.avg_latency_ms, report.metrics.p95_latency_ms
    );

    Ok(())
}

fn load_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let file = File::open(path)
        .with_context(|| format!("failed opening JSONL file '{}'", path.display()))?;
    let reader = BufReader::new(file);

    let mut values = Vec::new();
    for (line_idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("failed reading line {}", line_idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parsed = serde_json::from_str::<T>(trimmed).with_context(|| {
            format!(
                "failed parsing JSON at line {} in '{}'",
                line_idx + 1,
                path.display()
            )
        })?;
        values.push(parsed);
    }
    Ok(values)
}

fn parse_memory_type(input: &str) -> Result<MemoryType> {
    let norm = input.trim().to_lowercase();
    let mt = match norm.as_str() {
        "fact" => MemoryType::Fact,
        "preference" => MemoryType::Preference,
        "decision" => MemoryType::Decision,
        "identity" => MemoryType::Identity,
        "event" => MemoryType::Event,
        "observation" => MemoryType::Observation,
        "goal" => MemoryType::Goal,
        "todo" => MemoryType::Todo,
        "summary" => MemoryType::Summary,
        other => {
            anyhow::bail!("unsupported memory_type '{}' in benchmark dataset", other);
        }
    };
    Ok(mt)
}
