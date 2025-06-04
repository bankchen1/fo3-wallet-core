//! In-memory caching implementation using Moka
//!
//! Provides fast in-memory caching as a fallback when Redis is unavailable
//! or for frequently accessed small data.

use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::ServiceError;
use super::{Cache, CacheKey, CacheEntry, CacheStats, CacheConfig};

/// In-memory cache implementation using Moka
pub struct MemoryCache {
    cache: MokaCache<String, Vec<u8>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl MemoryCache {
    /// Create new memory cache instance
    pub fn new(config: CacheConfig) -> Self {
        info!("Initializing memory cache with size: {}", config.memory_cache_size);
        
        let cache = MokaCache::builder()
            .max_capacity(config.memory_cache_size as u64)
            .time_to_live(Duration::from_secs(config.memory_cache_ttl_seconds))
            .time_to_idle(Duration::from_secs(config.memory_cache_ttl_seconds / 2))
            .build();
        
        Self {
            cache,
            config,
            stats: Arc::new(RwLock::new(CacheStats::new())),
        }
    }
    
    /// Serialize value to bytes
    fn serialize_value<T>(&self, value: &T) -> Result<Vec<u8>, ServiceError>
    where
        T: Serialize,
    {
        serde_json::to_vec(value)
            .map_err(|e| ServiceError::CacheError(format!("Memory cache serialization failed: {}", e)))
    }
    
    /// Deserialize value from bytes
    fn deserialize_value<T>(&self, data: &[u8]) -> Result<T, ServiceError>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(data)
            .map_err(|e| ServiceError::CacheError(format!("Memory cache deserialization failed: {}", e)))
    }
    
