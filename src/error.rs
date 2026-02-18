//! Error types for Goldfish

use thiserror::Error;

/// Main error type for the memory system
#[derive(Error, Debug)]
pub enum MemoryError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Vector database error
    #[error("Vector database error: {0}")]
    VectorDb(String),

    /// Embedding generation failed
    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),

    /// Memory not found
    #[error("Memory not found: {0}")]
    NotFound(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Search index error (Tantivy)
    #[error("Search index error: {0}")]
    SearchIndex(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other errors
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, MemoryError>;
