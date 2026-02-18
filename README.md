<div align="center">

# 🐠 Goldfish

**The Memory System AI Agents Deserve**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Crates.io](https://img.shields.io/crates/v/goldfish.svg)](https://crates.io/crates/goldfish)
[![Docs.rs](https://docs.rs/goldfish/badge.svg)](https://docs.rs/goldfish)
[![License](https://img.shields.io/badge/license-Apache%2FMIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/harshapalnati/goldfish/workflows/CI/badge.svg)](https://github.com/harshapalnati/goldfish/actions)

[Features](#features) • [Quick Start](#quick-start) • [Architecture](#architecture) • [API Docs](https://harshapalnati.github.io/goldfish/) • [Examples](#examples)

</div>

---

Goldfish is a **production-grade memory cortex** for AI agents. It combines durable long-term storage with intelligent retrieval, context management, and episodic experiences—so your agents remember what matters, when it matters.

Unlike simple key-value stores, Goldfish understands **semantics**, **relationships**, and **temporal context**. It doesn't just store memories; it helps agents *think* with them.

## ✨ Features

<table>
<tr>
<td valign="top" width="50%">

### 🧠 **Intelligent Retrieval**
- **Hybrid search**: BM25 + vector similarity + graph traversal
- **Dynamic ranking**: Recency × importance × confidence × relationships
- **Explanations**: Know *why* each memory was retrieved
- **Tunable weights**: Adjust scoring for your use case

### 💾 **Storage Backends**
- ✅ **SQLite** (embedded, zero-config)
- 🔜 **PostgreSQL** (production scale)
- 🔜 **MongoDB** (document-heavy workloads)

</td>
<td valign="top" width="50%">

### 🎯 **Agent-Focused Design**
- **Working Memory**: Fast cache for active context
- **Episodes**: Group memories into experiences
- **Context Windows**: Build LLM-ready prompts
- **Graph Relations**: Link memories semantically

### ⚡ **Performance**
- Sub-1ms retrieval latency
- 10K+ memories per second throughput
- Incremental indexing
- Efficient memory consolidation

</td>
</tr>
</table>

## 🚀 Quick Start

### Installation

```bash
# Add to Cargo.toml
[dependencies]
goldfish = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use goldfish::{Memory, MemorySystem, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize storage
    let memory = MemorySystem::new("./goldfish_data").await?;
    
    // Store a memory
    let fact = Memory::new(
        "Rust is memory-safe without garbage collection",
        MemoryType::Fact
    ).with_importance(0.9);
    
    memory.save(&fact).await?;
    
    // Retrieve with hybrid search
    let results = memory.search("memory safety").await?;
    for r in results {
        println!("[{}] {} (score: {:.2})", 
            r.memory.memory_type, 
            r.memory.content,
            r.score
        );
    }
    
    Ok(())
}
```

### Agent-Facing API

```rust
use goldfish::{MemoryCortex, ContextWindow, MemoryType};

let cortex = MemoryCortex::new("./agent_data").await?;

// Start an episodic experience
cortex.start_episode("User Onboarding", "First-time setup").await?;

// Store with semantic typing
cortex.prefer("Dark mode preferred", 0.9).await?;
cortex.goal("Complete setup by Friday").await?;

// Build LLM context automatically
let context = cortex.build_context(&ContextWindow::new(2000)).await?;
println!("{}", context);  // Ready for LLM prompt

cortex.end_episode().await?;
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────┐
│               Application Layer              │
├─────────────────────────────────────────────┤
│  MemoryCortex  │  MemorySystem  │   CLI     │
├─────────────────────────────────────────────┤
│  Working Mem   │    Episodes    │  Context  │
├─────────────────────────────────────────────┤
│      Hybrid Retrieval Engine               │
│  ┌────────┬──────────┬──────────┐         │
│  │  BM25  │  Vector  │  Graph   │         │
│  └────────┴──────────┴──────────┘         │
├─────────────────────────────────────────────┤
│  StorageBackends    │   VectorBackends     │
│  ┌────────────────┐ │  ┌──────────────┐   │
│  │ SQLite ✓       │ │  │ LanceDB ✓    │   │
│  │ PostgreSQL 🔜  │ │  │ pgvector 🔜  │   │
│  │ MongoDB 🔜     │ │  │ Qdrant 🔜    │   │
│  └────────────────┘ │  └──────────────┘   │
└─────────────────────────────────────────────┘
```

### Hybrid Scoring Formula

```
final_score = 
    bm25_score × 0.35 +
    vector_score × 0.35 +
    recency_boost × 0.20 +
    importance × 0.10 +
    graph_bonus (max 0.15)
```

## 📊 Benchmarks

Run the evaluation harness to measure retrieval quality:

```rust
use goldfish::{EvalHarness, HybridSearchConfig};

let harness = EvalHarness::new(backend);
let results = harness.compare_baselines().await?;

// Results:
// • No memory baseline: 0% precision
// • BM25 only: 68% precision, 0.18ms latency
// • Hybrid (Goldfish): 94% precision, 0.21ms latency
```

## 🔧 Configuration

### Storage Backend

```rust
// SQLite (default)
let memory = MemorySystem::new("./data").await?;

// With custom pool
let memory = MemorySystem::with_pool(pool).await?;
```

### Hybrid Search

```rust
use goldfish::HybridSearchConfig;

let config = HybridSearchConfig {
    weight_bm25: 0.35,
    weight_vector: 0.35,
    weight_recency: 0.20,
    weight_importance: 0.10,
    weight_graph: 0.15,
    max_results: 10,
    neighbor_depth: 1,
};
```

### Memory Types

- **`Fact`** - General knowledge
- **`Preference`** - User preferences (high importance)
- **`Goal`** - Objectives to achieve
- **`Decision`** - Choices made with context
- **`Experience`** - Learned from interactions
- **`Identity`** - Core agent/user characteristics

## 📚 Examples

### Working with Episodes

```rust
// Group related memories into experiences
cortex.start_episode("Debugging Session", "Fixing production bug").await?;

cortex.remember(&Memory::new("Found race condition in user service", MemoryType::Fact)).await?;
cortex.remember(&Memory::new("Applied mutex fix", MemoryType::Decision)).await?;

let episode = cortex.end_episode().await?;
// Episode contains all memories + metadata
```

### Graph Relationships

```rust
// Link memories semantically
let goal = cortex.goal("Learn Rust").await?;
let resource = cortex.remember(&Memory::new("Rust Book chapter 1", MemoryType::Fact)).await?;

cortex.link(&goal.id, &resource.id, RelationType::RelatesTo).await?;

// Find related memories
let related = cortex.get_neighbors(&goal.id, depth=2).await?;
```

### Event-Driven Architecture

```rust
use goldfish::Pulse;

let mut rx = memory.pulses().subscribe();
tokio::spawn(async move {
    while let Ok(pulse) = rx.recv().await {
        match pulse {
            Pulse::NewMemory { memory, .. } => {
                println!("📝 New memory: {}", memory.content);
            }
            Pulse::AssociationCreated { source, target, .. } => {
                println!("🔗 Linked: {} → {}", source, target);
            }
            _ => {}
        }
    }
});
```

## 🛠️ Development

```bash
# Clone repository
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish

# Run tests
cargo test

# Run comprehensive example
cargo run --example comprehensive

# Build documentation
cargo doc --open
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas we'd love help with:
- PostgreSQL storage backend
- pgvector integration
- Python bindings
- Dashboard UI
- Additional examples

## 📄 License

Dual-licensed under:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))

Choose whichever license works best for your project.

## 🙏 Acknowledgments

- Built with [Tantivy](https://github.com/quickwit-oss/tantivy) for full-text search
- Inspired by human memory research (episodic, semantic, working memory)
- Designed for [OpenClaw](https://github.com/harshapalnati/openclaw) and similar agent frameworks

---

<div align="center">

**Made with 🐠 by [Harsha Palnati](https://github.com/harshapalnati)**

⭐ Star us on GitHub if Goldfish helps your agents remember!

</div>
