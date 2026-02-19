//! Publishable Proof-of-Concept retrieval benchmark runner.
//!
//! Produces JSON and Markdown reports with:
//! - Recall@1/3/5
//! - MRR
//! - nDCG@k
//! - avg/p95 latency
//! - optional baseline vs tuned sweep
//!
//! Run:
//!   cargo run --example benchmark_poc
//!   cargo run --example benchmark_poc --features lancedb -- --vector-backend lancedb --sweep

use anyhow::{bail, Context, Result};
use clap::Parser;
use goldfish::{
    aggregate_metrics, evaluate_query, BenchmarkQuery, Memory, MemoryCortex, MemoryType,
    RecallWeights, RetrievalMetrics,
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Parser)]
struct Args {
    /// Vector backend to request: auto, file, lancedb.
    #[arg(long, default_value = "auto", value_parser = ["auto", "file", "lancedb"])]
    vector_backend: String,

    /// Run baseline-vs-tuned sweep and include both in one report.
    #[arg(long, default_value_t = false)]
    sweep: bool,

    /// Number of benchmark runs (after warmup).
    #[arg(long, default_value_t = 3)]
    runs: usize,

    /// Number of warmup queries before each timed profile.
    #[arg(long, default_value_t = 20)]
    warmup_queries: usize,

    /// Number of memories generated per topic.
    #[arg(long, default_value_t = 300)]
    memories_per_topic: usize,

    /// Number of queries generated per topic.
    #[arg(long, default_value_t = 30)]
    queries_per_topic: usize,

    /// Number of relevant memories per query.
    #[arg(long, default_value_t = 25)]
    relevant_per_query: usize,

    /// Top-k retrieval depth.
    #[arg(long, default_value_t = 10)]
    top_k: usize,

    /// nDCG cutoff.
    #[arg(long, default_value_t = 10)]
    ndcg_k: usize,

    /// Custom recall weight: text overlap.
    #[arg(long, default_value_t = 0.25)]
    weight_text: f32,

    /// Custom recall weight: importance signal.
    #[arg(long, default_value_t = 0.25)]
    weight_importance: f32,

    /// Custom recall weight: vector similarity.
    #[arg(long, default_value_t = 0.50)]
    weight_vector: f32,

    /// LanceDB ANN index kind.
    #[arg(long, default_value = "ivfpq", value_parser = ["ivfpq", "ivfflat", "off"])]
    lancedb_ann_kind: String,

    /// LanceDB ANN nprobes.
    #[arg(long, default_value_t = 24)]
    lancedb_nprobes: usize,

    /// LanceDB ANN refine factor.
    #[arg(long, default_value_t = 2)]
    lancedb_refine_factor: u32,

    /// Minimum rows before creating ANN index.
    #[arg(long, default_value_t = 256)]
    lancedb_ann_min_rows: usize,

    /// Benchmark name prefix for output files.
    #[arg(long, default_value = "poc_benchmark")]
    name: String,

    /// Benchmark data directory used by MemoryCortex.
    #[arg(long, default_value = "./benchmark_cortex_data/poc")]
    data_dir: PathBuf,

    /// Folder for result reports.
    #[arg(long, default_value = "benchmark_suites/results")]
    results_dir: PathBuf,

    /// Folder for generated dataset export.
    #[arg(long, default_value = "benchmark_suites/datasets/generated")]
    datasets_dir: PathBuf,

    /// Export generated dataset JSONL files.
    #[arg(long, default_value_t = true)]
    export_dataset: bool,

    /// Remove old benchmark data before running.
    #[arg(long, default_value_t = true)]
    reset_data: bool,
}

#[derive(Debug, Clone)]
struct Topic {
    slug: &'static str,
    noun: &'static str,
    verb: &'static str,
    keyword: &'static str,
}

#[derive(Debug, Clone)]
struct Dataset {
    memories: Vec<Memory>,
    queries: Vec<BenchmarkQuery>,
    topic_count: usize,
}

#[derive(Debug, Serialize)]
struct RunReport {
    run_index: usize,
    metrics: RetrievalMetrics,
}

#[derive(Debug, Serialize)]
struct ProfileReport {
    name: String,
    weights: RecallWeights,
    runs: Vec<RunReport>,
    aggregate: RetrievalMetrics,
}

