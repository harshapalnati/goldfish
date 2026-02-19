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

const EMBEDDING_DIM: usize = 384;
const SUBWORD_DIMS: usize = 256;
const TOKEN_DIMS: usize = 96;
const GLOBAL_DIMS: usize = 32;
const GLOBAL_OFFSET: usize = SUBWORD_DIMS + TOKEN_DIMS;
const ALPHABET_SIZE: usize = 39;
const START_BUCKET: usize = 36;
const END_BUCKET: usize = 37;
const OTHER_BUCKET: usize = 38;

fn char_bucket(c: char) -> usize {
    match c {
        'a'..='z' => (c as u8 - b'a') as usize,
        '0'..='9' => 26 + (c as u8 - b'0') as usize,
        '^' => START_BUCKET,
        '$' => END_BUCKET,
        _ => OTHER_BUCKET,
    }
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "a"
            | "an"
            | "the"
            | "and"
            | "or"
            | "but"
            | "to"
            | "of"
            | "for"
            | "with"
            | "in"
            | "on"
            | "at"
            | "by"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "am"
            | "do"
            | "does"
            | "did"
            | "that"
            | "this"
            | "it"
            | "as"
    )
}

fn normalize_token(raw: &str) -> Option<String> {
    let mut token = raw
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();

    if token.len() < 2 {
        return None;
    }

    // Light stemming.
    for suffix in ["ing", "ed", "es", "ly", "s"] {
        if token.len() > 4 && token.ends_with(suffix) {
            token.truncate(token.len() - suffix.len());
            break;
        }
    }

    // Small canonicalization map for common paraphrases.
    let canonical = match token.as_str() {
        "like" | "love" | "prefer" | "favorite" => "prefer",
        "goal" | "aim" | "objective" => "goal",
        "build" | "create" | "make" => "build",
        "fix" | "repair" | "resolve" => "fix",
        "bug" | "issue" | "defect" => "bug",
        "quick" | "rapid" => "fast",
        "assistant" | "bot" => "agent",
        "coding" | "programming" => "code",
        _ => token.as_str(),
    };

    if is_stopword(canonical) {
        None
    } else {
        Some(canonical.to_string())
    }
}

fn token_signature(token: &str) -> usize {
    let mut chars = token.chars();
    let c1 = chars.next().map(char_bucket).unwrap_or(OTHER_BUCKET);
    let c2 = chars.next().map(char_bucket).unwrap_or(OTHER_BUCKET);
    let last = token
        .chars()
        .next_back()
        .map(char_bucket)
        .unwrap_or(OTHER_BUCKET);
    let penultimate = token
        .chars()
        .rev()
        .nth(1)
        .map(char_bucket)
        .unwrap_or(OTHER_BUCKET);
    let len = token.chars().count().min(31);

    ((((c1 * ALPHABET_SIZE + c2) * ALPHABET_SIZE + penultimate) * ALPHABET_SIZE + last) * 32)
        + len
}

/// Generate a deterministic subword/text embedding.
pub fn generate_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0f32; EMBEDDING_DIM];
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(normalize_token)
        .collect();

    if tokens.is_empty() {
        return embedding;
    }

    // Subword trigram features.
    for token in &tokens {
        let mut buckets = Vec::with_capacity(token.len() + 2);
        buckets.push(START_BUCKET);
        buckets.extend(token.chars().map(char_bucket));
        buckets.push(END_BUCKET);

        for tri in buckets.windows(3) {
            let raw_index = (tri[0] * ALPHABET_SIZE + tri[1]) * ALPHABET_SIZE + tri[2];
            let folded = raw_index % SUBWORD_DIMS;
            let sign = if ((raw_index / SUBWORD_DIMS) & 1) == 0 {
                1.0
            } else {
                -1.0
            };
            embedding[folded] += sign;
        }

        let token_idx = SUBWORD_DIMS + (token_signature(token) % TOKEN_DIMS);
        embedding[token_idx] += 1.0;
    }

    // Token bigram features.
    for pair in tokens.windows(2) {
        let left = token_signature(&pair[0]);
        let right = token_signature(&pair[1]);
        let idx = SUBWORD_DIMS + ((left.wrapping_mul(41) + right) % TOKEN_DIMS);
        embedding[idx] += 1.35;
    }

    // Global shape features.
    let token_count = tokens.len() as f32;
    let avg_len = tokens.iter().map(|t| t.len() as f32).sum::<f32>() / token_count;
    let digit_ratio = text.chars().filter(|c| c.is_ascii_digit()).count() as f32
        / text.chars().count().max(1) as f32;
    let uppercase_ratio = text.chars().filter(|c| c.is_ascii_uppercase()).count() as f32
        / text.chars().count().max(1) as f32;

    embedding[GLOBAL_OFFSET] = token_count / 64.0;
    embedding[GLOBAL_OFFSET + 1] = avg_len / 16.0;
    embedding[GLOBAL_OFFSET + 2] = digit_ratio;
    embedding[GLOBAL_OFFSET + 3] = uppercase_ratio;
    embedding[GLOBAL_OFFSET + 4] = (text.matches('?').count() as f32) / 8.0;
    embedding[GLOBAL_OFFSET + 5] = (text.matches('!').count() as f32) / 8.0;
    embedding[GLOBAL_OFFSET + 6] = (text.matches(':').count() as f32) / 8.0;
    embedding[GLOBAL_OFFSET + 7] = (text.matches('.').count() as f32) / 16.0;

    for (i, token) in tokens.iter().enumerate() {
        let idx = GLOBAL_OFFSET + 8 + (i % (GLOBAL_DIMS - 8));
        embedding[idx] += (token.len() as f32).ln_1p() * 0.1;
    }

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

    #[test]
    fn test_embedding_dimension_and_nonzero() {
        let v = generate_embedding("I prefer concise technical answers.");
        assert_eq!(v.len(), EMBEDDING_DIM);
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.0);
    }

    #[test]
    fn test_embedding_similarity_signal() {
        let a = generate_embedding("I like clean code and quick iteration");
        let b = generate_embedding("I prefer clean coding and rapid iterations");
        let c = generate_embedding("The weather forecast predicts heavy rain");

        let sim_ab = cosine_similarity(&a, &b);
        let sim_ac = cosine_similarity(&a, &c);
        assert!(sim_ab > sim_ac);
    }
}
