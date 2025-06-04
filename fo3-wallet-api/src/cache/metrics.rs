//! Cache performance metrics and monitoring
//!
//! Provides comprehensive metrics collection for cache performance analysis,
//! monitoring, and optimization.

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, 
    Opts, HistogramOpts, Registry, Result as PrometheusResult
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use chrono::{DateTime, Utc};

use crate::error::ServiceError;
use super::{CacheKey, CacheStats};

/// Cache metrics collector
pub struct CacheMetrics {
    // Basic cache operations
    pub cache_hits: CounterVec,
    pub cache_misses: CounterVec,
    pub cache_sets: CounterVec,
    pub cache_deletes: CounterVec,
    pub cache_evictions: CounterVec,
    
    // Performance metrics
    pub cache_operation_duration: HistogramVec,
    pub cache_size_bytes: GaugeVec,
    pub cache_entry_count: GaugeVec,
    pub cache_hit_rate: GaugeVec,
    
    // Error metrics
    pub cache_errors: CounterVec,
    pub cache_timeouts: CounterVec,
    pub cache_connection_errors: CounterVec,
    
    // Invalidation metrics
    pub cache_invalidations: CounterVec,
    pub cache_invalidation_duration: HistogramVec,
    
    // Memory usage metrics
    pub memory_cache_usage: Gauge,
    pub redis_memory_usage: Gauge,
    
    // Connection pool metrics
    pub redis_pool_active_connections: Gauge,
    pub redis_pool_idle_connections: Gauge,
    pub redis_pool_wait_time: Histogram,
    
    // Business metrics
    pub cache_warming_operations: Counter,
    pub cache_fallback_operations: Counter,
    
    registry: Registry,
}