#[derive(Debug, Serialize)]
struct ComparisonSummary {
    baseline: String,
    candidate: String,
    delta_recall_at_1: f32,
    delta_recall_at_3: f32,
    delta_recall_at_5: f32,
    delta_mrr: f32,
    delta_ndcg_at_k: f32,
    delta_avg_latency_ms: f64,
    delta_p95_latency_ms: f64,
}

#[derive(Debug, Serialize)]
struct PocReport {
    suite_name: String,
    generated_at_utc: String,
    backend: String,
    command: String,
    config: ConfigSummary,
    dataset: DatasetSummary,
    ingestion_time_ms: f64,
    profiles: Vec<ProfileReport>,
    comparison: Option<ComparisonSummary>,
}

#[derive(Debug, Serialize)]
struct ConfigSummary {
    runs: usize,
    warmup_queries: usize,
    top_k: usize,
    ndcg_k: usize,
    vector_backend_requested: String,
    sweep: bool,
}

#[derive(Debug, Serialize)]
struct DatasetSummary {
    topics: usize,
    memories_total: usize,
    queries_total: usize,
    memories_per_topic: usize,
    queries_per_topic: usize,
    relevant_per_query: usize,
}

#[derive(Debug, Serialize)]
struct DatasetMemoryRow {
    id: String,
    content: String,
    memory_type: String,
    importance: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.vector_backend != "auto" {
        std::env::set_var("GOLDFISH_VECTOR_BACKEND", args.vector_backend.clone());
    }

    // Apply ANN tuning knobs for LanceDB path (used when feature + backend are active).
    std::env::set_var("GOLDFISH_LANCEDB_ANN", args.lancedb_ann_kind.clone());
    std::env::set_var("GOLDFISH_LANCEDB_NPROBES", args.lancedb_nprobes.to_string());
    std::env::set_var(
        "GOLDFISH_LANCEDB_REFINE_FACTOR",
        args.lancedb_refine_factor.to_string(),
    );
    std::env::set_var(
        "GOLDFISH_LANCEDB_ANN_MIN_ROWS",
        args.lancedb_ann_min_rows.to_string(),
    );

    if args.reset_data && args.data_dir.exists() {
        fs::remove_dir_all(&args.data_dir)
            .with_context(|| format!("failed to clean '{}'", args.data_dir.display()))?;
    }

    let dataset = build_dataset(
        args.memories_per_topic,
        args.queries_per_topic,
        args.relevant_per_query,
    );

    let cortex = MemoryCortex::new(&args.data_dir).await?;
    let backend = cortex.vector_backend_name().to_string();
    if args.vector_backend == "lancedb" && backend != "lancedb" {
        bail!(
            "requested lancedb backend, but active backend is '{}'. Run with --features lancedb.",
            backend
        );
    }

    let ingest_start = Instant::now();
    for memory in &dataset.memories {
        cortex.remember(memory).await?;
    }
    let ingestion_time_ms = ingest_start.elapsed().as_secs_f64() * 1000.0;

    let profiles_to_run = if args.sweep {
        vec![
            (
                "baseline".to_string(),
                RecallWeights::default(),
            ),
            (
                "tuned".to_string(),
                RecallWeights {
                    text: 0.38,
                    importance: 0.12,
                    vector: 0.50,
                }
                .normalized(),
            ),
        ]
    } else {
        vec![(
            "custom".to_string(),
            RecallWeights {
                text: args.weight_text,
                importance: args.weight_importance,
                vector: args.weight_vector,
            }
            .normalized(),
        )]
    };

    let mut profile_reports = Vec::new();
    for (profile_name, weights) in &profiles_to_run {
        let profile = run_profile(
            &cortex,
            &dataset,
            profile_name,
            *weights,
            args.runs,
            args.warmup_queries,
            args.top_k,
            args.ndcg_k,
        )
        .await?;
        profile_reports.push(profile);
    }

    let comparison = if profile_reports.len() >= 2 {
        let baseline = &profile_reports[0];
        let candidate = &profile_reports[1];
        Some(ComparisonSummary {
            baseline: baseline.name.clone(),
            candidate: candidate.name.clone(),
            delta_recall_at_1: candidate.aggregate.recall_at_1 - baseline.aggregate.recall_at_1,
            delta_recall_at_3: candidate.aggregate.recall_at_3 - baseline.aggregate.recall_at_3,
            delta_recall_at_5: candidate.aggregate.recall_at_5 - baseline.aggregate.recall_at_5,
            delta_mrr: candidate.aggregate.mrr - baseline.aggregate.mrr,
            delta_ndcg_at_k: candidate.aggregate.ndcg_at_k - baseline.aggregate.ndcg_at_k,
            delta_avg_latency_ms: candidate.aggregate.avg_latency_ms - baseline.aggregate.avg_latency_ms,
            delta_p95_latency_ms: candidate.aggregate.p95_latency_ms - baseline.aggregate.p95_latency_ms,
        })
    } else {
        None
    };

