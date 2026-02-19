# Installation Guide

Complete installation instructions for Goldfish.

## Requirements

### Minimum Requirements

- **Rust**: 1.88.0 or later
- **Disk Space**: 100MB for embedding model + data
- **RAM**: 512MB minimum, 2GB recommended
- **OS**: Linux, macOS, Windows

### Recommended for Production

- **Rust**: Latest stable
- **Disk Space**: 1GB+
- **RAM**: 4GB+
- **OS**: Linux (Ubuntu 22.04 LTS recommended)

## Installing Rust

If you don't have Rust installed:

```bash
# Install via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload shell
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show 1.88.0 or later
```

Update Rust:

```bash
rustup update
```

## Installing Goldfish

### From crates.io

Add to your `Cargo.toml`:

```toml
[dependencies]
goldfish = "0.1"
```

### With Features

```toml
# Basic (built-in storage only)
[dependencies]
goldfish = "0.1"

# With connectors
[dependencies]
goldfish = { version = "0.1", features = ["pinecone", "postgres"] }

# All features
[dependencies]
goldfish = { version = "0.1", features = ["all-connectors"] }
```

### From Git

```toml
[dependencies]
goldfish = { git = "https://github.com/harshapalnati/goldfish" }
```

### From Source

```bash
# Clone repository
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish

# Build
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Built-in SQLite + LanceDB | minimal |
| `pinecone` | Pinecone connector | reqwest |
| `postgres` | PostgreSQL + pgvector | sqlx, pgvector |
| `redis` | Redis connector | redis |
| `chromadb` | ChromaDB connector (stub) | reqwest |
| `qdrant` | Qdrant connector (stub) | reqwest |
| `mongodb` | MongoDB connector (stub) | mongodb |
| `weaviate` | Weaviate connector (stub) | reqwest |
| `milvus` | Milvus connector (stub) | reqwest |
| `all-connectors` | All connector features | all above |

## System Dependencies

### Linux (Ubuntu/Debian)

```bash
# Required for TLS support
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config

# Optional: For PostgreSQL support
sudo apt-get install -y libpq-dev

# Optional: For SQLite FTS5 extension
sudo apt-get install -y libsqlite3-dev
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install openssl-devel pkgconfig
```

### macOS

```bash
# Using Homebrew
brew install openssl pkg-config

# Set environment variables if needed
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

### Windows

Install Visual Studio Build Tools or Visual Studio Community.

No additional dependencies required for the built-in storage.

## Database Setup (Optional)

### PostgreSQL with pgvector

1. **Install PostgreSQL**:

```bash
# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib

# macOS
brew install postgresql
brew services start postgresql

# Docker
docker run -d --name postgres \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  pgvector/pgvector:pg16
```

2. **Install pgvector**:

```bash
# If not using Docker image above
sudo apt-get install postgresql-16-pgvector

# Or compile from source
git clone --branch v0.6.0 https://github.com/pgvector/pgvector.git
cd pgvector
make
sudo make install
```

3. **Create database**:

```bash
sudo -u postgres psql -c "CREATE DATABASE agent_memory;"
sudo -u postgres psql -c "CREATE USER agent WITH PASSWORD 'password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE agent_memory TO agent;"
```

4. **Enable extension**:

```bash
psql -U agent -d agent_memory -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

### Redis

```bash
# Ubuntu/Debian
sudo apt-get install redis-server
sudo systemctl start redis

# macOS
brew install redis
brew services start redis

# Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine
```

### Pinecone

1. **Create account** at https://www.pinecone.io/
2. **Create an index**:
   - Name: `memories`
   - Dimensions: 384
   - Metric: Cosine
3. **Get API key** from dashboard
4. **Set environment variable**:

```bash
export PINECONE_API_KEY="your-api-key"
```

## Verification

### Test Installation

Create a test file:

```rust
// test.rs
use agent_memory::{Memory, MemorySystem, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let memory = MemorySystem::new("./test_data").await?;
    let fact = Memory::new("Test", MemoryType::Fact);
    memory.save(&fact).await?;
    println!("✓ Goldfish installed successfully!");
    Ok(())
}
```

Compile and run:

```bash
rustc --edition 2024 test.rs -L target/debug/deps --extern agent_memory=target/debug/libagent_memory.rlib
./test
```

Or with Cargo:

```bash
cargo new test-project
cd test-project
# Add goldfish to Cargo.toml
cargo run
```

### Check Features

```bash
# Check what features are available
cargo build --features all-connectors

# Check specific feature
cargo build --features pinecone
```

## Troubleshooting

### Common Issues

**Issue: `linker 'cc' not found`**

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**Issue: `could not find system library 'openssl'`**

```bash
# macOS
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"

# Ubuntu
sudo apt-get install libssl-dev pkg-config
```

**Issue: `failed to run custom build command for onnxruntime-sys`**

This is from the fastembed dependency. Usually resolves by:

```bash
# Clean and rebuild
cargo clean
cargo build
```

Or disable default features and use without embeddings:

```toml
[dependencies]
goldfish = { version = "0.1", default-features = false }
```

**Issue: Slow first search**

This is expected! The embedding model downloads on first use (~50MB). Subsequent operations will be fast.

**Issue: Out of disk space**

```bash
# Check space
df -h

# Clean cargo cache if needed
cargo cache --autoclean

# Or manually remove
cargo clean
rm -rf ~/.cargo/registry/cache
```

**Issue: Permission denied on data directory**

```bash
# Fix permissions
chmod 755 ./data
# Or use a different directory
let memory = MemorySystem::new("/path/with/permissions").await?;
```

## Docker Installation

### Using Docker

```dockerfile
FROM rust:1.88-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/my-agent /usr/local/bin/
CMD ["my-agent"]
```

Build and run:

```bash
docker build -t my-agent .
docker run -v $(pwd)/data:/app/data my-agent
```

### Docker Compose

```yaml
version: '3.8'
services:
  app:
    build: .
    volumes:
      - ./data:/app/data
    environment:
      - PINECONE_API_KEY=${PINECONE_API_KEY}
    depends_on:
      - postgres
      - redis

  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_PASSWORD: password
      POSTGRES_DB: agent_memory
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"

volumes:
  postgres_data:
  redis_data:
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Start building
- [Configuration](configuration.md) - Advanced configuration
- [Connectors](../connectors/README.md) - Database connectors
- [Examples](../../examples/README.md) - Example projects

## Getting Help

- 📖 [Documentation](https://docs.rs/goldfish)
- 🐛 [Issues](https://github.com/harshapalnati/goldfish/issues)
- 💬 [Discussions](https://github.com/harshapalnati/goldfish/discussions)