impl CacheMetrics {
    /// Create new cache metrics collector
    pub fn new() -> PrometheusResult<Self> {
        let registry = Registry::new();
        
        // Basic cache operations
        let cache_hits = CounterVec::new(
            Opts::new("cache_hits_total", "Total cache hits"),
            &["cache_type", "key_type"]
        )?;
        
        let cache_misses = CounterVec::new(
            Opts::new("cache_misses_total", "Total cache misses"),
            &["cache_type", "key_type"]
        )?;
        
        let cache_sets = CounterVec::new(
            Opts::new("cache_sets_total", "Total cache sets"),
            &["cache_type", "key_type"]
        )?;
        
        let cache_deletes = CounterVec::new(
            Opts::new("cache_deletes_total", "Total cache deletes"),
            &["cache_type", "key_type"]
        )?;
        
        let cache_evictions = CounterVec::new(
            Opts::new("cache_evictions_total", "Total cache evictions"),
            &["cache_type", "reason"]
        )?;
        
        // Performance metrics
        let cache_operation_duration = HistogramVec::new(
            HistogramOpts::new("cache_operation_duration_seconds", "Cache operation duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["cache_type", "operation"]
        )?;
        
        let cache_size_bytes = GaugeVec::new(
            Opts::new("cache_size_bytes", "Cache size in bytes"),
            &["cache_type"]
        )?;
        
        let cache_entry_count = GaugeVec::new(
            Opts::new("cache_entry_count", "Number of cache entries"),
            &["cache_type"]
        )?;
        
        let cache_hit_rate = GaugeVec::new(
            Opts::new("cache_hit_rate", "Cache hit rate (0-1)"),
            &["cache_type"]
        )?;
        
        // Error metrics
        let cache_errors = CounterVec::new(
            Opts::new("cache_errors_total", "Total cache errors"),
            &["cache_type", "error_type"]
        )?;
        
        let cache_timeouts = CounterVec::new(
            Opts::new("cache_timeouts_total", "Total cache timeouts"),
            &["cache_type", "operation"]
        )?;
        
        let cache_connection_errors = CounterVec::new(
            Opts::new("cache_connection_errors_total", "Total cache connection errors"),
            &["cache_type"]
        )?;
        
        // Invalidation metrics
        let cache_invalidations = CounterVec::new(
            Opts::new("cache_invalidations_total", "Total cache invalidations"),
            &["cache_type", "invalidation_type"]
        )?;
        
        let cache_invalidation_duration = HistogramVec::new(
            HistogramOpts::new("cache_invalidation_duration_seconds", "Cache invalidation duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["cache_type", "invalidation_type"]
        )?;
        
        // Memory usage metrics
        let memory_cache_usage = Gauge::with_opts(
            Opts::new("memory_cache_usage_bytes", "Memory cache usage in bytes")
        )?;
        
        let redis_memory_usage = Gauge::with_opts(
            Opts::new("redis_memory_usage_bytes", "Redis memory usage in bytes")
        )?;
        
        // Connection pool metrics
        let redis_pool_active_connections = Gauge::with_opts(
            Opts::new("redis_pool_active_connections", "Active Redis connections")
        )?;
        
        let redis_pool_idle_connections = Gauge::with_opts(
            Opts::new("redis_pool_idle_connections", "Idle Redis connections")
        )?;
        
        let redis_pool_wait_time = Histogram::with_opts(
            HistogramOpts::new("redis_pool_wait_time_seconds", "Redis connection wait time")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
        )?;
        
        // Business metrics
        let cache_warming_operations = Counter::with_opts(
            Opts::new("cache_warming_operations_total", "Total cache warming operations")
        )?;
        
        let cache_fallback_operations = Counter::with_opts(
            Opts::new("cache_fallback_operations_total", "Total cache fallback operations")
        )?;
        
        // Register all metrics
        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(cache_sets.clone()))?;
        registry.register(Box::new(cache_deletes.clone()))?;
        registry.register(Box::new(cache_evictions.clone()))?;
        registry.register(Box::new(cache_operation_duration.clone()))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;
        registry.register(Box::new(cache_entry_count.clone()))?;
        registry.register(Box::new(cache_hit_rate.clone()))?;
        registry.register(Box::new(cache_errors.clone()))?;
        registry.register(Box::new(cache_timeouts.clone()))?;
        registry.register(Box::new(cache_connection_errors.clone()))?;
        registry.register(Box::new(cache_invalidations.clone()))?;
        registry.register(Box::new(cache_invalidation_duration.clone()))?;
        registry.register(Box::new(memory_cache_usage.clone()))?;
        registry.register(Box::new(redis_memory_usage.clone()))?;
        registry.register(Box::new(redis_pool_active_connections.clone()))?;
        registry.register(Box::new(redis_pool_idle_connections.clone()))?;
        registry.register(Box::new(redis_pool_wait_time.clone()))?;
        registry.register(Box::new(cache_warming_operations.clone()))?;
        registry.register(Box::new(cache_fallback_operations.clone()))?;
        
        info!("Cache metrics initialized successfully");
        
        Ok(Self {
            cache_hits,
            cache_misses,
            cache_sets,
            cache_deletes,
            cache_evictions,
            cache_operation_duration,
            cache_size_bytes,
            cache_entry_count,
            cache_hit_rate,
            cache_errors,
            cache_timeouts,
            cache_connection_errors,
            cache_invalidations,
            cache_invalidation_duration,
            memory_cache_usage,
            redis_memory_usage,
            redis_pool_active_connections,
            redis_pool_idle_connections,
            redis_pool_wait_time,
            cache_warming_operations,
            cache_fallback_operations,
            registry,
        })
    }
    
    /// Record cache hit
    pub fn record_hit(&self, cache_type: &str, key_type: &str) {
        self.cache_hits
            .with_label_values(&[cache_type, key_type])
            .inc();
    }
    
    /// Record cache miss
    pub fn record_miss(&self, cache_type: &str, key_type: &str) {
        self.cache_misses
            .with_label_values(&[cache_type, key_type])
            .inc();
    }
    
    /// Record cache set operation
    pub fn record_set(&self, cache_type: &str, key_type: &str) {
        self.cache_sets
            .with_label_values(&[cache_type, key_type])
            .inc();
    }
    
    /// Record cache delete operation
    pub fn record_delete(&self, cache_type: &str, key_type: &str) {
        self.cache_deletes
            .with_label_values(&[cache_type, key_type])
            .inc();
    }
    
    /// Record cache eviction
    pub fn record_eviction(&self, cache_type: &str, reason: &str) {
        self.cache_evictions
            .with_label_values(&[cache_type, reason])
            .inc();
    }
    
    /// Record cache operation duration
    pub fn record_operation_duration(&self, cache_type: &str, operation: &str, duration: f64) {
        self.cache_operation_duration
            .with_label_values(&[cache_type, operation])
            .observe(duration);
    }
    
    /// Update cache size metrics
    pub fn update_cache_size(&self, cache_type: &str, size_bytes: u64, entry_count: u64) {
        self.cache_size_bytes
            .with_label_values(&[cache_type])
            .set(size_bytes as f64);
        
        self.cache_entry_count
            .with_label_values(&[cache_type])
            .set(entry_count as f64);
    }
    
    /// Update cache hit rate
    pub fn update_hit_rate(&self, cache_type: &str, hit_rate: f64) {
        self.cache_hit_rate
            .with_label_values(&[cache_type])
            .set(hit_rate);
    }
    
    /// Record cache error
    pub fn record_error(&self, cache_type: &str, error_type: &str) {
        self.cache_errors
            .with_label_values(&[cache_type, error_type])
            .inc();
    }
    
    /// Record cache timeout
    pub fn record_timeout(&self, cache_type: &str, operation: &str) {
        self.cache_timeouts
            .with_label_values(&[cache_type, operation])
            .inc();
    }
    
    /// Record connection error
    pub fn record_connection_error(&self, cache_type: &str) {
        self.cache_connection_errors
            .with_label_values(&[cache_type])
            .inc();
    }
    
    /// Record cache invalidation
    pub fn record_invalidation(&self, cache_type: &str, invalidation_type: &str, duration: f64) {
        self.cache_invalidations
            .with_label_values(&[cache_type, invalidation_type])
            .inc();
        
        self.cache_invalidation_duration
            .with_label_values(&[cache_type, invalidation_type])
            .observe(duration);
    }
    
    /// Update memory usage
    pub fn update_memory_usage(&self, memory_bytes: u64, redis_bytes: u64) {
        self.memory_cache_usage.set(memory_bytes as f64);
        self.redis_memory_usage.set(redis_bytes as f64);
    }
    
    /// Update Redis pool metrics
    pub fn update_redis_pool_metrics(&self, active: u32, idle: u32, wait_time: f64) {
        self.redis_pool_active_connections.set(active as f64);
        self.redis_pool_idle_connections.set(idle as f64);
        self.redis_pool_wait_time.observe(wait_time);
    }
    
    /// Record cache warming operation
    pub fn record_cache_warming(&self) {
        self.cache_warming_operations.inc();
    }
    
    /// Record cache fallback operation
    pub fn record_cache_fallback(&self) {
        self.cache_fallback_operations.inc();
    }
    
    /// Get cache key type for metrics
    pub fn get_key_type(&self, key: &CacheKey) -> &'static str {
        match key {
            CacheKey::UserSession(_) => "user_session",
            CacheKey::UserPermissions(_) => "user_permissions",
            CacheKey::JwtBlacklist(_) => "jwt_blacklist",
            CacheKey::KycStatus(_) => "kyc_status",
            CacheKey::KycDocuments(_) => "kyc_documents",
            CacheKey::ComplianceCheck(_) => "compliance_check",
            CacheKey::WalletBalance(_) => "wallet_balance",
            CacheKey::TransactionHistory(_) => "transaction_history",
            CacheKey::PendingTransactions(_) => "pending_transactions",
            CacheKey::CardLimits(_) => "card_limits",
            CacheKey::CardTransactions(_) => "card_transactions",
            CacheKey::PaymentMethods(_) => "payment_methods",
            CacheKey::AssetPrice(_) => "asset_price",
            CacheKey::MarketData(_) => "market_data",
            CacheKey::ExchangeRates(_) => "exchange_rates",
            CacheKey::YieldRates(_) => "yield_rates",
            CacheKey::PoolData(_) => "pool_data",
            CacheKey::StakingRewards(_) => "staking_rewards",
            CacheKey::SpendingInsights(_) => "spending_insights",
            CacheKey::UserAnalytics(_) => "user_analytics",
            CacheKey::ReferralData(_) => "referral_data",
            CacheKey::RiskAssessment(_) => "risk_assessment",
            CacheKey::SentimentAnalysis(_) => "sentiment_analysis",
            CacheKey::MarketPrediction(_) => "market_prediction",
            CacheKey::FeatureFlags(_) => "feature_flags",
            CacheKey::SystemConfig(_) => "system_config",
            CacheKey::ServiceHealth(_) => "service_health",
        }
    }
    