    let report = PocReport {
        suite_name: "Goldfish PoC Retrieval Benchmark".to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339(),
        backend: backend.clone(),
        command: std::env::args().collect::<Vec<_>>().join(" "),
        config: ConfigSummary {
            runs: args.runs,
            warmup_queries: args.warmup_queries,
            top_k: args.top_k,
            ndcg_k: args.ndcg_k,
            vector_backend_requested: args.vector_backend.clone(),
            sweep: args.sweep,
        },
        dataset: DatasetSummary {
            topics: dataset.topic_count,
            memories_total: dataset.memories.len(),
            queries_total: dataset.queries.len(),
            memories_per_topic: args.memories_per_topic,
            queries_per_topic: args.queries_per_topic,
            relevant_per_query: args.relevant_per_query,
        },
        ingestion_time_ms,
        profiles: profile_reports,
        comparison,
    };

    fs::create_dir_all(&args.results_dir)
        .with_context(|| format!("failed creating '{}'", args.results_dir.display()))?;
    if args.export_dataset {
        fs::create_dir_all(&args.datasets_dir)
            .with_context(|| format!("failed creating '{}'", args.datasets_dir.display()))?;
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let base = format!("{}_{}_{}", args.name, backend, ts);
    let json_path = args.results_dir.join(format!("{base}.json"));
    let md_path = args.results_dir.join(format!("{base}.md"));

    fs::write(&json_path, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed writing '{}'", json_path.display()))?;
    fs::write(&md_path, render_markdown(&report))
        .with_context(|| format!("failed writing '{}'", md_path.display()))?;

    if args.export_dataset {
        let mem_path = args.datasets_dir.join(format!("{base}_memories.jsonl"));
        let qry_path = args.datasets_dir.join(format!("{base}_queries.jsonl"));
        export_dataset_jsonl(&dataset, mem_path, qry_path)?;
    }

    println!("PoC benchmark completed.");
    println!("Backend: {}", report.backend);
    println!("Memories: {}", report.dataset.memories_total);
    println!("Queries: {}", report.dataset.queries_total);
    println!("JSON report: {}", json_path.display());
    println!("Markdown report: {}", md_path.display());
    for profile in &report.profiles {
        println!(
            "[{}] Recall@1 {:.4} | Recall@3 {:.4} | Recall@5 {:.4} | MRR {:.4} | nDCG@{} {:.4} | avg {:.4} ms | p95 {:.4} ms",
            profile.name,
            profile.aggregate.recall_at_1,
            profile.aggregate.recall_at_3,
            profile.aggregate.recall_at_5,
            profile.aggregate.mrr,
            args.ndcg_k,
            profile.aggregate.ndcg_at_k,
            profile.aggregate.avg_latency_ms,
            profile.aggregate.p95_latency_ms
        );
    }

    Ok(())
}

async fn run_profile(
    cortex: &MemoryCortex,
    dataset: &Dataset,
    profile_name: &str,
    weights: RecallWeights,
    runs: usize,
    warmup_queries: usize,
    top_k: usize,
    ndcg_k: usize,
) -> Result<ProfileReport> {
    cortex.set_recall_weights(weights).await;

    for q in dataset.queries.iter().take(warmup_queries) {
        let _ = cortex.recall(&q.query, top_k).await?;
    }

    let mut run_reports = Vec::new();
    for run_idx in 0..runs {
        let mut per_query = Vec::new();
        for q in &dataset.queries {
            let start = Instant::now();
            let hits = cortex.recall(&q.query, top_k).await?;
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let retrieved_ids = hits.into_iter().map(|h| h.memory.id).collect::<Vec<_>>();
            per_query.push(evaluate_query(
                q.query_id.clone(),
                retrieved_ids,
                q.relevance_map(),
                latency_ms,
                ndcg_k,
            ));
        }

        run_reports.push(RunReport {
            run_index: run_idx + 1,
            metrics: aggregate_metrics(&per_query),
        });
    }

    let aggregate = average_run_metrics(&run_reports);
    Ok(ProfileReport {
        name: profile_name.to_string(),
        weights,
        runs: run_reports,
        aggregate,
    })
}

fn build_dataset(
    memories_per_topic: usize,
    queries_per_topic: usize,
    relevant_per_query: usize,
) -> Dataset {
    let topics = vec![
        Topic {
            slug: "rust",
            noun: "systems programming",
            verb: "optimize",
            keyword: "ownership",
        },
        Topic {
            slug: "python",
            noun: "data science",
            verb: "prototype",
            keyword: "notebook",
        },
        Topic {
            slug: "database",
            noun: "query performance",
            verb: "index",
            keyword: "sql",
        },
        Topic {
            slug: "vector",
            noun: "embedding retrieval",
            verb: "rank",
            keyword: "similarity",
        },
        Topic {
            slug: "agents",
            noun: "task planning",
            verb: "coordinate",
            keyword: "memory",
        },
        Topic {
            slug: "security",
            noun: "access control",
            verb: "harden",
            keyword: "audit",
        },
        Topic {
            slug: "ops",
            noun: "deployment reliability",
            verb: "monitor",
            keyword: "latency",
        },
        Topic {
            slug: "testing",
            noun: "evaluation quality",
            verb: "validate",
            keyword: "benchmark",
        },
    ];

    let detail_tokens = [
        "baseline", "regression", "pipeline", "release", "incident", "workflow", "optimizer",
        "adapter", "connector", "signal",
    ];
    let mut memories = Vec::new();
    let mut topic_ids: HashMap<&str, Vec<String>> = HashMap::new();

    for topic in &topics {
        let mut ids = Vec::with_capacity(memories_per_topic);
        for i in 0..memories_per_topic {
            let token = detail_tokens[i % detail_tokens.len()];
            let mut memory = Memory::new(
                format!(
                    "{} memo {}: {} with {} focuses on {}. Key token: {}.",
                    topic.slug,
                    i + 1,
                    topic.noun,
                    topic.keyword,
                    topic.verb,
                    token
                ),
                match i % 5 {
                    0 => MemoryType::Fact,
                    1 => MemoryType::Goal,
                    2 => MemoryType::Preference,
                    3 => MemoryType::Observation,
                    _ => MemoryType::Todo,
                },
            );
            memory.id = format!("{}_m_{:04}", topic.slug, i);
            memory.importance = if i < 5 { 0.95 } else { 0.65 };
            ids.push(memory.id.clone());
            memories.push(memory);
        }
        topic_ids.insert(topic.slug, ids);
    }

    let mut queries = Vec::new();
    for topic in &topics {
        let ids = topic_ids.get(topic.slug).cloned().unwrap_or_default();
        for i in 0..queries_per_topic {
            let q = match i % 6 {
                0 => format!("{} {} best practices", topic.slug, topic.keyword),
                1 => format!("how to {} {}", topic.verb, topic.slug),
                2 => format!("{} tuning for {}", topic.keyword, topic.slug),
                3 => format!("{} workflow with {}", topic.noun, topic.slug),
                4 => format!("agent memory for {}", topic.slug),
                _ => format!("{} {} production checklist", topic.slug, topic.verb),
            };

            let mut relevance = HashMap::new();
            for (idx, id) in ids.iter().take(relevant_per_query).enumerate() {
                let grade = if idx < 3 {
                    3
                } else if idx < 10 {
                    2
                } else {
                    1
                };
                relevance.insert(id.clone(), grade);
            }

            queries.push(BenchmarkQuery {
                query_id: format!("q_{}_{}", topic.slug, i),
                query: q,
                relevant_ids: relevance.keys().cloned().collect(),
                relevance,
            });
        }
    }

    Dataset {
        memories,
        queries,
        topic_count: topics.len(),
    }
}

fn average_run_metrics(runs: &[RunReport]) -> RetrievalMetrics {
    if runs.is_empty() {
        return RetrievalMetrics::default();
    }
    let n = runs.len() as f32;
    let nf = runs.len() as f64;
    RetrievalMetrics {
        evaluated_queries: runs
            .iter()
            .map(|r| r.metrics.evaluated_queries)
            .sum::<usize>()
            / runs.len(),
        recall_at_1: runs.iter().map(|r| r.metrics.recall_at_1).sum::<f32>() / n,
        recall_at_3: runs.iter().map(|r| r.metrics.recall_at_3).sum::<f32>() / n,
        recall_at_5: runs.iter().map(|r| r.metrics.recall_at_5).sum::<f32>() / n,
        mrr: runs.iter().map(|r| r.metrics.mrr).sum::<f32>() / n,
        ndcg_at_k: runs.iter().map(|r| r.metrics.ndcg_at_k).sum::<f32>() / n,
        avg_latency_ms: runs.iter().map(|r| r.metrics.avg_latency_ms).sum::<f64>() / nf,
        p95_latency_ms: runs.iter().map(|r| r.metrics.p95_latency_ms).sum::<f64>() / nf,
    }
}

fn export_dataset_jsonl(dataset: &Dataset, memories_path: PathBuf, queries_path: PathBuf) -> Result<()> {
    let memory_rows: Vec<DatasetMemoryRow> = dataset
        .memories
        .iter()
        .map(|m| DatasetMemoryRow {
            id: m.id.clone(),
            content: m.content.clone(),
            memory_type: m.memory_type.to_string(),
            importance: m.importance,
        })
        .collect();

    let memories_body = memory_rows
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    let queries_body = dataset
        .queries
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    fs::write(&memories_path, format!("{memories_body}\n"))
        .with_context(|| format!("failed writing '{}'", memories_path.display()))?;
    fs::write(&queries_path, format!("{queries_body}\n"))
        .with_context(|| format!("failed writing '{}'", queries_path.display()))?;
    Ok(())
}

fn render_markdown(report: &PocReport) -> String {
    let mut out = String::new();
    out.push_str("# Goldfish Benchmark Report\n\n");
    out.push_str(&format!(
        "- Generated: `{}`\n- Backend: `{}`\n- Suite: `{}`\n\n",
        report.generated_at_utc, report.backend, report.suite_name
    ));
    out.push_str("## Dataset\n\n");
    out.push_str(&format!(
        "- Topics: {}\n- Memories: {}\n- Queries: {}\n- Ingestion time: {:.2} ms\n\n",
        report.dataset.topics,
        report.dataset.memories_total,
        report.dataset.queries_total,
        report.ingestion_time_ms
    ));

    out.push_str("## Profiles\n\n");
    out.push_str("| Profile | w_text | w_importance | w_vector | Recall@1 | Recall@3 | Recall@5 | MRR | nDCG | Avg Latency (ms) | P95 Latency (ms) |\n");
    out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for profile in &report.profiles {
        out.push_str(&format!(
            "| {} | {:.3} | {:.3} | {:.3} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} |\n",
            profile.name,
            profile.weights.text,
            profile.weights.importance,
            profile.weights.vector,
            profile.aggregate.recall_at_1,
            profile.aggregate.recall_at_3,
            profile.aggregate.recall_at_5,
            profile.aggregate.mrr,
            profile.aggregate.ndcg_at_k,
            profile.aggregate.avg_latency_ms,
            profile.aggregate.p95_latency_ms
        ));
    }
    out.push('\n');

    if let Some(cmp) = &report.comparison {
        out.push_str("## Sweep Delta\n\n");
        out.push_str(&format!(
            "Compared `{}` -> `{}`\n\n",
            cmp.baseline, cmp.candidate
        ));
        out.push_str("| Metric | Delta |\n");
        out.push_str("|---|---:|\n");
        out.push_str(&format!("| Recall@1 | {:.4} |\n", cmp.delta_recall_at_1));
        out.push_str(&format!("| Recall@3 | {:.4} |\n", cmp.delta_recall_at_3));
        out.push_str(&format!("| Recall@5 | {:.4} |\n", cmp.delta_recall_at_5));
        out.push_str(&format!("| MRR | {:.4} |\n", cmp.delta_mrr));
        out.push_str(&format!("| nDCG | {:.4} |\n", cmp.delta_ndcg_at_k));
        out.push_str(&format!(
            "| Avg Latency (ms) | {:.4} |\n",
            cmp.delta_avg_latency_ms
        ));
        out.push_str(&format!(
            "| P95 Latency (ms) | {:.4} |\n\n",
            cmp.delta_p95_latency_ms
        ));
    }

    out.push_str("## Repro Command\n\n");
    out.push_str("```bash\n");
    out.push_str(&report.command);
    out.push_str("\n```\n");
    out
}
