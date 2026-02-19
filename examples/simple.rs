//! Simple example demonstrating the basic MemorySystem API.
//! For advanced agent capabilities (Working Memory, Episodic Memory), see `examples/agent.rs`.
//!
//! Run: cargo run --example simple

use goldfish::{Memory, MemorySystem, MemoryType, RelationType, SearchConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧠 Goldfish Memory System - Simple Example\n");
    println!("==========================================\n");

    // Initialize memory system
    println!("Initializing memory system...");
    let memory = MemorySystem::new("./example_data").await?;
    println!("✓ Ready\n");

    // Store some facts
    println!("📝 Storing facts...");
    let fact1 = Memory::new(
        "Rust is a systems programming language focused on safety",
        MemoryType::Fact,
    );
    let fact2 = Memory::new("Python is popular for machine learning", MemoryType::Fact);
    let fact3 = Memory::new(
        "TypeScript adds type safety to JavaScript",
        MemoryType::Fact,
    );

    memory.save(&fact1).await?;
    memory.save(&fact2).await?;
    memory.save(&fact3).await?;
    println!("✓ Stored 3 facts\n");

    // Store preferences
    println!("💭 Storing preferences...");
    let pref = Memory::new("User prefers dark mode in editors", MemoryType::Preference)
        .with_importance(0.9);
    memory.save(&pref).await?;
    println!("✓ Stored 1 preference\n");

    // Store a goal
    println!("🎯 Storing goals...");
    let goal = Memory::new("Learn distributed systems with Rust", MemoryType::Goal);
    memory.save(&goal).await?;
    println!("✓ Stored 1 goal\n");

    // Search for memories
    println!("🔍 Searching for 'programming'...");
    let config = SearchConfig::default();
    let results = memory.search_with_config("programming", &config).await?;

    println!("   Found {} results:", results.len());
    for result in &results {
        println!(
            "   - [{:?}] {} (score: {:.2})",
            result.memory.memory_type, result.memory.content, result.score
        );
    }
    println!();

    // Get memories by type
    println!("📋 All Facts:");
    let facts = memory.get_by_type(MemoryType::Fact, 10).await?;
    for m in &facts {
        println!("   - {}", m.content);
    }
    println!();

    // Get high importance memories
    println!("⭐ High Importance Memories (>= 0.8):");
    let important = memory.get_high_importance(0.8, 10).await?;
    for m in &important {
        println!(
            "   - [{}] {} (importance: {:.1})",
            m.memory_type, m.content, m.importance
        );
    }
    println!();

    // Create associations
    println!("🔗 Creating associations...");
    if facts.len() >= 2 {
        memory
            .associate(&facts[0].id, &facts[1].id, RelationType::RelatedTo)
            .await?;
        println!(
            "✓ Associated '{}' with '{}'\n",
            &facts[0].content[..30],
            &facts[1].content[..30]
        );
    }

    // Get today's memories
    println!("📅 Today's memories:");
    let today = memory.get_today().await?;
    println!("   Found {} memories added today\n", today.len());

    println!("==========================================");
    println!("✅ Example completed successfully!");
    println!("\n💡 Try these commands:");
    println!("   cargo run -- search \"machine learning\"");
    println!("   cargo run -- list");
    println!("   cargo run -- stats");

    Ok(())
}
