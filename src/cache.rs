//! # Caching Layer - Simple in-memory LRU cache

use crate::error::{MemoryError, Result};
use crate::types::MemoryId;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unique cache key
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    pub fn memory(id: &MemoryId) -> Self {
        Self(format!("memory:{}", id))
    }
    
    pub fn search(query: &str) -> Self {
        Self(format!("search:{}", query))
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            default_ttl: Duration::minutes(30),
        }
    }
}

/// L1 in-memory cache
pub struct L1Cache {
    entries: RwLock<HashMap<CacheKey, Vec<u8>>>,
    config: CacheConfig,
    stats: RwLock<CacheStats>,
}

impl L1Cache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
            stats: RwLock::new(CacheStats::default()),
        }
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> Option<T> {
        let entries = self.entries.read().await;
        if let Some(data) = entries.get(key) {
            if let Ok(value) = bincode::deserialize(data) {
                self.stats.write().await.hits += 1;
                return Some(value);
            }
        }
        self.stats.write().await.misses += 1;
        None
    }

    pub async fn put<T: Serialize>(&self, key: CacheKey, value: &T) -> Result<()> {
        let data = bincode::serialize(value)
            .map_err(|e| MemoryError::Serialization(e.to_string()))?;
        
        let mut entries = self.entries.write().await;
        
        if entries.len() >= self.config.max_entries {
            self.stats.write().await.evictions += 1;
            if let Some(first) = entries.keys().next().cloned() {
                entries.remove(&first);
            }
        }
        
        entries.insert(key, data);
        Ok(())
    }

    pub async fn invalidate(&self, key: &CacheKey) -> Result<bool> {
        let mut entries = self.entries.write().await;
        Ok(entries.remove(key).is_some())
    }

    pub async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }

    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
}

/// Main cache manager
pub struct CacheManager {
    l1: L1Cache,
}

impl CacheManager {
    pub async fn new(config: CacheConfig) -> Result<Self> {
        Ok(Self {
            l1: L1Cache::new(config),
        })
    }

    pub async fn get<T: for<'de> Deserialize<'de> + Clone>(&self, key: &CacheKey) -> Option<T> {
        self.l1.get(key).await
    }

    pub async fn put<T: Serialize>(&self, key: CacheKey, value: &T) -> Result<()> {
        self.l1.put(key, value).await
    }

    pub async fn invalidate(&self, key: &CacheKey) -> Result<bool> {
        self.l1.invalidate(key).await
    }

    pub async fn stats(&self) -> CacheStats {
        self.l1.stats().await
    }

    pub async fn clear(&self) -> Result<()> {
        self.l1.clear().await
    }
}

/// Cache operations wrapper
pub struct CachedMemoryOperations {
    cache: Arc<CacheManager>,
}

impl CachedMemoryOperations {
    pub fn new(cache: Arc<CacheManager>) -> Self {
        Self { cache }
    }

    pub fn cache(&self) -> &Arc<CacheManager> {
        &self.cache
    }
}

/// Configuration builder
#[derive(Debug, Default)]
pub struct CacheConfigBuilder {
    config: CacheConfig,
}

impl CacheConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn max_entries(mut self, count: usize) -> Self {
        self.config.max_entries = count;
        self
    }
    
    pub fn build(self) -> CacheConfig {
        self.config
    }
}
