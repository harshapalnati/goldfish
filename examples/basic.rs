//! Basic example showing core memory operations

use goldfish::{Memory, MemorySystem, MemoryType, RelationType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Goldfish - Basic Example ===\n");

    // Create a new memory system
    let memory = MemorySystem::new("./data/basic_example").await?;
    println!("✓ Memory system initialized\n");

    // Create some memories
    println!("Creating memories...");

    let rust_fact = Memory::new(
        "Rust is a systems programming language with memory safety guarantees",
        MemoryType::Fact,
    );
    memory.save(&rust_fact).await?;
    println!("  ✓ Saved: '{}'", rust_fact.content);

    let ownership_fact = Memory::new(
        "Rust uses an ownership system to manage memory without a garbage collector",
        MemoryType::Fact,
    );
    memory.save(&ownership_fact).await?;
    println!("  ✓ Saved: '{}'", ownership_fact.content);

    let user_pref = Memory::new(
        "User prefers concise explanations",
        MemoryType::Preference,
    )
    .with_importance(0.8);
    memory.save(&user_pref).await?;
    println!("  ✓ Saved: '{}' (importance: {})", user_pref.content, user_pref.importance);

    let goal = Memory::new(
        "Learn Rust programming",
        MemoryType::Goal,
    );
    memory.save(&goal).await?;
    println!("  ✓ Saved: '{}'", goal.content);

    // Create an association
    println!("\nCreating associations...");
    memory
        .associate(&goal.id, &rust_fact.id, RelationType::RelatedTo)
        .await?;
    println!("  ✓ Associated goal with Rust fact");

    // Search for memories
    println!("\nSearching for 'memory safety'...");
    let results = memory.search("memory safety").await?;
    for (i, result) in results.iter().take(5).enumerate() {
        println!(
            "  {}. {} (type: {}, score: {:.3})",
            i + 1,
            result.memory.content,
            result.memory.memory_type,
            result.score
        );
    }

    // Get memories by type
    println!("\nAll facts:");
    let facts = memory.get_by_type(MemoryType::Fact, 10).await?;
    for fact in facts {
        println!("  • {}", fact.content);
    }

    // Get high-importance memories
    println!("\nHigh-importance memories (≥0.7):");
    let important = memory.get_high_importance(0.7, 10).await?;
    for mem in important {
        println!("  • {} (importance: {})", mem.content, mem.importance);
    }

    // Demonstrate forgetting
    println!("\nForgetting a memory...");
    memory.forget(&ownership_fact.id).await?;
    println!("  ✓ Forgotten: '{}'", ownership_fact.content);

    println!("\nSearching again (forgotten memory excluded):");
    let results = memory.search("garbage collector").await?;
    if results.is_empty() {
        println!("  (No results - memory was forgotten)");
    } else {
        for result in results {
            println!("  • {}", result.memory.content);
        }
    }

    // Restore the memory
    println!("\nRestoring memory...");
    memory.restore(&ownership_fact.id).await?;
    println!("  ✓ Restored: '{}'", ownership_fact.content);

    println!("\n=== Example complete ===");
    Ok(())
}