    /// Update metrics from cache stats
    pub fn update_from_stats(&self, cache_type: &str, stats: &CacheStats) {
        self.update_cache_size(cache_type, stats.memory_usage_bytes, stats.entry_count);
        self.update_hit_rate(cache_type, stats.hit_rate);
    }
    
    /// Get Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

/// Cache performance analyzer
pub struct CachePerformanceAnalyzer {
    metrics: Arc<CacheMetrics>,
    analysis_history: Arc<RwLock<Vec<PerformanceSnapshot>>>,
}

/// Performance snapshot for analysis
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub redis_hit_rate: f64,
    pub memory_hit_rate: f64,
    pub average_response_time: f64,
    pub error_rate: f64,
    pub memory_usage_mb: f64,
    pub redis_memory_usage_mb: f64,
    pub total_operations: u64,
}

impl CachePerformanceAnalyzer {
    /// Create new performance analyzer
    pub fn new(metrics: Arc<CacheMetrics>) -> Self {
        Self {
            metrics,
            analysis_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Take performance snapshot
    pub async fn take_snapshot(&self) -> Result<PerformanceSnapshot, ServiceError> {
        // In a real implementation, you would gather metrics from Prometheus
        // For now, we'll create a mock snapshot
        let snapshot = PerformanceSnapshot {
            timestamp: Utc::now(),
            redis_hit_rate: 0.85,
            memory_hit_rate: 0.92,
            average_response_time: 0.015,
            error_rate: 0.001,
            memory_usage_mb: 256.0,
            redis_memory_usage_mb: 512.0,
            total_operations: 10000,
        };
        
        let mut history = self.analysis_history.write().await;
        history.push(snapshot.clone());
        
        // Keep only last 100 snapshots
        if history.len() > 100 {
            history.remove(0);
        }
        
        debug!("Performance snapshot taken: hit_rate={:.2}%, response_time={:.3}ms", 
               snapshot.redis_hit_rate * 100.0, 
               snapshot.average_response_time * 1000.0);
        
        Ok(snapshot)
    }
    
    /// Analyze performance trends
    pub async fn analyze_trends(&self) -> Result<PerformanceTrends, ServiceError> {
        let history = self.analysis_history.read().await;
        
        if history.len() < 2 {
            return Ok(PerformanceTrends::default());
        }
        
        let recent = &history[history.len() - 1];
        let previous = &history[history.len() - 2];
        
        let trends = PerformanceTrends {
            hit_rate_trend: recent.redis_hit_rate - previous.redis_hit_rate,
            response_time_trend: recent.average_response_time - previous.average_response_time,
            error_rate_trend: recent.error_rate - previous.error_rate,
            memory_usage_trend: recent.memory_usage_mb - previous.memory_usage_mb,
            recommendations: self.generate_recommendations(recent).await,
        };
        
        Ok(trends)
    }
    
    /// Generate performance recommendations
    async fn generate_recommendations(&self, snapshot: &PerformanceSnapshot) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if snapshot.redis_hit_rate < 0.8 {
            recommendations.push("Consider increasing Redis cache TTL for frequently accessed data".to_string());
        }
        
        if snapshot.average_response_time > 0.05 {
            recommendations.push("Cache response times are high, consider optimizing serialization".to_string());
        }
        
        if snapshot.error_rate > 0.01 {
            recommendations.push("High cache error rate detected, check Redis connectivity".to_string());
        }
        
        if snapshot.memory_usage_mb > 1024.0 {
            recommendations.push("Memory cache usage is high, consider reducing cache size or TTL".to_string());
        }
        
        recommendations
    }
    
    /// Get performance history
    pub async fn get_history(&self) -> Vec<PerformanceSnapshot> {
        let history = self.analysis_history.read().await;
        history.clone()
    }
}

/// Performance trends analysis
#[derive(Debug, Clone, Default)]
pub struct PerformanceTrends {
    pub hit_rate_trend: f64,
    pub response_time_trend: f64,
    pub error_rate_trend: f64,
    pub memory_usage_trend: f64,
    pub recommendations: Vec<String>,
}
