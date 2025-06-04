//! Cache manager with multi-tier caching strategy
//!
//! Provides intelligent caching with Redis as primary cache and memory cache as fallback.
//! Implements cache warming, invalidation strategies, and performance monitoring.

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::error::ServiceError;
use super::{
    Cache, CacheKey, CacheStats, CacheConfig,
    redis_cache::RedisCache,
    memory_cache::MemoryCache,
};

/// Multi-tier cache manager
pub struct CacheManager {
    redis_cache: Option<Arc<RedisCache>>,
    memory_cache: Arc<MemoryCache>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheManagerStats>>,
}

/// Cache manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheManagerStats {
    pub redis_stats: Option<CacheStats>,
    pub memory_stats: CacheStats,
    pub redis_available: bool,
    pub fallback_count: u64,
    pub cache_warming_count: u64,
    pub total_operations: u64,
}

impl CacheManagerStats {
    pub fn new() -> Self {
        Self {
            redis_stats: None,
            memory_stats: CacheStats::new(),
            redis_available: false,
            fallback_count: 0,
            cache_warming_count: 0,
            total_operations: 0,
        }
    }
}

impl CacheManager {
    /// Create new cache manager with Redis and memory cache
    pub async fn new(config: CacheConfig) -> Result<Self, ServiceError> {
        info!("Initializing cache manager");
        
        // Initialize memory cache (always available)
        let memory_cache = Arc::new(MemoryCache::new(config.clone()));
        
        // Try to initialize Redis cache
        let redis_cache = match RedisCache::new(config.clone()).await {
            Ok(cache) => {
                info!("Redis cache initialized successfully");
                Some(Arc::new(cache))
            }
            Err(e) => {
                warn!("Failed to initialize Redis cache, using memory cache only: {}", e);
                None
            }
        };
        
        let stats = Arc::new(RwLock::new(CacheManagerStats::new()));
        
        let manager = Self {
            redis_cache,
            memory_cache,
            config,
            stats,
        };
        
        // Update initial stats
        manager.update_stats().await;
        
        info!("Cache manager initialized with Redis: {}", manager.redis_cache.is_some());
        Ok(manager)
    }
    
    /// Create cache manager with memory cache only
    pub fn memory_only(config: CacheConfig) -> Self {
        info!("Initializing memory-only cache manager");
        
        let memory_cache = Arc::new(MemoryCache::new(config.clone()));
        let stats = Arc::new(RwLock::new(CacheManagerStats::new()));
        
        Self {
            redis_cache: None,
            memory_cache,
            config,
            stats,
        }
    }
    
    /// Check if Redis is available
    pub fn is_redis_available(&self) -> bool {
        self.redis_cache.is_some()
    }
    
    /// Get cache tier preference for key type
    fn get_cache_tier_preference(&self, key: &CacheKey) -> CacheTier {
        match key {
            // High-frequency, small data - prefer memory cache
            CacheKey::UserSession(_) |
            CacheKey::UserPermissions(_) |
            CacheKey::JwtBlacklist(_) |
            CacheKey::ServiceHealth(_) => CacheTier::Memory,
            
            // Large data or less frequent access - prefer Redis
            CacheKey::KycDocuments(_) |
            CacheKey::TransactionHistory(_) |
            CacheKey::SpendingInsights(_) |
            CacheKey::UserAnalytics(_) => CacheTier::Redis,
            
            // Medium data - use both tiers
            _ => CacheTier::Both,
        }
    }
    
    /// Update cache manager statistics
    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;
        
        // Update Redis stats if available
        if let Some(redis_cache) = &self.redis_cache {
            match redis_cache.stats().await {
                Ok(redis_stats) => {
                    stats.redis_stats = Some(redis_stats);
                    stats.redis_available = true;
                }
                Err(_) => {
                    stats.redis_available = false;
                }
            }
        } else {
            stats.redis_available = false;
        }
        
