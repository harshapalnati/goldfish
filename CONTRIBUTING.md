# Contributing to Goldfish

Thank you for your interest in contributing to Goldfish! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [How to Contribute](#how-to-contribute)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code:

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Respect differing viewpoints

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a branch** for your changes
4. **Make your changes** following our guidelines
5. **Submit a pull request**

## Development Setup

### Prerequisites

- Rust 1.88.0 or later
- Git
- Docker (for running integration tests)

### Setup Steps

```bash
# Clone the repository
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish

# Build the project
cargo build

# Run tests
cargo test

# Run with all features
cargo build --features all-connectors
cargo test --features all-connectors

# Run benchmarks
cargo bench

# Check code formatting
cargo fmt -- --check

# Run clippy
cargo clippy --all-features
```

## Project Structure

```
goldfish/
├── src/                    # Source code
│   ├── lib.rs             # Library entry point
│   ├── types.rs           # Core types (Memory, Association, etc.)
│   ├── store.rs           # SQLite storage
│   ├── lance.rs           # LanceDB integration
│   ├── embedding.rs       # Embedding generation
│   ├── search.rs          # Hybrid search
│   ├── maintenance.rs     # Decay and pruning
│   ├── error.rs           # Error types
│   └── connectors/        # Database connectors
│       ├── mod.rs
│       ├── traits.rs
│       ├── error.rs
│       ├── pinecone.rs
│       ├── postgres.rs
│       └── ...
├── examples/              # Example applications
│   ├── basic/            # Basic usage
│   ├── complete/         # Complete applications
│   └── connectors/       # Connector examples
├── tests/                # Tests
│   ├── unit/            # Unit tests
│   └── integration/     # Integration tests
├── benches/             # Benchmarks
├── docs/                # Documentation
├── migrations/          # Database migrations
└── scripts/            # Utility scripts
```

## How to Contribute

### Reporting Bugs

Before creating a bug report, please:

1. Check if the issue already exists
2. Update to the latest version to see if it's fixed
3. Try to isolate the problem

When reporting bugs, include:

- **Title**: Clear and descriptive
- **Description**: What happened vs. what you expected
- **Steps to reproduce**: Minimal example
- **Environment**: OS, Rust version, dependencies
- **Logs**: Relevant error messages or logs

### Suggesting Enhancements

Enhancement suggestions are welcome! Please:

1. Check if the enhancement is already suggested
2. Provide a clear use case
3. Explain why it would be useful

### Adding Connectors

To add a new database connector:

1. Create `src/connectors/<name>.rs`
2. Implement `VectorStore` and/or `MetadataStore` traits
3. Add feature flag in `Cargo.toml`
4. Add documentation in `docs/connectors/`
5. Add example in `examples/connectors/`
6. Add tests

Example connector structure:

```rust
use crate::connectors::traits::{VectorStore, VectorFilter, VectorSearchResult};
use crate::connectors::error::{ConnectorError, ConnectorResult};
use async_trait::async_trait;

pub struct MyConnector {
    // Configuration
}

pub struct MyConfig {
    // Configuration fields
}

#[async_trait]
impl VectorStore for MyConnector {
    // Implement all required methods
}
```

## Coding Standards

### Rust Style Guide

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- Use `cargo fmt` for formatting
- Follow naming conventions
- Document all public items
- Use meaningful variable names
- Keep functions focused and small

### Code Quality

- **Clippy**: All code must pass clippy lints
- **Documentation**: All public APIs must be documented
- **Tests**: New code should have tests
- **Error Handling**: Use `thiserror` for error types
- **Async**: Use `async-trait` for async traits

### Example

```rust
/// Stores a memory in the system.
///
/// # Arguments
///
/// * `memory` - The memory to store
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if storage fails.
///
/// # Examples
///
/// ```rust,no_run
/// use agent_memory::{Memory, MemoryType};
///
/// # async fn example() -> anyhow::Result<()> {
/// let memory = Memory::new("Content", MemoryType::Fact);
/// memory_system.save(&memory).await?;
/// # Ok(())
/// # }
/// ```
pub async fn save(&self, memory: &Memory) -> Result<()> {
    // Implementation
}
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with all features
cargo test --features all-connectors

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration

# Run with logging
cargo test -- --nocapture

# Generate coverage
cargo tarpaulin --out Html
```

### Writing Tests

**Unit tests** go in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_memory() {
        let memory = MemorySystem::new_in_memory().await.unwrap();
        let fact = Memory::new("Test", MemoryType::Fact);
        memory.save(&fact).await.unwrap();
        
        let loaded = memory.load(&fact.id).await.unwrap();
        assert!(loaded.is_some());
    }
}
```

**Integration tests** go in `tests/integration/`:

```rust
// tests/integration/postgres.rs
#[tokio::test]
async fn test_postgres_connector() {
    // Test full workflow
}
```

### Test Guidelines

- Test both success and failure cases
- Use `MemorySystem::new_in_memory()` for unit tests
- Use Docker for integration tests with real databases
- Mock external services when appropriate
- Keep tests deterministic

## Documentation

### Code Documentation

- Document all public items with `///`
- Include examples in doc comments
- Use `# Arguments`, `# Returns`, `# Examples` sections
- Document panics and errors

### User Documentation

Documentation is in the `docs/` directory:

- `docs/guides/` - How-to guides
- `docs/architecture/` - Architecture documentation
- `docs/api/` - API reference
- `docs/connectors/` - Connector documentation

### Documentation Guidelines

- Use clear, concise language
- Include code examples
- Keep guides focused on one topic
- Update docs when code changes

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test changes
- `chore`: Build process, dependencies

**Examples:**

```
feat(connectors): add Pinecone connector

Implement full Pinecone vector store connector with:
- Vector storage and retrieval
- Similarity search
- Metadata filtering
- Health checks

fix(search): correct RRF score calculation

docs(api): add examples to MemorySystem methods

refactor(store): optimize SQLite queries
```

## Pull Request Process

1. **Create a branch**: `git checkout -b feature/my-feature`
2. **Make changes**: Follow coding standards
3. **Add tests**: Ensure coverage
4. **Update docs**: If needed
5. **Run checks**:
   ```bash
   cargo fmt -- --check
   cargo clippy --all-features
   cargo test --all-features
   ```
6. **Commit**: Follow commit message guidelines
7. **Push**: `git push origin feature/my-feature`
8. **Create PR**: On GitHub with clear description

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Refactoring
- [ ] Performance improvement

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests pass
- [ ] No breaking changes (or documented)
```

### Review Process

1. Automated checks must pass
2. At least one maintainer review
3. Address review comments
4. Squash commits if requested
5. Maintainer merges

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v0.x.x`
4. Push tag: `git push origin v0.x.x`
5. GitHub Actions creates release
6. Publish to crates.io: `cargo publish`

## Questions?

- Open an issue for questions
- Join our Discord (coming soon)
- Email: contribute@goldfish.dev

## Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Added to the project readme (significant contributions)

Thank you for contributing! 🎉
