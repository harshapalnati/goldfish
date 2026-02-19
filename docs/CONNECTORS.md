# Connector System Documentation

## Overview

The Goldfish connector system allows you to use various external databases and vector stores instead of (or alongside) the built-in SQLite + LanceDB setup.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Goldfish                             │
├─────────────────────────────────────────────────────────────┤
│  MemorySystem                                               │
│  ┌─────────────────┬─────────────────────────────────────┐ │
│  │  Metadata Store │         Vector Store                │ │
│  ├─────────────────┼─────────────────────────────────────┤ │
│  │  - SQLite       │  - Pinecone                         │ │
│  │  - PostgreSQL   │  - Qdrant                           │ │
│  │  - MongoDB      │  - Weaviate                         │ │
│  │  (Graph edges)  │  - Milvus                           │ │
│  │                 │  - Redis                            │ │
│  │                 │  - LanceDB (built-in)               │ │
│  └─────────────────┴─────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Available Connectors

### Fully Implemented

1. **SQLite + LanceDB** (built-in, default)
   - No external dependencies
   - Local file storage
   - Good for development and small deployments

2. **Pinecone** (cloud vector database)
   - Managed service
   - Scales automatically
   - Production-ready
   - Enable with feature: `pinecone`

3. **PostgreSQL + pgvector** (relational + vector)
   - Full ACID compliance
   - Can store both metadata and vectors
   - Well-supported in production environments
   - Enable with feature: `postgres`

4. **Redis** (caching + vector storage)
   - Fast in-memory operations
   - Good for caching layer
   - Vector search requires Redis Stack
   - Enable with feature: `redis`

### Stub Implementations (Ready to Extend)

5. **Qdrant** - Open-source vector database in Rust
6. **ChromaDB** - Popular embedding database
7. **MongoDB** - Document database with Atlas vector search
8. **Weaviate** - Vector database with GraphQL interface
9. **Milvus** - Cloud-native vector database

## Using Connectors

### Basic Usage with Built-in Storage

```rust
use goldfish::MemorySystem;

let memory = MemorySystem::new("./data").await?;
```

### Using Pinecone for Vectors

```rust
use goldfish::connectors::{PineconeConfig, PineconeConnector};
use goldfish::MemorySystemBuilder;

let pinecone_config = PineconeConfig {
    api_key: std::env::var("PINECONE_API_KEY")?,
    index_name: "memories".to_string(),
    namespace: Some("agent-1".to_string()),
    ..Default::default()
};

let pinecone = PineconeConnector::new(pinecone_config).await?;

let memory = MemorySystemBuilder::new()
    .data_dir("./data")
    .vector_store(pinecone)
    .build()
    .await?;
```

### Using PostgreSQL for Everything

```rust
use goldfish::connectors::{PostgresConfig, PostgresConnector};

let pg_config = PostgresConfig {
    connection_string: "postgres://user:pass@localhost/db".to_string(),
    table_name: "memories".to_string(),
    use_pgvector: true,
    ..Default::default()
};

let postgres = PostgresConnector::new(pg_config).await?;

// PostgreSQL can handle both metadata and vectors
let memory = MemorySystemBuilder::new()
    .hybrid_store(postgres)
    .build()
    .await?;
```

### Hybrid Setup: SQLite + Pinecone

```rust
use goldfish::MemorySystemBuilder;
use goldfish::connectors::PineconeConnector;

// Use SQLite for graph/metadata, Pinecone for vectors
let pinecone = PineconeConnector::new(config).await?;

let memory = MemorySystemBuilder::new()
    .data_dir("./data")  // SQLite location
    .vector_store(pinecone)
    .build()
    .await?;
```

## Connector Configuration

### Pinecone

```rust
PineconeConfig {
    api_key: String,           // Required: Pinecone API key
    index_name: String,        // Required: Index name
    namespace: Option<String>, // Optional: Namespace for multi-tenancy
    dimension: usize,          // Default: 384
    environment: Option<String>, // Legacy: Environment name
    base_url: Option<String>,  // Optional: Custom endpoint
}
```

**Environment Variables:**
- `PINECONE_API_KEY` - Your Pinecone API key

### PostgreSQL

```rust
PostgresConfig {
    connection_string: String,     // Required: Connection URL
    vector_dimension: usize,       // Default: 384
    table_name: String,            // Default: "memories"
    associations_table: String,    // Default: "associations"
    use_pgvector: bool,            // Default: true
}
```

**Prerequisites:**
```sql
-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;
```

**Connection String Format:**
```
postgres://username:password@host:port/database
```

### Redis

```rust
RedisConfig {
    url: String,                   // Default: "redis://localhost:6379"
    key_prefix: String,            // Default: "memory:"
    vector_dimension: usize,       // Default: 384
    enable_vectors: bool,          // Default: false (requires Redis Stack)
    default_ttl: Option<u64>,      // Default: Some(3600) - 1 hour
}
```

**Note:** Full vector search requires Redis Stack (not just Redis).

## Building with Connectors

### Enable Specific Connectors

```bash
# Just Pinecone
cargo build --features pinecone

# PostgreSQL + Redis
cargo build --features "postgres redis"

# All connectors
cargo build --features all-connectors
```

### Cargo.toml Configuration

```toml
[dependencies]
goldfish = { 
    version = "0.1",
    features = ["pinecone", "postgres"] 
}
```

## Connector Traits

### VectorStore

