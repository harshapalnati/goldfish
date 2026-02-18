# Architecture Overview

This document provides a high-level overview of the Goldfish system architecture.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Application Layer                            │
│                    (Your AI Agent / Chatbot)                         │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         MemorySystem API                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │    save()   │  │   search()  │  │  associate()│  │  forget() │  │
│  │    load()   │  │ get_by_type()│  │get_neighbors│  │  prune()  │  │
│  │   update()  │  │get_high_imp()│  │get_associations│  │  decay()  │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Core Components                                 │
│                                                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   Types Module  │  │  Search Module  │  │ Maintenance Module  │  │
│  │                 │  │                 │  │                     │  │
│  │ • Memory        │  │ • Hybrid search │  │ • Decay             │  │
│  │ • MemoryType    │  │ • Vector search │  │ • Pruning           │  │
│  │ • Association   │  │ • Text search   │  │ • Merging           │  │
│  │ • RelationType  │  │ • Graph search  │  │ • Cleanup           │  │
│  └─────────────────┘  │ • RRF fusion    │  └─────────────────────┘  │
│                       └─────────────────┘                           │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Storage Layer                                   │
│                                                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  MetadataStore  │  │   VectorStore   │  │    HybridStore      │  │
│  │    (Trait)      │  │    (Trait)      │  │    (Trait)          │  │
│  │                 │  │                 │  │                     │  │
│  │ Graph edges     │  │ Embeddings      │  │ Both in one         │  │
│  │ Memory metadata │  │ Similarity      │  │ Atomic operations   │  │
│  │ Associations    │  │ Vector ops      │  │ Hybrid search       │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                   │
        ┌──────────────────────────┼──────────────────────────┐
        │                          │                          │
        ▼                          ▼                          ▼
┌───────────────┐        ┌───────────────┐        ┌───────────────┐
│   Built-in    │        │   External    │        │    Cloud      │
│  SQLite+Lance │        │   PostgreSQL  │        │    Pinecone   │
│               │        │     Qdrant    │        │    Weaviate   │
│  ┌─────────┐  │        │     Redis     │        │               │
│  │ SQLite  │  │        │               │        │               │
│  │(metadata│  │        │  ┌─────────┐  │        │  ┌─────────┐  │
│  │  graph) │  │        │  │pgvector │  │        │  │ REST API│  │
│  └─────────┘  │        │  │(vectors)│  │        │  └─────────┘  │
│  ┌─────────┐  │        │  └─────────┘  │        │               │
│  │ LanceDB │  │        └───────────────┘        └───────────────┘
│  │(vectors│  │
│  │  text) │  │
│  └─────────┘  │
└───────────────┘
```

## Design Principles

### 1. Separation of Concerns

The system separates storage into two distinct concerns:

- **Metadata Storage**: Graph relationships, memory metadata, associations (SQLite, PostgreSQL)
- **Vector Storage**: Embeddings and similarity search (LanceDB, Pinecone, Qdrant)

This allows users to:
- Use different databases for different concerns
- Scale vector storage independently from metadata
- Choose the best tool for each job

### 2. Type Safety

All operations are type-safe:

```rust
// Memory types are enums, not strings
let fact = Memory::new("Content", MemoryType::Fact);

// Relation types are enforced
memory.associate(&id1, &id2, RelationType::PartOf).await?;

// Configuration is strongly typed
let config = PineconeConfig { ... };
```

### 3. Async-First

All operations are async:

```rust
// All methods are async
pub async fn save(&self, memory: &Memory) -> Result<()>;
pub async fn search(&self, query: &str) -> Result<Vec<MemorySearchResult>>;
```

Benefits:
- Non-blocking I/O
- Concurrent operations
- Better resource utilization
- Scales to many concurrent users

### 4. Pluggable Architecture

Connectors implement traits:

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_vector(&self, ...) -> ConnectorResult<()>;
    async fn search_similar(&self, ...) -> ConnectorResult<Vec<...>>;
    // ...
}
```

