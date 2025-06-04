//! Caching infrastructure for FO3 Wallet Core
//!
//! Provides Redis-based caching with fallback to in-memory caching for improved performance.
//! Implements cache invalidation strategies and performance monitoring.

pub mod redis_cache;
pub mod memory_cache;
pub mod cache_manager;
pub mod invalidation;
pub mod metrics;
pub mod load_testing;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::ServiceError;

/// Cache key types for different data categories
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    // User session and authentication
    UserSession(Uuid),
    UserPermissions(Uuid),
    JwtBlacklist(String),
    
    // KYC and compliance
    KycStatus(Uuid),
    KycDocuments(Uuid),
    ComplianceCheck(Uuid),
    
    // Wallet and transaction data
    WalletBalance(Uuid),
    TransactionHistory(Uuid),
    PendingTransactions(Uuid),
    
    // Card and payment data
    CardLimits(Uuid),
    CardTransactions(Uuid),
    PaymentMethods(Uuid),
    
    // Pricing and market data
    AssetPrice(String),
    MarketData(String),
    ExchangeRates(String),
    
    // DeFi and yield data
    YieldRates(String),
    PoolData(String),
    StakingRewards(Uuid),
    
    // Analytics and insights
    SpendingInsights(Uuid),
    UserAnalytics(Uuid),
    ReferralData(Uuid),
    
    // ML model results
    RiskAssessment(Uuid),
    SentimentAnalysis(String),
    MarketPrediction(String),
    
    // System configuration
    FeatureFlags(String),
    SystemConfig(String),
    ServiceHealth(String),
}

impl CacheKey {
    /// Generate Redis key string
    pub fn to_redis_key(&self) -> String {
        match self {
            CacheKey::UserSession(id) => format!("session:{}", id),
            CacheKey::UserPermissions(id) => format!("permissions:{}", id),
            CacheKey::JwtBlacklist(token) => format!("jwt_blacklist:{}", token),
            CacheKey::KycStatus(id) => format!("kyc:status:{}", id),
            CacheKey::KycDocuments(id) => format!("kyc:docs:{}", id),
            CacheKey::ComplianceCheck(id) => format!("compliance:{}", id),
            CacheKey::WalletBalance(id) => format!("wallet:balance:{}", id),
            CacheKey::TransactionHistory(id) => format!("wallet:tx_history:{}", id),
            CacheKey::PendingTransactions(id) => format!("wallet:pending:{}", id),
            CacheKey::CardLimits(id) => format!("card:limits:{}", id),
            CacheKey::CardTransactions(id) => format!("card:transactions:{}", id),
            CacheKey::PaymentMethods(id) => format!("payment:methods:{}", id),
            CacheKey::AssetPrice(symbol) => format!("price:{}", symbol),
            CacheKey::MarketData(symbol) => format!("market:{}", symbol),
            CacheKey::ExchangeRates(pair) => format!("exchange:{}", pair),
            CacheKey::YieldRates(protocol) => format!("yield:{}", protocol),
            CacheKey::PoolData(pool_id) => format!("pool:{}", pool_id),
            CacheKey::StakingRewards(id) => format!("staking:{}", id),
            CacheKey::SpendingInsights(id) => format!("insights:{}", id),
            CacheKey::UserAnalytics(id) => format!("analytics:{}", id),
            CacheKey::ReferralData(id) => format!("referral:{}", id),
            CacheKey::RiskAssessment(id) => format!("risk:{}", id),
            CacheKey::SentimentAnalysis(topic) => format!("sentiment:{}", topic),
            CacheKey::MarketPrediction(symbol) => format!("prediction:{}", symbol),
            CacheKey::FeatureFlags(flag) => format!("feature:{}", flag),
            CacheKey::SystemConfig(key) => format!("config:{}", key),
            CacheKey::ServiceHealth(service) => format!("health:{}", service),
        }
    }
    