For vector-only databases:

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_vector(&self, id: &str, embedding: &[f32], metadata: Option<HashMap<String, String>>) -> ConnectorResult<()>;
    async fn delete_vector(&self, id: &str) -> ConnectorResult<()>;
    async fn search_similar(&self, query: &[f32], limit: usize, filter: Option<VectorFilter>) -> ConnectorResult<Vec<VectorSearchResult>>;
    async fn vector_exists(&self, id: &str) -> ConnectorResult<bool>;
    fn vector_dimension(&self) -> usize;
    fn name(&self) -> &'static str;
}
```

### MetadataStore

For relational databases:

```rust
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn save_memory(&self, memory: &Memory) -> ConnectorResult<()>;
    async fn load_memory(&self, id: &str) -> ConnectorResult<Option<Memory>>;
    async fn update_memory(&self, memory: &Memory) -> ConnectorResult<()>;
    async fn delete_memory(&self, id: &str) -> ConnectorResult<()>;
    async fn create_association(&self, association: &Association) -> ConnectorResult<()>;
    async fn get_associations(&self, memory_id: &str) -> ConnectorResult<Vec<Association>>;
    async fn query_memories(&self, query: &MetadataQuery) -> ConnectorResult<Vec<Memory>>;
    async fn health_check(&self) -> ConnectorResult<()>;
    fn name(&self) -> &'static str;
}
```

### HybridStore

For databases that can do both:

```rust
#[async_trait]
pub trait HybridStore: VectorStore + MetadataStore {
    async fn hybrid_search(&self, query: &[f32], filter: Option<MetadataQuery>, limit: usize) -> ConnectorResult<Vec<HybridSearchResult>>;
    async fn store_memory_with_embedding(&self, memory: &Memory, embedding: &[f32]) -> ConnectorResult<()>;
}
```

## Implementing a Custom Connector

1. **Create a new module** in `src/connectors/`

2. **Implement the trait(s)**:

```rust
use goldfish::connectors::traits::{VectorStore, VectorFilter, VectorSearchResult};
use goldfish::connectors::error::{ConnectorError, ConnectorResult};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct MyConnector {
    // Your configuration
}

#[async_trait]
impl VectorStore for MyConnector {
    async fn store_vector(&self, id: &str, embedding: &[f32], metadata: Option<HashMap<String, String>>) -> ConnectorResult<()> {
        // Implementation
        Ok(())
    }
    
    // ... implement other methods
    
    fn name(&self) -> &'static str {
        "MyConnector"
    }
}
```

3. **Add to the connectors module**:

```rust
// In src/connectors/mod.rs
#[cfg(feature = "myconnector")]
pub mod myconnector;
#[cfg(feature = "myconnector")]
pub use myconnector::{MyConfig, MyConnector};
```

4. **Add feature to Cargo.toml**:

```toml
[features]
myconnector = ["dep:my-dependency"]
```

## Performance Considerations

### Vector Stores

| Connector | Latency | Throughput | Best For |
|-----------|---------|------------|----------|
| LanceDB (local) | Low | Medium | Development, edge deployment |
| Pinecone | Medium | High | Production, cloud-native |
| PostgreSQL + pgvector | Low-Medium | Medium | Existing Postgres users |
| Qdrant | Low | High | Self-hosted production |
| Redis | Very Low | Very High | Caching layer |

### Metadata Stores

| Connector | Query Performance | Best For |
|-----------|-------------------|----------|
| SQLite | Fast (local) | Single-node, development |
| PostgreSQL | Fast | Production, complex queries |
| MongoDB | Fast | Document-oriented apps |

## Migration Guide

### From Built-in to Pinecone

1. **Export existing memories**:
```rust
let memories = memory.get_by_type(MemoryType::Fact, 10000).await?;
```

2. **Re-import to new system**:
```rust
let pinecone = PineconeConnector::new(config).await?;
for mem in memories {
    let embedding = embedding_model.embed_one(&mem.content).await?;
    pinecone.store_vector(&mem.id, &embedding, Some(metadata)).await?;
}
```

### Backup Strategy

- **SQLite**: Copy the `.db` file
- **PostgreSQL**: Use `pg_dump`
- **Pinecone**: Export via API
- **Redis**: Use `BGSAVE`

## Troubleshooting

### Pinecone

**Error: "Index not found"**
- Ensure the index exists in your Pinecone dashboard
- Check that `index_name` matches exactly

**Error: "Authentication failed"**
- Verify `PINECONE_API_KEY` is set correctly
- Check API key permissions

### PostgreSQL

**Error: "pgvector extension not found"**
```sql
-- Install pgvector
CREATE EXTENSION IF NOT EXISTS vector;
```

**Error: "Connection refused"**
- Check PostgreSQL is running
- Verify connection string host/port
- Check firewall rules

### Redis

**Error: "Vector operations not supported"**
- Install Redis Stack (not just Redis)
- Or use Redis Enterprise with RediSearch module

## Best Practices

1. **Use environment variables** for credentials, never hardcode them
2. **Enable health checks** in production
3. **Use connection pooling** when available
4. **Set appropriate TTLs** for cached data in Redis
5. **Create indexes** for frequently queried fields
6. **Monitor latency** and throughput
7. **Use namespaces** in Pinecone for multi-tenancy
8. **Implement retry logic** for transient failures

## Future Connectors

Planned additions:
- Elasticsearch/OpenSearch
- OpenAI Embeddings API (for cloud embeddings)
- AWS OpenSearch
- Azure Cognitive Search
- Google Vertex AI Matching Engine

## Contributing New Connectors

1. Follow the existing connector patterns
2. Implement all required trait methods
3. Add comprehensive error handling
4. Include documentation and examples
5. Add integration tests
6. Update this documentation

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.
