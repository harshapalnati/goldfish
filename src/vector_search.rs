//! Vector search integration for semantic similarity

use crate::error::{MemoryError, Result};
use crate::types::MemoryId;

/// Configuration for vector search
#[derive(Debug, Clone)]
pub struct VectorSearchConfig {
    pub dimension: usize,
    pub index_path: std::path::PathBuf,
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            dimension: 384, // Default for all-MiniLM-L6-v2
            index_path: std::path::PathBuf::from("./vector_index"),
        }
    }
}

/// A vector index for semantic search
pub struct VectorIndex {
    config: VectorSearchConfig,
}

impl VectorIndex {
    pub fn new(config: VectorSearchConfig) -> Self {
        Self { config }
    }

    pub async fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config.index_path).map_err(|e| {
            MemoryError::Storage(format!("Failed to create vector index dir: {}", e))
        })?;
        Ok(())
    }

    /// Store a vector for a memory
    pub async fn store(&self, memory_id: &MemoryId, embedding: Vec<f32>) -> Result<()> {
        // For now, store in a simple file-based index
        // In production, this would use LanceDB or similar
        let index_file = self.config.index_path.join(format!("{}.bin", memory_id));
        let data = bincode::serialize(&embedding)
            .map_err(|e| MemoryError::Serialization(e.to_string()))?;
        tokio::fs::write(&index_file, data)
            .await
            .map_err(|e| MemoryError::Storage(format!("Failed to write vector: {}", e)))?;
        Ok(())
    }

    /// Search for similar vectors using cosine similarity
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        let mut results = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.config.index_path)
            .await
            .map_err(|e| MemoryError::Storage(format!("Failed to read index: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| MemoryError::Storage(format!("Failed to read entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                let memory_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let data = tokio::fs::read(&path)
                    .await
                    .map_err(|e| MemoryError::Storage(format!("Failed to read vector: {}", e)))?;

                let embedding: Vec<f32> = bincode::deserialize(&data)
                    .map_err(|e| MemoryError::Serialization(e.to_string()))?;

                let similarity = cosine_similarity(query_embedding, &embedding);
                results.push((memory_id, similarity));
            }
        }

        // Sort by similarity (highest first) and take limit
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    /// Delete a vector
    pub async fn delete(&self, memory_id: &MemoryId) -> Result<()> {
        let index_file = self.config.index_path.join(format!("{}.bin", memory_id));
        if index_file.exists() {
            tokio::fs::remove_file(&index_file)
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to delete vector: {}", e)))?;
        }
        Ok(())
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Generate a simple embedding (placeholder for real embedding model)
pub fn generate_embedding(text: &str) -> Vec<f32> {
    // This is a placeholder implementation
    // In production, use a real embedding model like:
    // - sentence-transformers/all-MiniLM-L6-v2 (384 dim)
    // - OpenAI text-embedding-ada-002 (1536 dim)
    // - Or fastembed crate

    // Simple hash-based embedding for demonstration
    let mut embedding = vec![0.0f32; 384];
    let bytes = text.as_bytes();

    for (i, &byte) in bytes.iter().enumerate() {
        let idx = i % 384;
        embedding[idx] += (byte as f32) / 255.0;
    }

    // Normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        embedding.iter_mut().for_each(|x| *x /= norm);
    }

    embedding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }
}