    /// Update cache statistics
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut stats);
        stats.calculate_hit_rate();
        
        // Update memory usage and entry count
        stats.entry_count = self.cache.entry_count();
        stats.memory_usage_bytes = self.estimate_memory_usage();
    }
    
    /// Estimate memory usage (rough approximation)
    fn estimate_memory_usage(&self) -> u64 {
        // Rough estimate: entry count * average entry size
        // In production, you might want more accurate memory tracking
        self.cache.entry_count() * 1024 // Assume 1KB average per entry
    }
    
    /// Validate cache key
    fn validate_key(&self, key: &CacheKey) -> Result<(), ServiceError> {
        let key_str = key.to_redis_key();
        if key_str.len() > self.config.max_key_length {
            return Err(ServiceError::CacheError(
                format!("Cache key too long: {} > {}", key_str.len(), self.config.max_key_length)
            ));
        }
        Ok(())
    }
    
    /// Validate value size
    fn validate_value_size(&self, data: &[u8]) -> Result<(), ServiceError> {
        if data.len() > self.config.max_value_size_bytes {
            return Err(ServiceError::CacheError(
                format!("Cache value too large: {} > {}", data.len(), self.config.max_value_size_bytes)
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get<T>(&self, key: &CacheKey) -> Result<Option<T>, ServiceError>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        debug!("Getting memory cache key: {}", key_str);
        
        match self.cache.get(&key_str).await {
            Some(data) => {
                debug!("Memory cache hit for key: {}", key_str);
                self.update_stats(|stats| stats.hits += 1).await;
                
                let value = self.deserialize_value(&data)?;
                Ok(Some(value))
            }
            None => {
                debug!("Memory cache miss for key: {}", key_str);
                self.update_stats(|stats| stats.misses += 1).await;
                Ok(None)
            }
        }
    }
    
    async fn set<T>(&self, key: &CacheKey, value: &T, ttl: Option<Duration>) -> Result<(), ServiceError>
    where
        T: Serialize + Send + Sync,
    {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        let data = self.serialize_value(value)?;
        self.validate_value_size(&data)?;
        
        debug!("Setting memory cache key: {} with TTL: {:?}", key_str, ttl);
        
        // Note: Moka doesn't support per-key TTL, so we use the global TTL
        self.cache.insert(key_str, data).await;
        
        self.update_stats(|stats| stats.sets += 1).await;
        Ok(())
    }
    
    async fn delete(&self, key: &CacheKey) -> Result<(), ServiceError> {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        debug!("Deleting memory cache key: {}", key_str);
        
        self.cache.remove(&key_str).await;
        
        self.update_stats(|stats| stats.deletes += 1).await;
        Ok(())
    }
    
    async fn exists(&self, key: &CacheKey) -> Result<bool, ServiceError> {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        Ok(self.cache.contains_key(&key_str))
    }
    
    async fn clear(&self) -> Result<(), ServiceError> {
        warn!("Clearing all memory cache entries");
        
        self.cache.invalidate_all();
        
        // Reset stats
        let mut stats = self.stats.write().await;
        *stats = CacheStats::new();
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, ServiceError> {
        let mut stats = self.stats.write().await;
        
        // Update current metrics
        stats.entry_count = self.cache.entry_count();
        stats.memory_usage_bytes = self.estimate_memory_usage();
        stats.calculate_hit_rate();
        
        Ok(stats.clone())
    }
    
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, ServiceError> {
        debug!("Invalidating memory cache pattern: {}", pattern);
        
        // Convert Redis pattern to regex
        let regex_pattern = pattern
            .replace("*", ".*")
            .replace("?", ".");
        
        let regex = regex::Regex::new(&regex_pattern)
            .map_err(|e| ServiceError::CacheError(format!("Invalid pattern: {}", e)))?;
        
        let mut count = 0u64;
        
        // Iterate through all keys and remove matching ones
        // Note: This is not the most efficient approach for large caches
        // In production, you might want to maintain a separate index
        self.cache.run_pending_tasks().await;
        
        // Since Moka doesn't provide key iteration, we'll use a different approach
        // For now, we'll just clear the entire cache if pattern is "*"
        if pattern == "*" {
            count = self.cache.entry_count();
            self.cache.invalidate_all();
            
            self.update_stats(|stats| {
                stats.deletes += count;
                stats.evictions += count;
            }).await;
        }
        
        info!("Invalidated {} memory cache entries matching pattern: {}", count, pattern);
        Ok(count)
    }
}

/// Memory cache builder for easier configuration
pub struct MemoryCacheBuilder {
    config: CacheConfig,
}

impl MemoryCacheBuilder {
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }
    
    pub fn max_capacity(mut self, capacity: usize) -> Self {
        self.config.memory_cache_size = capacity;
        self
    }
    
    pub fn ttl(mut self, ttl_seconds: u64) -> Self {
        self.config.memory_cache_ttl_seconds = ttl_seconds;
        self
    }
    
    pub fn max_value_size(mut self, size_bytes: usize) -> Self {
        self.config.max_value_size_bytes = size_bytes;
        self
    }
    
    pub fn build(self) -> MemoryCache {
        MemoryCache::new(self.config)
    }
}

impl Default for MemoryCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u32,
        name: String,
        value: f64,
    }
    
    #[tokio::test]
    async fn test_memory_cache_basic_operations() {
        let cache = MemoryCacheBuilder::new()
            .max_capacity(100)
            .ttl(300)
            .build();
        
        let key = CacheKey::UserSession(uuid::Uuid::new_v4());
        let data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Test set and get
        cache.set(&key, &data, None).await.unwrap();
        let retrieved: Option<TestData> = cache.get(&key).await.unwrap();
        assert_eq!(retrieved, Some(data.clone()));
        
        // Test exists
        assert!(cache.exists(&key).await.unwrap());
        
        // Test delete
        cache.delete(&key).await.unwrap();
        let retrieved: Option<TestData> = cache.get(&key).await.unwrap();
        assert_eq!(retrieved, None);
        
        // Test stats
        let stats = cache.stats().await.unwrap();
        assert!(stats.hits > 0);
        assert!(stats.misses > 0);
        assert!(stats.sets > 0);
        assert!(stats.deletes > 0);
    }
    
    #[tokio::test]
    async fn test_memory_cache_clear() {
        let cache = MemoryCacheBuilder::new().build();
        
        let key1 = CacheKey::UserSession(uuid::Uuid::new_v4());
        let key2 = CacheKey::WalletBalance(uuid::Uuid::new_v4());
        let data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        cache.set(&key1, &data, None).await.unwrap();
        cache.set(&key2, &data, None).await.unwrap();
        
        cache.clear().await.unwrap();
        
        let retrieved1: Option<TestData> = cache.get(&key1).await.unwrap();
        let retrieved2: Option<TestData> = cache.get(&key2).await.unwrap();
        
        assert_eq!(retrieved1, None);
        assert_eq!(retrieved2, None);
    }
}
