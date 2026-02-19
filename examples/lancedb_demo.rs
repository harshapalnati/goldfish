//! LanceDB-backed MemoryCortex demo.
//!
//! Run:
//!   cargo run --example lancedb_demo --features lancedb

use goldfish::{Memory, MemoryCortex, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Prefer LanceDB for this process.
    std::env::set_var("GOLDFISH_VECTOR_BACKEND", "lancedb");

    let cortex = MemoryCortex::new("./lancedb_demo_data").await?;
    println!("Vector backend: {}", cortex.vector_backend_name());

    let entries = vec![
        ("User prefers concise answers", MemoryType::Preference),
        ("Project goal is to launch memory search", MemoryType::Goal),
        ("Rust async code uses tokio runtime", MemoryType::Fact),
        ("Fix vector recall quality regression", MemoryType::Todo),
    ];

    for (content, kind) in entries {
        cortex.remember(&Memory::new(content, kind)).await?;
    }

    let results = cortex.recall("concise memory search goal", 3).await?;
    println!("Top recalls:");
    for r in results {
        println!("  - [{:.3}] {}", r.score, r.memory.content);
    }

    Ok(())
}
