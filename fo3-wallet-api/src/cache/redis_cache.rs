//! Redis-based caching implementation
//!
//! Provides high-performance Redis caching with connection pooling, compression,
//! and comprehensive error handling.

use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::{AsyncCommands, RedisResult};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::error::ServiceError;
use super::{Cache, CacheKey, CacheEntry, CacheStats, CacheConfig};

/// Redis cache implementation
pub struct RedisCache {
    pool: Pool,
    config: CacheConfig,
    stats: std::sync::Arc<tokio::sync::RwLock<CacheStats>>,
}

impl RedisCache {
    /// Create new Redis cache instance
    pub async fn new(config: CacheConfig) -> Result<Self, ServiceError> {
        info!("Initializing Redis cache with URL: {}", config.redis_url);
        
        let redis_config = Config::from_url(&config.redis_url)
            .map_err(|e| ServiceError::CacheError(format!("Invalid Redis URL: {}", e)))?;
        
        let pool = redis_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| ServiceError::CacheError(format!("Failed to create Redis pool: {}", e)))?;
        
        // Test connection
        let mut conn = pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let _: String = conn.ping().await
            .map_err(|e| ServiceError::CacheError(format!("Redis ping failed: {}", e)))?;
        
        info!("Redis cache initialized successfully");
        
        Ok(Self {
            pool,
            config,
            stats: std::sync::Arc::new(tokio::sync::RwLock::new(CacheStats::new())),
        })
    }
    
    /// Serialize value with optional compression
    fn serialize_value<T>(&self, value: &T) -> Result<Vec<u8>, ServiceError>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| ServiceError::CacheError(format!("Serialization failed: {}", e)))?;
        
        if self.config.enable_compression && serialized.len() > 1024 {
            // Use simple compression for large values
            self.compress_data(&serialized)
        } else {
            Ok(serialized)
        }
    }
    
    /// Deserialize value with optional decompression
    fn deserialize_value<T>(&self, data: &[u8]) -> Result<T, ServiceError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let decompressed = if self.config.enable_compression && self.is_compressed(data) {
            self.decompress_data(data)?
        } else {
            data.to_vec()
        };
        
        serde_json::from_slice(&decompressed)
            .map_err(|e| ServiceError::CacheError(format!("Deserialization failed: {}", e)))
    }
    
    /// Simple compression implementation
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, ServiceError> {
        // For now, just return the data as-is
        // In production, you might want to use a proper compression library
        Ok(data.to_vec())
    }
    
    /// Simple decompression implementation
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, ServiceError> {
        // For now, just return the data as-is
        Ok(data.to_vec())
    }
    
    /// Check if data is compressed
    fn is_compressed(&self, _data: &[u8]) -> bool {
        // Simple heuristic - in production, use proper compression headers
        false
    }
    
    /// Update cache statistics
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut stats);
        stats.calculate_hit_rate();
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
impl Cache for RedisCache {
    async fn get<T>(&self, key: &CacheKey) -> Result<Option<T>, ServiceError>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        debug!("Getting cache key: {}", key_str);
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let result: RedisResult<Vec<u8>> = conn.get(&key_str).await;
        
        match result {
            Ok(data) => {
                debug!("Cache hit for key: {}", key_str);
                self.update_stats(|stats| stats.hits += 1).await;
                
                let value = self.deserialize_value(&data)?;
                Ok(Some(value))
            }
            Err(redis::RedisError { kind: redis::ErrorKind::TypeError, .. }) => {
                debug!("Cache miss for key: {}", key_str);
                self.update_stats(|stats| stats.misses += 1).await;
                Ok(None)
            }
            Err(e) => {
                error!("Redis get error for key {}: {}", key_str, e);
                self.update_stats(|stats| stats.misses += 1).await;
                Err(ServiceError::CacheError(format!("Redis get failed: {}", e)))
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
        
        let ttl = ttl.unwrap_or(key.default_ttl());
        debug!("Setting cache key: {} with TTL: {:?}", key_str, ttl);
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let _: RedisResult<()> = conn.set_ex(&key_str, data, ttl.as_secs()).await;
        
        self.update_stats(|stats| stats.sets += 1).await;
        Ok(())
    }
    
    async fn delete(&self, key: &CacheKey) -> Result<(), ServiceError> {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        debug!("Deleting cache key: {}", key_str);
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let _: RedisResult<i32> = conn.del(&key_str).await;
        
        self.update_stats(|stats| stats.deletes += 1).await;
        Ok(())
    }
    
    async fn exists(&self, key: &CacheKey) -> Result<bool, ServiceError> {
        self.validate_key(key)?;
        
        let key_str = key.to_redis_key();
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let exists: RedisResult<bool> = conn.exists(&key_str).await;
        
        exists.map_err(|e| ServiceError::CacheError(format!("Redis exists failed: {}", e)))
    }
    
    async fn clear(&self) -> Result<(), ServiceError> {
        warn!("Clearing all cache entries");
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        let _: RedisResult<()> = conn.flushdb().await;
        
        // Reset stats
        let mut stats = self.stats.write().await;
        *stats = CacheStats::new();
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, ServiceError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, ServiceError> {
        debug!("Invalidating cache pattern: {}", pattern);
        
        let mut conn = self.pool.get().await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
        
        // Get all keys matching pattern
        let keys: RedisResult<Vec<String>> = conn.keys(pattern).await;
        
        match keys {
            Ok(key_list) => {
                if !key_list.is_empty() {
                    let count = key_list.len() as u64;
                    let _: RedisResult<i32> = conn.del(&key_list).await;
                    
                    self.update_stats(|stats| {
                        stats.deletes += count;
                        stats.evictions += count;
                    }).await;
                    
                    info!("Invalidated {} cache entries matching pattern: {}", count, pattern);
                    Ok(count)
                } else {
                    Ok(0)
                }
            }
            Err(e) => {
                error!("Failed to get keys for pattern {}: {}", pattern, e);
                Err(ServiceError::CacheError(format!("Pattern invalidation failed: {}", e)))
            }
        }
    }
}

/// Redis cache builder for easier configuration
pub struct RedisCacheBuilder {
    config: CacheConfig,
}

impl RedisCacheBuilder {
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }
    
    pub fn redis_url(mut self, url: &str) -> Self {
        self.config.redis_url = url.to_string();
        self
    }
    
    pub fn pool_size(mut self, size: u32) -> Self {
        self.config.redis_pool_size = size;
        self
    }
    
    pub fn timeout(mut self, timeout_ms: u64) -> Self {
        self.config.redis_timeout_ms = timeout_ms;
        self
    }
    
    pub fn enable_compression(mut self, enable: bool) -> Self {
        self.config.enable_compression = enable;
        self
    }
    
    pub fn max_value_size(mut self, size_bytes: usize) -> Self {
        self.config.max_value_size_bytes = size_bytes;
        self
    }
    
    pub async fn build(self) -> Result<RedisCache, ServiceError> {
        RedisCache::new(self.config).await
    }
}

impl Default for RedisCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}
