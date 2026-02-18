# Quick Start Guide

Get up and running with Goldfish in 5 minutes.

## Prerequisites

- Rust 1.88.0 or later
- 100MB free disk space (for embeddings model)

## Installation

### 1. Create a New Project

```bash
cargo new my-agent
cd my-agent
```

### 2. Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
goldfish = "0.1"
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
```

### 3. Write Your First Agent

Replace `src/main.rs` with:

```rust
use goldfish::{Memory, MemorySystem, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the memory system
    println!("Initializing memory system...");
    let memory = MemorySystem::new("./data").await?;
    
    // Save some facts
    println!("\nSaving memories...");
    let fact1 = Memory::new("Rust is a systems programming language", MemoryType::Fact);
    let fact2 = Memory::new("Rust guarantees memory safety", MemoryType::Fact);
    let preference = Memory::new("User prefers concise explanations", MemoryType::Preference);
    
    memory.save(&fact1).await?;
    memory.save(&fact2).await?;
    memory.save(&preference).await?;
    
    println!("✓ Saved 3 memories");
    
    // Search for memories
    println!("\nSearching for 'programming'...");
    let results = memory.search("programming").await?;
    
    println!("Found {} results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", 
            i + 1, 
            result.memory.content,
            result.score
        );
    }
    
    // Create a relationship
    println!("\nCreating relationship...");
    use goldfish::RelationType;
    memory.associate(&fact1.id, &fact2.id, RelationType::RelatedTo).await?;
    println!("✓ Associated facts");
    
    // Get related memories
    println!("\nGetting related memories...");
    let related = memory.get_associations(&fact1.id).await?;
    println!("Found {} related memories", related.len());
    
    Ok(())
}
```

### 4. Run Your Agent

```bash
cargo run
```

First run will download the embedding model (~50MB).

Expected output:

```
Initializing memory system...

Saving memories...
✓ Saved 3 memories

Searching for 'programming'...
Found 2 results:
  1. Rust is a systems programming language (score: 0.823)
  2. Rust guarantees memory safety (score: 0.756)

Creating relationship...
✓ Associated facts

Getting related memories...
Found 1 related memories
```

## Next Steps

### Explore Memory Types

```rust
use goldfish::{Memory, MemoryType};

// Different types have different importance
let identity = Memory::new("User is a software engineer", MemoryType::Identity);
let goal = Memory::new("Learn Rust programming", MemoryType::Goal);
let todo = Memory::new("Read chapter 1", MemoryType::Todo);
```

### Configure Search

```rust
use goldfish::{SearchConfig, SearchMode, SearchSort};

let config = SearchConfig {
    mode: SearchMode::Hybrid,
    max_results: 10,
    max_graph_depth: 2,
    ..Default::default()
};

let results = memory.search_with_config("query", &config).await?;
```

### Use Connectors

```rust
// Use Pinecone for vectors
use goldfish::connectors::{PineconeConfig, PineconeConnector};

let pinecone_config = PineconeConfig {
    api_key: std::env::var("PINECONE_API_KEY")?,
    index_name: "memories".to_string(),
    ..Default::default()
};

let pinecone = PineconeConnector::new(pinecone_config).await?;
// Configure MemorySystem to use pinecone...
```

## Common Patterns

### Pattern 1: Simple Goldfish

```rust
struct Agent {
    memory: MemorySystem,
}

impl Agent {
    async fn remember(&self, content: &str, mem_type: MemoryType) -> Result<()> {
        let memory = Memory::new(content, mem_type);
        self.memory.save(&memory).await?;
        Ok(())
    }
    
    async fn recall(&self, query: &str) -> Result<Vec<Memory>> {
        let results = self.memory.search(query).await?;
        Ok(results.into_iter().map(|r| r.memory).collect())
    }
}
```

### Pattern 2: Chatbot with Context

```rust
async fn respond(&self, user_input: &str) -> Result<String> {
    // Get relevant context
    let context = self.memory.search(user_input).await?;
    
    // Generate response using context...
    let response = generate_response(&context, user_input);
    
    // Store interaction
    let observation = Memory::new(
        format!("User: {}", user_input),
        MemoryType::Observation
    );
    self.memory.save(&observation).await?;
    
    Ok(response)
}
```

### Pattern 3: Maintenance

```rust
use goldfish::{MaintenanceConfig, run_maintenance};

// Run maintenance periodically
let config = MaintenanceConfig {
    decay_rate: 0.05,
    prune_threshold: 0.1,
    min_age_days: 30,
    ..Default::default()
};

let report = memory.run_maintenance(&config).await?;
println!("Decayed: {}, Pruned: {}", report.decayed, report.pruned);
```

## Troubleshooting

### Compilation Errors

**Error: `rustc` version not supported**
```bash
# Update Rust
rustup update
```

**Error: OpenSSL not found**
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl pkg-config
```

### Runtime Errors

**Error: Permission denied (data directory)**
```bash
# Fix permissions
chmod 755 ./data
```

**Error: Out of disk space**
```bash
# Check space
df -h

# The embedding model needs ~50MB
# Vector database grows with usage
```

### Slow Performance

**First search is slow**
- This is expected! The embedding model is loading.
- Subsequent searches will be fast.

**Large dataset is slow**
- Use PostgreSQL connector for production
- Enable indexes (automatic in built-in mode)
- Consider Pinecone for millions of vectors

## Examples

See the `examples/` directory for complete examples:

- `examples/basic` - Core operations
- `examples/complete/chatbot` - Conversational memory
- `examples/complete/agent` - AI agent integration
- `examples/connectors/pinecone` - Using Pinecone

Run an example:

```bash
cargo run --example basic
cargo run --example chatbot
cargo run --example pinecone --features pinecone
```

## Resources

- **Documentation**: https://docs.rs/goldfish
- **Repository**: https://github.com/harshapalnati/goldfish
- **Issues**: https://github.com/harshapalnati/goldfish/issues

## Getting Help

- 📖 Read the [full documentation](docs/)
- 🐛 [Report bugs](https://github.com/harshapalnati/goldfish/issues)
- 💬 [Start a discussion](https://github.com/harshapalnati/goldfish/discussions)

## Summary

You've learned:

✅ How to install Goldfish  
✅ How to save and retrieve memories  
✅ How to create relationships  
✅ How to search with semantic similarity  
✅ Common usage patterns  

Next: [Architecture Overview](architecture/overview.md) | [API Documentation](https://docs.rs/goldfish) | [Connectors](connectors/README.md)
