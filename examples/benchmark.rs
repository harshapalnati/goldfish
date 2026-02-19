//! Benchmark example - Run retrieval quality tests
//! 
//! This example demonstrates Goldfish's retrieval performance
//! using a realistic personal assistant scenario.
//!
//! Usage: cargo run --example benchmark --release

use goldfish::{Memory, MemoryType, MemoryCortex};
use goldfish::eval_harness::{compare_configurations, create_test_dataset, print_results};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n");
    println!("🐠══════════════════════════════════════════════════════════🐠");
    println!("       GOLDFISH MEMORY RETRIEVAL BENCHMARK");
    println!("🐠══════════════════════════════════════════════════════════🐠\n");
    
    println!("Setting up test environment...\n");
    
    // Initialize cortex
    let cortex = MemoryCortex::new("./benchmark_data").await?;
    
    // Create test memories - 30 personal assistant memories
    let memories = create_test_memories();
    
    println!("Loading {} test memories into cortex...", memories.len());
    for (i, mem) in memories.iter().enumerate() {
        cortex.remember(mem).await?;
        if (i + 1) % 10 == 0 {
            println!("  ✓ Loaded {}/{} memories", i + 1, memories.len());
        }
    }
    println!("  ✓ All {} memories loaded\n", memories.len());
    
    // Create test cases
    let test_cases = create_test_dataset();
    println!("Test dataset: {} queries to evaluate\n", test_cases.len());
    
    // Print test case overview
    println!("Test queries:");
    for (i, tc) in test_cases.iter().enumerate() {
        println!("  {}. \"{}\" - {}", i + 1, tc.query, tc.description);
    }
    println!();
    
    // Run benchmarks
    println!("Running benchmarks (this may take a moment)...\n");
    let results = compare_configurations(&cortex, &test_cases).await?;
    
    // Print results
    print_results(&results);
    
    // Print sample query details
    println!("📋 SAMPLE QUERY DETAILS:\n");
    if let Some(hybrid) = results.iter().find(|r| r.name.contains("Hybrid")) {
        for (i, qr) in hybrid.query_results.iter().take(5).enumerate() {
            println!("Query {}: \"{}\"", i + 1, qr.query);
            println!("  Expected keywords: {}", 
                create_test_dataset()[i].expected_keywords.join(", "));
            println!("  Precision: {:.1}% | Recall: {:.1}% | Latency: {:.1}ms", 
                qr.precision * 100.0, qr.recall * 100.0, qr.latency_ms);
            if !qr.top_results.is_empty() {
                println!("  Top result: {} (score: {:.2})", 
                    qr.top_results[0].0, qr.top_results[0].1);
            }
            println!();
        }
    }
    
    println!("✅ Benchmark complete!\n");
    
    Ok(())
}

/// Create 30 realistic personal assistant memories
fn create_test_memories() -> Vec<Memory> {
    vec![
        // Identity & Facts (5)
        Memory::new("User's name is Alex", MemoryType::Identity),
        Memory::new("User works as a software engineer at a startup", MemoryType::Fact),
        Memory::new("User lives in San Francisco, California", MemoryType::Fact),
        Memory::new("User has a dog named Rusty who is a golden retriever", MemoryType::Fact),
        Memory::new("User speaks English and Spanish fluently", MemoryType::Identity),
        
        // Preferences (10)
        Memory::new("User prefers dark mode in all applications", MemoryType::Preference),
        Memory::new("User likes coffee with oat milk, not dairy", MemoryType::Preference),
        Memory::new("User prefers Slack over email for work communication", MemoryType::Preference),
        Memory::new("User likes hiking on weekends in nearby trails", MemoryType::Preference),
        Memory::new("User prefers minimal UI designs without clutter", MemoryType::Preference),
        Memory::new("User dislikes video calls before 10am", MemoryType::Preference),
        Memory::new("User prefers reading books over watching videos", MemoryType::Preference),
        Memory::new("User likes Thai food, especially pad thai", MemoryType::Preference),
        Memory::new("User prefers async communication over real-time chat", MemoryType::Preference),
        Memory::new("User dislikes notification sounds and keeps phone on silent", MemoryType::Preference),
        
        // Goals (5)
        Memory::new("Goal: Learn Rust programming language this year", MemoryType::Goal)
            .with_importance(0.9),
        Memory::new("Goal: Build a side project and launch it", MemoryType::Goal)
            .with_importance(0.85),
        Memory::new("Goal: Get AWS certification by end of quarter", MemoryType::Goal)
            .with_importance(0.8),
        Memory::new("Goal: Exercise 3 times per week minimum", MemoryType::Goal)
            .with_importance(0.75),
        Memory::new("Goal: Read 20 books this year", MemoryType::Goal)
            .with_importance(0.7),
        
        // Decisions (5)
        Memory::new("Decision: Use SQLite for local storage instead of PostgreSQL", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Switch to MacBook Pro for development work", MemoryType::Decision)
            .with_importance(0.75),
        Memory::new("Decision: Cancel Netflix subscription to save money", MemoryType::Decision)
            .with_importance(0.6),
        Memory::new("Decision: Adopt Docker for all deployment scenarios", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Use Figma for all design work", MemoryType::Decision)
            .with_importance(0.7),
        
        // Events (5)
        Memory::new("Last week: Presented at local tech meetup about Rust", MemoryType::Event),
        Memory::new("Yesterday: Had coffee with mentor to discuss career growth", MemoryType::Event),
        Memory::new("Today: Deployed v1.0 of the project to production", MemoryType::Event),
        Memory::new("Last month: Moved to new apartment in Mission District", MemoryType::Event),
        Memory::new("Two weeks ago: Adopted a golden retriever puppy named Rusty", MemoryType::Event),
    ]
}