    /// Get default TTL for cache key type
    pub fn default_ttl(&self) -> Duration {
        match self {
            // Short-lived session data
            CacheKey::UserSession(_) => Duration::from_secs(1800), // 30 minutes
            CacheKey::JwtBlacklist(_) => Duration::from_secs(3600), // 1 hour
            
            // Medium-lived user data
            CacheKey::UserPermissions(_) => Duration::from_secs(3600), // 1 hour
            CacheKey::KycStatus(_) => Duration::from_secs(7200), // 2 hours
            CacheKey::ComplianceCheck(_) => Duration::from_secs(3600), // 1 hour
            
            // Wallet data with moderate volatility
            CacheKey::WalletBalance(_) => Duration::from_secs(300), // 5 minutes
            CacheKey::TransactionHistory(_) => Duration::from_secs(1800), // 30 minutes
            CacheKey::PendingTransactions(_) => Duration::from_secs(60), // 1 minute
            
            // Card and payment data
            CacheKey::CardLimits(_) => Duration::from_secs(3600), // 1 hour
            CacheKey::CardTransactions(_) => Duration::from_secs(900), // 15 minutes
            CacheKey::PaymentMethods(_) => Duration::from_secs(3600), // 1 hour
            
            // High-frequency market data
            CacheKey::AssetPrice(_) => Duration::from_secs(60), // 1 minute
            CacheKey::MarketData(_) => Duration::from_secs(300), // 5 minutes
            CacheKey::ExchangeRates(_) => Duration::from_secs(300), // 5 minutes
            
            // DeFi data
            CacheKey::YieldRates(_) => Duration::from_secs(600), // 10 minutes
            CacheKey::PoolData(_) => Duration::from_secs(300), // 5 minutes
            CacheKey::StakingRewards(_) => Duration::from_secs(1800), // 30 minutes
            
            // Analytics data
            CacheKey::SpendingInsights(_) => Duration::from_secs(3600), // 1 hour
            CacheKey::UserAnalytics(_) => Duration::from_secs(7200), // 2 hours
            CacheKey::ReferralData(_) => Duration::from_secs(1800), // 30 minutes
            
            // ML results
            CacheKey::RiskAssessment(_) => Duration::from_secs(1800), // 30 minutes
            CacheKey::SentimentAnalysis(_) => Duration::from_secs(900), // 15 minutes
            CacheKey::MarketPrediction(_) => Duration::from_secs(3600), // 1 hour
            
            // Long-lived system data
            CacheKey::FeatureFlags(_) => Duration::from_secs(7200), // 2 hours
            CacheKey::SystemConfig(_) => Duration::from_secs(14400), // 4 hours
            CacheKey::ServiceHealth(_) => Duration::from_secs(60), // 1 minute
            
            // Documents are cached longer
            CacheKey::KycDocuments(_) => Duration::from_secs(14400), // 4 hours
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            data,
            created_at: now,
            expires_at: now + chrono::Duration::from_std(ttl).unwrap(),
            access_count: 0,
            last_accessed: now,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
    pub evictions: u64,
    pub memory_usage_bytes: u64,
    pub entry_count: u64,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            sets: 0,
            deletes: 0,
            evictions: 0,
            memory_usage_bytes: 0,
            entry_count: 0,
            hit_rate: 0.0,
        }
    }
    
    pub fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// Cache trait for different implementations
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get value from cache
    async fn get<T>(&self, key: &CacheKey) -> Result<Option<T>, ServiceError>
    where
        T: for<'de> Deserialize<'de> + Send;
    
    /// Set value in cache with TTL
    async fn set<T>(&self, key: &CacheKey, value: &T, ttl: Option<Duration>) -> Result<(), ServiceError>
    where
        T: Serialize + Send + Sync;
    
    /// Delete value from cache
    async fn delete(&self, key: &CacheKey) -> Result<(), ServiceError>;
    
    /// Check if key exists in cache
    async fn exists(&self, key: &CacheKey) -> Result<bool, ServiceError>;
    
    /// Clear all cache entries
    async fn clear(&self) -> Result<(), ServiceError>;
    
    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats, ServiceError>;
    
    /// Invalidate cache entries by pattern
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, ServiceError>;
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub redis_url: String,
    pub redis_pool_size: u32,
    pub redis_timeout_ms: u64,
    pub memory_cache_size: usize,
    pub memory_cache_ttl_seconds: u64,
    pub enable_compression: bool,
    pub enable_encryption: bool,
    pub default_ttl_seconds: u64,
    pub max_key_length: usize,
    pub max_value_size_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            redis_pool_size: 10,
            redis_timeout_ms: 5000,
            memory_cache_size: 10000,
            memory_cache_ttl_seconds: 300,
            enable_compression: true,
            enable_encryption: false,
            default_ttl_seconds: 300,
            max_key_length: 250,
            max_value_size_bytes: 1024 * 1024, // 1MB
        }
    }
}
