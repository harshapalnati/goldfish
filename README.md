# Goldfish

Goldfish is a typed, graph-connected memory cortex for AI agents. It provides durable long-term storage (SQLite), fast full-text search (Tantivy), and agent-facing ergonomics (working memory + episodic “experiences”).

## What you get

- **Typed memories** (`MemoryType`) with importance and confidence scoring.
- **Graph relationships** between memories (`RelationType`, `Association`).
- **Full-text search** over the corpus (`MemorySystem.search*`, `SearchConfig`).
- **Agent-focused API** (`MemoryCortex`) with working memory controls (think/focus/pin) and episodic experiences.
- **Context window builder** for LLM prompts (`ContextWindow`).
- **Event stream** for changes (`GoldfishPulses`, `Pulse`).
- **Maintenance** utilities (decay/prune/optional consolidation helpers).

## Installation

Goldfish is a Rust crate with an optional CLI and a separate REST server (`goldfish-server`).

### As a dependency (Git)

```toml
[dependencies]
goldfish = { git = "https://github.com/harshapalnati/goldfish", branch = "main" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

### From source (local dev)

```bash
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish
cargo test
```

## Quick start (library)

### 1) Durable storage + search (`MemorySystem`)

`MemorySystem` persists to SQLite in the directory you provide and maintains a Tantivy index under the same directory.

```rust
use goldfish::{Memory, MemorySystem, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem = MemorySystem::new("./goldfish_data").await?;

    let fact = Memory::new("Rust is memory-safe", MemoryType::Fact)
        .with_source("docs");
    mem.save(&fact).await?;

    let results = mem.search("memory-safe").await?;
    for r in results {
        println!("[{}] {} (score {:.2})", r.memory.memory_type, r.memory.content, r.score);
    }

    Ok(())
}
```

If you want more control over ranking and filtering:

```rust
use goldfish::{MemorySystem, SearchConfig, SearchMode, SearchSort, MemoryType};

# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
let mem = MemorySystem::new("./goldfish_data").await?;

let config = SearchConfig {
    mode: SearchMode::FullText,
    memory_type: Some(MemoryType::Fact),
    sort_by: SearchSort::Recent,
    max_results: 20,
    fuzzy: true,
    boost_recent: true,
};

let _results = mem.search_with_config("memroy safty", &config).await?;
# Ok(()) }
```

### 2) Agent-facing workflows (`MemoryCortex`)

`MemoryCortex` layers “working memory” and “experiences” on top of the same SQLite store.

```rust
use goldfish::{ContextWindow, Memory, MemoryCortex, MemoryType, RelationType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cortex = MemoryCortex::new("./agent_data").await?;

    // Start an episodic experience (optional)
    let _episode_id = cortex.start_episode("Onboarding", "Initial setup session").await?;

    // Remember durable knowledge
    let pref = Memory::new("User prefers concise answers", MemoryType::Preference)
        .with_importance(0.8);
    cortex.remember(&pref).await?;

    // Bring something into working memory (attention)
    cortex.think_about(&pref.id).await?;
    cortex.pin(&pref.id).await;

    // Link memories (graph edge)
    let goal = cortex.goal("Ship v0.1").await?;
    cortex.link(&goal.id, &pref.id, RelationType::RelatedTo).await?;

    // Build prompt-ready context
    let ctx = cortex.build_context(&ContextWindow::new(2000)).await?;
    println!("{ctx}");

    cortex.end_episode().await?;
    Ok(())
}
```

## Events (Pulses)

Subscribe to an in-process event stream for new memories, updates, deletions, associations, and maintenance operations:

```rust
use goldfish::{Memory, MemorySystem, MemoryType, Pulse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem = MemorySystem::new("./goldfish_data").await?;

    let mut rx = mem.pulses().subscribe();
    tokio::spawn(async move {
        while let Ok(p) = rx.recv().await {
            if let Pulse::NewMemory { memory, .. } = p {
                println!("new memory: {}", memory.content);
            }
        }
    });

    mem.save(&Memory::new("Hello", MemoryType::Fact)).await?;
    Ok(())
}
```

## Maintenance

Run decay/prune tasks over the SQLite store:

```rust
use goldfish::{MemorySystem, MaintenanceConfig, run_maintenance};

# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
let mem = MemorySystem::new("./goldfish_data").await?;
let report = run_maintenance(mem.store(), &MaintenanceConfig::default()).await?;
println!("decayed={}, pruned={}", report.decayed, report.pruned);
# Ok(()) }
```

## CLI

Install the CLI from the repo:

```bash
cargo install --path .
goldfish --help
```

Common commands:

```bash
goldfish init --name my-agent
goldfish add "User likes dark mode" --memory-type preference --importance 0.8
goldfish search "dark mode" --limit 5
goldfish list --sort created --limit 20
goldfish get <id> --verbose
goldfish stats
```

## REST server (`goldfish-server`)

Run the standalone server:

```bash
cargo run -p goldfish-server
```

Endpoints:

- `GET /health`
- `POST /v1/memory` (JSON: `{ "content": "...", "memory_type": "fact", "importance": 0.5 }`)
- `GET /v1/search?q=...&limit=...`
- `GET /v1/context`

## Dashboard (optional)

Goldfish includes an API server for a future dashboard behind the `dashboard` feature flag.
To embed it in your own binary:

```toml
[dependencies]
goldfish = { git = "https://github.com/harshapalnati/goldfish", branch = "main", features = ["dashboard"] }
```

## License

Dual-licensed under Apache-2.0 and MIT.
