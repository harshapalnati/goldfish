# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release
- Core memory system with 8 semantic memory types
- Graph relationship system
- Hybrid search (vector + text + graph)
- Maintenance system (decay, pruning)
- Pinecone connector
- PostgreSQL + pgvector connector
- Redis connector
- Qdrant stub
- ChromaDB stub
- MongoDB stub
- Weaviate stub
- Milvus stub

## [0.1.0] - 2024-XX-XX

### Added

#### Core Features
- **Memory System**: Typed, graph-connected memory storage
  - 8 memory types: Identity, Goal, Decision, Todo, Preference, Fact, Event, Observation
  - Automatic importance scoring
  - Soft delete (forget) functionality
  - Session-based isolation

- **Graph System**: Relationship management
  - 6 relation types: RelatedTo, Updates, Contradicts, CausedBy, ResultOf, PartOf
  - Automatic association on high similarity
  - Graph traversal for context expansion
  - Bidirectional relationship support

- **Search System**: Hybrid search with RRF
  - Vector similarity search (HNSW index)
  - Full-text search (Tantivy)
  - Graph traversal search
  - Reciprocal Rank Fusion (RRF)
  - Configurable search modes

- **Maintenance**: Automatic memory management
  - Importance decay over time
  - Configurable decay rates
  - Pruning of low-importance memories
  - Access count tracking

#### Storage
- **Built-in**: SQLite + LanceDB
  - No external dependencies
  - Local file storage
  - HNSW vector index
  - Full-text search index

#### Connectors
- **Pinecone**: Full REST API implementation
  - Vector storage and retrieval
  - Similarity search with metadata filtering
  - Namespace support
  - Health checks

- **PostgreSQL + pgvector**: Complete integration
  - Full CRUD operations
  - Automatic schema initialization
  - Index creation
  - Health checks

- **Redis**: Caching layer
  - Vector storage
  - TTL support
  - Connection pooling

- **Stubs**: Ready for implementation
  - Qdrant
  - ChromaDB
  - MongoDB
  - Weaviate
  - Milvus

#### Embedding
- Local embedding generation via fastembed
- all-MiniLM-L6-v2 model (384 dimensions)
- Batch embedding support
- Async operations

#### Documentation
- Comprehensive README
- Architecture documentation
- API documentation
- Connector guides
- Usage examples
- Security documentation

### Security
- Input validation
- Parameterized queries (SQL injection prevention)
- No hardcoded credentials
- Support for TLS connections

### Performance
- ~15ms for save operation
- ~8ms for vector search
- ~20ms for hybrid search
- HNSW index for fast similarity search

---

## Release Notes Template

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security fixes
```

---

## Versioning Guide

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

### Examples

- `0.1.0` → `0.2.0`: Added new connector (minor)
- `0.1.0` → `0.1.1`: Fixed search bug (patch)
- `0.9.0` → `1.0.0`: Stable API release (major)

---

## Migration Guides

### Upgrading to 0.2.0 (Future)

When 0.2.0 is released, this section will contain migration instructions.

---

[Unreleased]: https://github.com/harshapalnati/goldfish/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/harshapalnati/goldfish/releases/tag/v0.1.0
