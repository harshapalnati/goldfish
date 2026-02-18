# Examples

This directory contains example applications demonstrating various use cases of Goldfish.

## Directory Structure

```
examples/
├── basic/              # Basic usage examples
│   └── simple.rs       # Simple example from quickstart
├── complete/           # Complete applications
│   ├── chatbot/        # Conversational agent
│   └── agent/          # AI agent with memory
└── connectors/         # Database connector examples
    ├── pinecone/
    ├── postgres/
    └── redis/
```

## Running Examples

### Basic Examples

```bash
# Simple usage
cargo run --example simple

# Basic operations
cargo run --example basic
```

### Complete Applications

```bash
# Chatbot with memory
cargo run --example chatbot

# AI agent
cargo run --example agent
```

### Connector Examples

```bash
# Pinecone (requires API key)
export PINECONE_API_KEY="your-key"
cargo run --example pinecone --features pinecone

# PostgreSQL (requires running PostgreSQL)
cargo run --example postgres --features postgres

# Redis (requires running Redis)
cargo run --example redis --features redis
```

## Example Descriptions

### basic/simple.rs

The simplest possible example - shows basic save and search.

**Concepts:**
- Creating a MemorySystem
- Saving memories
- Searching

### basic/basic.rs

Comprehensive example showing all basic operations.

**Concepts:**
- All memory types
- Creating associations
- Searching with filters
- Maintenance operations

### complete/chatbot

Interactive chatbot that remembers conversations.

**Concepts:**
- Session management
- Context retrieval
- Learning from conversations
- Forgetting memories

**Run:**
```bash
cargo run --example chatbot
```

### complete/agent

Full AI agent with sophisticated memory management.

**Concepts:**
- Multi-turn conversations
- Goal tracking
- Decision recording
- Cross-session memory

**Run:**
```bash
cargo run --example agent
```

### connectors/pinecone.rs

Using Pinecone as the vector store.

**Concepts:**
- Pinecone configuration
- Vector storage
- Similarity search
- Metadata filtering

**Requirements:**
- Pinecone account
- API key

### connectors/postgres.rs

Using PostgreSQL for both metadata and vectors.

**Concepts:**
- PostgreSQL connection
- pgvector extension
- Hybrid storage
- Schema initialization

**Requirements:**
- Running PostgreSQL with pgvector

## Creating Your Own Example

1. Create a file in the appropriate directory:
   ```bash
   touch examples/basic/my_example.rs
   ```

2. Add to `Cargo.toml`:
   ```toml
   [[example]]
   name = "my_example"
   path = "examples/basic/my_example.rs"
   ```

3. Write your example:
   ```rust
   use agent_memory::{Memory, MemorySystem, MemoryType};

   #[tokio::main]
   async fn main() -> anyhow::Result<()> {
       let memory = MemorySystem::new("./data").await?;
       // Your example code
       Ok(())
   }
   ```

4. Run it:
   ```bash
   cargo run --example my_example
   ```

## Tips

- Use `MemorySystem::new_in_memory()` for examples that don't need persistence
- Clean up data directories after examples: `rm -rf ./data`
- Use environment variables for credentials
- Add comments explaining key concepts

## Contributing Examples

We welcome new examples! Please:

1. Follow the existing structure
2. Include comprehensive comments
3. Add a README if complex
4. Test on all platforms
5. Follow the code of conduct

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.
