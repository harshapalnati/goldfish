//! Vector Search Demo - Shows semantic similarity search
//!
//! Run: cargo run --example vector_search_demo

use goldfish::{MemoryCortex, Memory, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Vector Search Demo\n");
    println!("====================\n");

    // Create cortex with vector search enabled
    let cortex = MemoryCortex::new("./vector_demo_data").await?;
    println!("✅ Cortex initialized with vector search\n");

    // Store memories with semantic content
    println!("📝 Storing memories...");
    
    let memories = vec![
        ("Rust is a systems programming language", MemoryType::Fact),
        ("Python is great for machine learning", MemoryType::Fact),
        ("JavaScript runs in the browser", MemoryType::Fact),
        ("I love coding in Rust", MemoryType::Preference),
        ("User prefers dark mode", MemoryType::Preference),
        ( "The weather is sunny today", MemoryType::Fact),
        ("Need to learn more about async programming", MemoryType::Goal),
    ];

    for (content, mem_type) in &memories {
        cortex.remember(&Memory::new(content.to_string(), *mem_type)).await?;
    }
    println!("   ✅ Stored {} memories\n", memories.len());

    // Search with different queries to show semantic similarity
    let queries = vec![
        "programming languages",
        "coding",
        "user preferences",
        "weather",
    ];

    for query in queries {
        println!("🔎 Query: '{}'", query);
        let results = cortex.recall(query, 3).await?;
        
        for (i, result) in results.iter().enumerate() {
            println!("   {}. {} [{}] (score: {:.3})",
                i + 1,
                result.memory.content,
                result.memory.memory_type,
                result.score
            );
        }
        println!();
    }

    println!("====================");
    println!("✅ Demo complete!");
    println!("\nNote: Using simple hash-based embeddings for demo.");
    println!("Production: Use real embedding models (OpenAI, sentence-transformers, etc.)");

    Ok(())
}
