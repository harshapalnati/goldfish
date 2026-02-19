//! Comprehensive example demonstrating all Goldfish features
//!
//! This example shows:
//! - Pluggable storage backends
//! - Hybrid retrieval (BM25 + vector + recency + importance + graph)
//! - Working memory
//! - Episodic experiences
//! - Evaluation harness
//!
//! Run: cargo run --example comprehensive

use goldfish::{
    print_results, run_standard_eval, EvalHarness, Experience, HybridSearchConfig, Memory,
    MemoryCortex, MemorySystem, MemoryType, StorageBackend,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🐠 Goldfish Memory System - Comprehensive Demo\n");
    println!("================================================\n");

    // Phase 1: Initialize with SQLite backend
    println!("📦 Phase 1: Initialize Storage Backend (SQLite)");
    let memory = MemorySystem::new("./goldfish_data").await?;
    let cortex = MemoryCortex::new("./goldfish_cortex").await?;
    println!("✅ Storage backends initialized\n");

    // Phase 2: Store diverse memories
    println!("📝 Phase 2: Store Memories");

    // Facts
    cortex
        .remember(&Memory::new(
            "Rust is a systems programming language with zero-cost abstractions",
            MemoryType::Fact,
        ))
        .await?;

    cortex
        .remember(&Memory::new(
            "Python is dynamically typed and great for rapid prototyping",
            MemoryType::Fact,
        ))
        .await?;

    cortex
        .remember(&Memory::new(
            "PostgreSQL supports JSONB for flexible schema design",
            MemoryType::Fact,
        ))
        .await?;

    // Preferences
    cortex
        .prefer("User prefers dark mode in all applications", 0.9)
        .await?;
    cortex
        .prefer("User likes concise, direct answers", 0.85)
        .await?;
    cortex
        .prefer("User prefers Rust over Python for systems code", 0.8)
        .await?;

    // Goals
    cortex
        .goal("Build a production-ready memory system")
        .await?;
    cortex.goal("Learn advanced Rust patterns").await?;

    println!("✅ Stored facts, preferences, and goals\n");

    // Phase 3: Working Memory & Experiences
    println!("🧠 Phase 3: Working Memory & Experiences");

    // Start an experience
    cortex
        .start_episode("Coding Session", "Implementing hybrid retrieval")
        .await?;

    // Think about specific memories (adds to working memory)
    cortex.think_about("rust").await?;
    cortex.think_about("memory").await?;

    // Show working memory
    let context = cortex.get_context().await;
    println!("Working Memory ({} items):", context.len());
    for item in context.iter().take(3) {
        println!(
            "  • {}... (attention: {:.2})",
            &item.content[..item.content.len().min(40)],
            item.attention_score
        );
    }

    cortex.end_episode().await?;
    println!("✅ Experience captured\n");

    // Phase 4: Hybrid Retrieval
    println!("🔍 Phase 4: Hybrid Retrieval");

    let queries = vec!["programming", "user preferences", "database"];

    for query in queries {
        println!("\n  Query: '{}'", query);
        let results = cortex.recall(query, 3).await?;

        for (i, result) in results.iter().enumerate() {
            println!(
                "    {}. {} [{}] (score: {:.3})",
                i + 1,
                &result.memory.content[..result.memory.content.len().min(45)],
                result.memory.memory_type,
                result.score
            );
        }
    }
    println!();

    // Phase 5: Important Memories
    println!("⭐ Phase 5: Most Important Memories");
    let important = cortex.get_important(5).await?;
    for m in &important {
        println!(
            "  • [{}] {} (imp: {:.2})",
            m.memory_type,
            &m.content[..m.content.len().min(50)],
            m.importance
        );
    }
    println!();

    // Phase 6: Evaluation
    println!("📊 Phase 6: Evaluation Harness");
    println!("Running baseline comparisons...\n");

    // Create eval harness
    let backend = memory.store().clone();
    let mut harness = EvalHarness::new(backend);

    // Add test cases
    harness.add_test_case(
        "rust",
        vec![], // Would populate with actual IDs in real test
        "Find Rust-related memories",
    );

    harness.add_test_case("preferences", vec![], "Find user preferences");

    // Run comparison
    let results = harness.compare_baselines().await?;
    print_results(&results);

    // Phase 7: Full Context for LLM
    println!("📝 Phase 7: LLM Context Window");
    println!("------------------------------------------");
    let llm_context = cortex.get_full_context(8).await?;
    println!("{}", llm_context);
    println!("------------------------------------------\n");

    println!("================================================");
    println!("✅ All features working successfully!");
    println!("\n🐠 Goldfish provides:");
    println!("  • Pluggable StorageBackends (SQLite ✓, Postgres, MongoDB)");
    println!("  • Pluggable VectorBackends (LanceDB ✓, pgvector, Qdrant)");
    println!("  • Hybrid Retrieval (BM25 + vector + recency + importance + graph)");
    println!("  • Working Memory (active context)");
    println!("  • Experiences (grouped memories)");
    println!("  • Evaluation Harness (benchmarks & baselines)");
    println!("  • Importance Scoring (dynamic relevance)");
    println!("\nWeights:");
    println!("  • BM25: 0.35");
    println!("  • Vector: 0.35");
    println!("  • Recency: 0.20");
    println!("  • Importance: 0.10");
    println!("  • Graph: 0.15 (bonus)");

    Ok(())
}