        // Update memory stats
        if let Ok(memory_stats) = self.memory_cache.stats().await {
            stats.memory_stats = memory_stats;
        }
        
        stats.total_operations += 1;
    }
    
    /// Warm cache with frequently accessed data
    pub async fn warm_cache(&self, keys_and_values: Vec<(CacheKey, serde_json::Value)>) -> Result<(), ServiceError> {
        info!("Warming cache with {} entries", keys_and_values.len());
        
        for (key, value) in keys_and_values {
            if let Err(e) = self.set(&key, &value, None).await {
                warn!("Failed to warm cache for key {:?}: {}", key, e);
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.cache_warming_count += 1;
        
        Ok(())
    }
    
    /// Invalidate cache entries by user ID
    pub async fn invalidate_user_cache(&self, user_id: uuid::Uuid) -> Result<u64, ServiceError> {
        let pattern = format!("*:{}*", user_id);
        self.invalidate_pattern(&pattern).await
    }
    
    /// Invalidate cache entries by service
    pub async fn invalidate_service_cache(&self, service: &str) -> Result<u64, ServiceError> {
        let pattern = format!("{}:*", service);
        self.invalidate_pattern(&pattern).await
    }
    
    /// Get comprehensive cache statistics
    pub async fn get_comprehensive_stats(&self) -> Result<CacheManagerStats, ServiceError> {
        self.update_stats().await;
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Health check for cache manager
    pub async fn health_check(&self) -> Result<CacheHealthStatus, ServiceError> {
        let mut status = CacheHealthStatus {
            redis_healthy: false,
            memory_healthy: false,
            overall_healthy: false,
            error_message: None,
        };
        
        // Check memory cache
        match self.memory_cache.stats().await {
            Ok(_) => status.memory_healthy = true,
            Err(e) => status.error_message = Some(format!("Memory cache error: {}", e)),
        }
        
        // Check Redis cache if available
        if let Some(redis_cache) = &self.redis_cache {
            match redis_cache.stats().await {
                Ok(_) => status.redis_healthy = true,
                Err(e) => {
                    if status.error_message.is_none() {
                        status.error_message = Some(format!("Redis cache error: {}", e));
                    }
                }
            }
        } else {
            status.redis_healthy = false;
        }
        
        status.overall_healthy = status.memory_healthy && (status.redis_healthy || self.redis_cache.is_none());
        
        Ok(status)
    }
}

#[async_trait]
impl Cache for CacheManager {
    async fn get<T>(&self, key: &CacheKey) -> Result<Option<T>, ServiceError>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let tier_preference = self.get_cache_tier_preference(key);
        
        // Try Redis first if preferred and available
        if matches!(tier_preference, CacheTier::Redis | CacheTier::Both) {
            if let Some(redis_cache) = &self.redis_cache {
                match redis_cache.get(key).await {
                    Ok(Some(value)) => {
                        debug!("Cache hit from Redis for key: {:?}", key);
                        self.update_stats().await;
                        return Ok(Some(value));
                    }
                    Ok(None) => {
                        debug!("Cache miss from Redis for key: {:?}", key);
                    }
                    Err(e) => {
                        warn!("Redis cache error for key {:?}: {}", key, e);
                        let mut stats = self.stats.write().await;
                        stats.fallback_count += 1;
                    }
                }
            }
        }
        
        // Try memory cache
        match self.memory_cache.get(key).await {
            Ok(Some(value)) => {
                debug!("Cache hit from memory for key: {:?}", key);
                
                // If we got a hit from memory but Redis is available, warm Redis cache
                if let Some(redis_cache) = &self.redis_cache {
                    if matches!(tier_preference, CacheTier::Redis | CacheTier::Both) {
                        let _ = redis_cache.set(key, &value, None).await;
                    }
                }
                
                self.update_stats().await;
                Ok(Some(value))
            }
            Ok(None) => {
                debug!("Cache miss from memory for key: {:?}", key);
                self.update_stats().await;
                Ok(None)
            }
            Err(e) => {
                error!("Memory cache error for key {:?}: {}", key, e);
                self.update_stats().await;
                Err(e)
            }
        }
    }
    
    async fn set<T>(&self, key: &CacheKey, value: &T, ttl: Option<Duration>) -> Result<(), ServiceError>
    where
        T: Serialize + Send + Sync,
    {
        let tier_preference = self.get_cache_tier_preference(key);
        let mut redis_success = false;
        let mut memory_success = false;
        
        // Set in Redis if preferred and available
        if matches!(tier_preference, CacheTier::Redis | CacheTier::Both) {
            if let Some(redis_cache) = &self.redis_cache {
                match redis_cache.set(key, value, ttl).await {
                    Ok(()) => {
                        debug!("Set cache in Redis for key: {:?}", key);
                        redis_success = true;
                    }
                    Err(e) => {
                        warn!("Failed to set Redis cache for key {:?}: {}", key, e);
                        let mut stats = self.stats.write().await;
                        stats.fallback_count += 1;
                    }
                }
            }
        }
        
        // Set in memory cache
        if matches!(tier_preference, CacheTier::Memory | CacheTier::Both) || !redis_success {
            match self.memory_cache.set(key, value, ttl).await {
                Ok(()) => {
                    debug!("Set cache in memory for key: {:?}", key);
                    memory_success = true;
                }
                Err(e) => {
                    error!("Failed to set memory cache for key {:?}: {}", key, e);
                }
            }
        }
        
        self.update_stats().await;
        
        if redis_success || memory_success {
            Ok(())
        } else {
            Err(ServiceError::CacheError("Failed to set cache in any tier".to_string()))
        }
    }
    
    async fn delete(&self, key: &CacheKey) -> Result<(), ServiceError> {
        // Delete from both caches
        if let Some(redis_cache) = &self.redis_cache {
            let _ = redis_cache.delete(key).await;
        }
        
        let _ = self.memory_cache.delete(key).await;
        
        self.update_stats().await;
        Ok(())
    }
    
    async fn exists(&self, key: &CacheKey) -> Result<bool, ServiceError> {
        // Check Redis first if available
        if let Some(redis_cache) = &self.redis_cache {
            if let Ok(exists) = redis_cache.exists(key).await {
                if exists {
                    return Ok(true);
                }
            }
        }
        
        // Check memory cache
        self.memory_cache.exists(key).await
    }
    
    async fn clear(&self) -> Result<(), ServiceError> {
        // Clear both caches
        if let Some(redis_cache) = &self.redis_cache {
            let _ = redis_cache.clear().await;
        }
        
        self.memory_cache.clear().await?;
        
        // Reset stats
        let mut stats = self.stats.write().await;
        *stats = CacheManagerStats::new();
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, ServiceError> {
        // Return combined stats from memory cache (primary)
        self.memory_cache.stats().await
    }
    
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, ServiceError> {
        let mut total_invalidated = 0u64;
        
        // Invalidate from Redis if available
        if let Some(redis_cache) = &self.redis_cache {
            match redis_cache.invalidate_pattern(pattern).await {
                Ok(count) => total_invalidated += count,
                Err(e) => warn!("Failed to invalidate Redis pattern {}: {}", pattern, e),
            }
        }
        
        // Invalidate from memory cache
        match self.memory_cache.invalidate_pattern(pattern).await {
            Ok(count) => total_invalidated += count,
            Err(e) => warn!("Failed to invalidate memory pattern {}: {}", pattern, e),
        }
        
        self.update_stats().await;
        Ok(total_invalidated)
    }
}

/// Cache tier preference
#[derive(Debug, Clone, PartialEq)]
enum CacheTier {
    Redis,
    Memory,
    Both,
}

/// Cache health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealthStatus {
    pub redis_healthy: bool,
    pub memory_healthy: bool,
    pub overall_healthy: bool,
    pub error_message: Option<String>,
}
