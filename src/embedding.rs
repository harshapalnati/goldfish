use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn dimension(&self) -> usize;

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Zero-config embedding provider.
///
/// This is intentionally lightweight and deterministic (no network, no model downloads).
/// It is *not* intended to match the semantic quality of a learned embedding model.
#[derive(Debug, Clone)]
pub struct HashEmbeddingProvider {
    dimension: usize,
}

impl HashEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    fn embed_one(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dimension];
        let mut token_count = 0u32;

        for token in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
        {
            token_count += 1;
            let mut hash = 1469598103934665603u64;
            for b in token.as_bytes() {
                hash ^= *b as u64;
                hash = hash.wrapping_mul(1099511628211u64);
            }

            let idx = (hash as usize) % self.dimension;
            vec[idx] += 1.0;
        }

        if token_count == 0 {
            return vec;
        }

        let norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }

        vec
    }
}

#[async_trait]
impl EmbeddingProvider for HashEmbeddingProvider {
    fn name(&self) -> &'static str {
        "hash"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.embed_one(t)).collect())
    }
}