New connectors can be added without changing core code.

## Core Components

### MemorySystem

The main entry point. Provides high-level operations:

```rust
pub struct MemorySystem {
    store: Arc<MemoryStore>,
    search: MemorySearch,
    embedding_model: Arc<EmbeddingModel>,
}
```

**Responsibilities:**
- Coordinate between storage and search
- Manage embedding generation
- Provide user-friendly API

### Types Module

Core domain types:

- `Memory`: A piece of knowledge with type and importance
- `MemoryType`: Enum of 8 semantic types
- `Association`: Graph edge between memories
- `RelationType`: Type of relationship

### Store Module

SQLite implementation:

- CRUD operations for memories
- Graph operations (associations, neighbors)
- Query building
- Index management

### Lance Module

LanceDB integration:

- Vector storage
- HNSW index
- Full-text search (Tantivy)
- Similarity search

### Embedding Module

Embedding generation:

- fastembed integration
- all-MiniLM-L6-v2 model
- Batch processing
- Local inference (no API calls)

### Search Module

Hybrid search implementation:

- Vector similarity
- Full-text search
- Graph traversal
- RRF fusion
- Result ranking

### Maintenance Module

Automatic memory management:

- Importance decay
- Pruning
- Merging (future)
- Cleanup

### Connectors Module

Pluggable storage backends:

- Trait definitions
- Error types
- Connector implementations
- Configuration structs

## Data Flow

### Saving a Memory

```
User calls save(memory)
        │
        ▼
┌──────────────────┐
│  MemorySystem    │
│  1. Validate     │
│  2. Generate     │
│     embedding    │
└────────┬─────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐ ┌────────┐
│SQLite  │ │LanceDB │
│(meta)  │ │(vector)│
└────────┘ └────────┘
    │         │
    └────┬────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐ ┌────────┐
│Check   │ │Create  │
│similar │ │assoc   │
└────────┘ └────────┘
```

### Searching

```
User calls search(query)
        │
        ▼
┌──────────────────┐
│  MemorySystem    │
│  1. Embed query  │
│  2. Dispatch     │
│     search       │
└────────┬─────────┘
         │
    ┌────┴────┬────────┐
    │         │        │
    ▼         ▼        ▼
┌────────┐ ┌────────┐ ┌────────┐
│Vector  │ │  Text  │ │ Graph  │
│Search  │ │ Search │ │ Traversal
└────┬───┘ └───┬────┘ └────┬───┘
     │         │           │
     └────┬────┴─────┬─────┘
          │          │
          ▼          ▼
    ┌──────────────────┐
    │   RRF Fusion     │
    │   Merge results  │
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  Return ranked   │
    │  results         │
    └──────────────────┘
```

## Memory Lifecycle

```
┌─────────┐
│  Create │
│  Memory │
└────┬────┘
     │
     ▼
┌─────────┐     ┌─────────┐
│  Save   │────▶│  Store  │
│         │     │         │
└────┬────┘     │ • SQLite│
     │          │ • Lance │
     ▼          └─────────┘
┌─────────┐
│ Access  │
│ Recorded│
└────┬────┘
     │
     ▼
┌─────────┐
│  Decay  │◀─── Time passes
│  Check  │
└────┬────┘
     │
┌────┴────┐
│Importance│
│decreases?│
└────┬────┘
     │
  Yes│    No
     │     │
     ▼     │
┌─────────┐│
│  Prune  ││
│  Check  ││
└────┬────┘│
     │     │
  Yes│    No
     │     │
     ▼     │
┌─────────┐│
│  Forget ││
│(soft    ││
│ delete) ││
└─────────┘│
           │
           ▼
    ┌─────────────┐
    │  Continue   │
    │  Lifecycle  │
    └─────────────┘
```

## Connector Architecture

Connectors abstract different storage backends:

