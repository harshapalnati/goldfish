use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn dimension(&self) -> usize;

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Corpus statistics for TF-IDF calculation
#[derive(Debug, Clone, Default)]
pub struct CorpusStats {
    /// Inverse document frequency for each term
    pub idf: HashMap<String, f32>,
    /// Total number of documents in corpus
    pub total_docs: usize,
}

impl CorpusStats {
    /// Build IDF map from a collection of documents
    pub fn from_documents(docs: &[String]) -> Self {
        use std::collections::HashSet;
        
        let total_docs = docs.len();
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        
        // Count document frequency for each term
        for doc in docs {
            let tokens: HashSet<String> = doc
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|t| t.len() > 2) // Skip short tokens
                .map(|s| s.to_string())
                .collect();
            
            for token in tokens {
                *doc_freq.entry(token).or_insert(0) += 1;
            }
        }
        
        // Calculate IDF: log(N / df)
        let idf: HashMap<String, f32> = doc_freq
            .into_iter()
            .map(|(term, df)| {
                let idf_val = ((total_docs as f32) / (df as f32)).ln().max(1.0);
                (term, idf_val)
            })
            .collect();
        
        Self { idf, total_docs }
    }
    
    /// Get IDF for a term (default 1.0 if not in corpus)
    pub fn idf(&self, term: &str) -> f32 {
        self.idf.get(term).copied().unwrap_or(1.0)
    }
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
    
    /// Generate TF-IDF weighted embedding
    /// This captures term importance without requiring a learned model
    pub fn embed_tfidf(&self, text: &str, corpus_stats: &CorpusStats) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dimension];
        
        // Tokenize
        let text_lower = text.to_lowercase();
        let tokens: Vec<&str> = text_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2) // Skip short tokens
            .collect();
        
        if tokens.is_empty() {
            return vec;
        }
        
        // Count term frequencies
        let mut tf: HashMap<&str, f32> = HashMap::new();
        for token in &tokens {
            *tf.entry(*token).or_insert(0.0) += 1.0;
        }
        
        // Apply TF-IDF weighting and hash to buckets
        let total_terms = tokens.len() as f32;
        for (token, count) in tf {
            let term_freq = count / total_terms;
            let idf = corpus_stats.idf(token);
            
            // Hash token to bucket (same as regular embedding)
            let mut hash = 1469598103934665603u64;
            for b in token.as_bytes() {
                hash ^= *b as u64;
                hash = hash.wrapping_mul(1099511628211u64);
            }
            let idx = (hash as usize) % self.dimension;
            
            // Accumulate TF-IDF weighted value
            vec[idx] += term_freq * idf;
        }
        
        // L2 normalize
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