```
┌─────────────────────────────────────────────────────┐
│                    Application                       │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│                 MemorySystem                         │
│                                                      │
│  Uses traits: VectorStore, MetadataStore,            │
│  HybridStore                                        │
└─────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│VectorStore   │ │MetadataStore │ │ HybridStore  │
│   Trait      │ │   Trait      │ │   Trait      │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       │                │                │
       ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  LanceDB     │ │   SQLite     │ │  PostgreSQL  │
│  (vectors)   │ │  (metadata)  │ │  (+pgvector) │
└──────────────┘ └──────────────┘ └──────────────┘
       │                │                │
       ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   Pinecone   │ │  PostgreSQL  │ │   MongoDB    │
│   (cloud)    │ │  (metadata)  │ │  (hybrid)    │
└──────────────┘ └──────────────┘ └──────────────┘
```

Benefits:
- **Flexibility**: Use any combination of backends
- **Testability**: Easy to mock for testing
- **Extensibility**: Add new backends without changes
- **Performance**: Choose optimal backend for each use case

## Performance Characteristics

### Built-in (SQLite + LanceDB)

- **Best for**: Development, edge deployment, single-node
- **Capacity**: ~1M memories
- **Latency**: Low (local disk)
- **Scaling**: Vertical only

### PostgreSQL + pgvector

- **Best for**: Production, existing Postgres users
- **Capacity**: ~10M+ memories
- **Latency**: Low (local network)
- **Scaling**: Read replicas, connection pooling

### Pinecone

- **Best for**: Cloud-native, managed service
- **Capacity**: Billions of vectors
- **Latency**: Medium (internet)
- **Scaling**: Automatic

### Redis

- **Best for**: Caching, real-time
- **Capacity**: Memory limited
- **Latency**: Very low
- **Scaling**: Redis Cluster

## Security Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Application                        │
│           (Authentication/Authorization)             │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│                 MemorySystem                         │
│                                                      │
│  • Input validation                                  │
│  • Parameterized queries (SQL injection prevention) │
│  • Content length limits                            │
└─────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   SQLite     │ │  PostgreSQL  │ │   Pinecone   │
│              │ │              │ │              │
│ • File perms │ │ • TLS        │ │ • API keys   │
│ • Encryption │ │ • Auth       │ │ • HTTPS      │
└──────────────┘ └──────────────┘ └──────────────┘
```

## Error Handling

```
┌─────────────────────────────────────────────────────┐
│                  User Code                           │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│              Result<T, MemoryError>                  │
│                                                      │
│  • Database errors                                   │
│  • Vector DB errors                                  │
│  • Not found                                         │
│  • Invalid operations                                │
└─────────────────────────────────────────────────────┘
                        │
            ┌───────────┴───────────┐
            │                       │
            ▼                       ▼
┌───────────────────┐   ┌───────────────────┐
│   ConnectorError  │   │    MemoryError    │
│                   │   │                   │
│ • Connection      │   │ • Validation      │
│ • Authentication  │   │ • Search          │
│ • Operation       │   │ • Maintenance     │
└───────────────────┘   └───────────────────┘
```

## Future Architecture

### Planned Enhancements

1. **Distributed Storage**
   - Sharding support
   - Multi-region replication
   - Consensus for metadata

2. **Caching Layer**
   - Multi-tier caching
   - Cache invalidation
   - Redis integration

3. **Observability**
   - Metrics (Prometheus)
   - Tracing (OpenTelemetry)
   - Structured logging

4. **WASM Support**
   - Browser-compatible
   - Edge deployment
   - Smaller bundle size

## Conclusion

The Goldfish architecture is designed for:

- **Flexibility**: Pluggable connectors
- **Performance**: Optimized storage per use case
- **Scalability**: From edge to cloud
- **Maintainability**: Clear separation of concerns
- **Type Safety**: Rust's type system
- **Async**: Modern async/await

This architecture enables AI agents to have sophisticated, long-term memory that scales from prototypes to production.
