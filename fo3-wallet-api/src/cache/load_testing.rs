//! Load testing framework for cache performance validation
//!
//! Provides comprehensive load testing scenarios to validate cache performance
//! under various load conditions and identify bottlenecks.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use rand::Rng;

use crate::error::ServiceError;
use super::{Cache, CacheKey, cache_manager::CacheManager};

/// Load testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    pub concurrent_users: usize,
    pub test_duration_seconds: u64,
    pub operations_per_second: u64,
    pub read_write_ratio: f64, // 0.8 = 80% reads, 20% writes
    pub cache_key_distribution: KeyDistribution,
    pub value_size_bytes: ValueSizeConfig,
    pub ramp_up_duration_seconds: u64,
    pub ramp_down_duration_seconds: u64,
}

/// Cache key distribution patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyDistribution {
    Uniform,
    Zipfian { alpha: f64 },
    Hotspot { hot_keys_percentage: f64, hot_traffic_percentage: f64 },
    Sequential,
}

/// Value size configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSizeConfig {
    pub min_bytes: usize,
    pub max_bytes: usize,
    pub average_bytes: usize,
}

/// Load test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResults {
    pub config: LoadTestConfig,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub operations_per_second: f64,
    pub average_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub max_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
    pub throughput_timeline: Vec<ThroughputDataPoint>,
    pub latency_timeline: Vec<LatencyDataPoint>,
    pub error_breakdown: HashMap<String, u64>,
    pub recommendations: Vec<String>,
}

/// Throughput data point for timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputDataPoint {
    pub timestamp: DateTime<Utc>,
    pub operations_per_second: f64,
    pub cache_hit_rate: f64,
}

/// Latency data point for timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDataPoint {
    pub timestamp: DateTime<Utc>,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
}

/// Load test operation result
#[derive(Debug, Clone)]
struct OperationResult {
    pub operation_type: OperationType,
    pub success: bool,
    pub latency: Duration,
    pub cache_hit: bool,
    pub error_type: Option<String>,
    pub timestamp: Instant,
}

/// Operation types
#[derive(Debug, Clone, PartialEq)]
enum OperationType {
    Get,
    Set,
    Delete,
    Exists,
}

/// Cache load tester
pub struct CacheLoadTester {
    cache_manager: Arc<CacheManager>,
    config: LoadTestConfig,
    results: Arc<RwLock<Vec<OperationResult>>>,
    key_generator: Arc<RwLock<KeyGenerator>>,
}

/// Key generator for different distribution patterns
struct KeyGenerator {
    distribution: KeyDistribution,
    key_pool: Vec<CacheKey>,
    hot_keys: Vec<CacheKey>,
    access_counter: u64,
}

impl CacheLoadTester {
    /// Create new load tester
    pub fn new(cache_manager: Arc<CacheManager>, config: LoadTestConfig) -> Self {
        let key_generator = KeyGenerator::new(config.clone());
        
        Self {
            cache_manager,
            config,
            results: Arc::new(RwLock::new(Vec::new())),
            key_generator: Arc::new(RwLock::new(key_generator)),
        }
    }
    
    /// Run comprehensive load test
    pub async fn run_load_test(&self) -> Result<LoadTestResults, ServiceError> {
        info!("Starting cache load test with {} concurrent users for {}s", 
              self.config.concurrent_users, self.config.test_duration_seconds);
        
        let start_time = Utc::now();
        let test_start = Instant::now();
        
        // Pre-populate cache with some data
        self.populate_initial_data().await?;
        
        // Create semaphore to control concurrency
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_users));
        
        // Run load test phases
        self.run_ramp_up_phase(&semaphore).await?;
        self.run_steady_state_phase(&semaphore).await?;
        self.run_ramp_down_phase(&semaphore).await?;
        
        let end_time = Utc::now();
        
        // Analyze results
        let results = self.analyze_results(start_time, end_time, test_start).await?;
        
        info!("Load test completed: {:.2} ops/sec, {:.2}ms avg latency, {:.1}% hit rate",
              results.operations_per_second, results.average_latency_ms, results.cache_hit_rate * 100.0);
        
        Ok(results)
    }
    
    /// Populate cache with initial data
    async fn populate_initial_data(&self) -> Result<(), ServiceError> {
        info!("Populating cache with initial data");
        
        let initial_keys = 1000;
        for i in 0..initial_keys {
            let key = CacheKey::UserSession(Uuid::new_v4());
            let value = self.generate_test_value();
            
            if let Err(e) = self.cache_manager.set(&key, &value, None).await {
                warn!("Failed to populate initial data: {}", e);
            }
        }
        
        info!("Initial data population completed");
        Ok(())
    }
    
    /// Run ramp-up phase
    async fn run_ramp_up_phase(&self, semaphore: &Arc<Semaphore>) -> Result<(), ServiceError> {
        if self.config.ramp_up_duration_seconds == 0 {
            return Ok(());
        }
        
        info!("Starting ramp-up phase: {}s", self.config.ramp_up_duration_seconds);
        
        let ramp_up_duration = Duration::from_secs(self.config.ramp_up_duration_seconds);
        let steps = 10;
        let step_duration = ramp_up_duration / steps;
        
        for step in 1..=steps {
            let target_ops = (self.config.operations_per_second * step as u64) / steps as u64;
            self.run_load_phase(step_duration, target_ops, semaphore).await?;
        }
        
        info!("Ramp-up phase completed");
        Ok(())
    }
    
    /// Run steady state phase
    async fn run_steady_state_phase(&self, semaphore: &Arc<Semaphore>) -> Result<(), ServiceError> {
        info!("Starting steady state phase: {}s", self.config.test_duration_seconds);
        
        let steady_duration = Duration::from_secs(self.config.test_duration_seconds);
        self.run_load_phase(steady_duration, self.config.operations_per_second, semaphore).await?;
        
        info!("Steady state phase completed");
        Ok(())
    }
    
    /// Run ramp-down phase
    async fn run_ramp_down_phase(&self, semaphore: &Arc<Semaphore>) -> Result<(), ServiceError> {
        if self.config.ramp_down_duration_seconds == 0 {
            return Ok(());
        }
        
        info!("Starting ramp-down phase: {}s", self.config.ramp_down_duration_seconds);
        
        let ramp_down_duration = Duration::from_secs(self.config.ramp_down_duration_seconds);
        let steps = 10;
        let step_duration = ramp_down_duration / steps;
        
        for step in (1..=steps).rev() {
            let target_ops = (self.config.operations_per_second * step as u64) / steps as u64;
            self.run_load_phase(step_duration, target_ops, semaphore).await?;
        }
        
        info!("Ramp-down phase completed");
        Ok(())
    }
    
    /// Run load phase with specific parameters
    async fn run_load_phase(
        &self,
        duration: Duration,
        target_ops_per_second: u64,
        semaphore: &Arc<Semaphore>,
    ) -> Result<(), ServiceError> {
        let end_time = Instant::now() + duration;
        let operation_interval = Duration::from_nanos(1_000_000_000 / target_ops_per_second);
        
        let mut tasks = Vec::new();
        
        while Instant::now() < end_time {
            let permit = semaphore.clone().acquire_owned().await
                .map_err(|e| ServiceError::InternalError(format!("Semaphore error: {}", e)))?;
            
            let cache_manager = self.cache_manager.clone();
            let key_generator = self.key_generator.clone();
            let results = self.results.clone();
            let config = self.config.clone();
            
            let task = tokio::spawn(async move {
                let _permit = permit; // Keep permit alive
                
                let operation_result = Self::execute_random_operation(
                    cache_manager,
                    key_generator,
                    config,
                ).await;
                
                let mut results_guard = results.write().await;
                results_guard.push(operation_result);
            });
            
            tasks.push(task);
            
            // Control operation rate
            sleep(operation_interval).await;
        }
        
        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }
        
        Ok(())
    }
    
    /// Execute random cache operation
    async fn execute_random_operation(
        cache_manager: Arc<CacheManager>,
        key_generator: Arc<RwLock<KeyGenerator>>,
        config: LoadTestConfig,
    ) -> OperationResult {
        let start_time = Instant::now();
        let mut rng = rand::thread_rng();
        
        // Determine operation type based on read/write ratio
        let operation_type = if rng.gen::<f64>() < config.read_write_ratio {
            if rng.gen::<f64>() < 0.9 {
                OperationType::Get
            } else {
                OperationType::Exists
            }
        } else {
            if rng.gen::<f64>() < 0.8 {
                OperationType::Set
            } else {
                OperationType::Delete
            }
        };
        
        // Generate key
        let key = {
            let mut generator = key_generator.write().await;
            generator.generate_key()
        };
        
        // Execute operation
        let (success, cache_hit, error_type) = match operation_type {
            OperationType::Get => {
                match cache_manager.get::<serde_json::Value>(&key).await {
                    Ok(Some(_)) => (true, true, None),
                    Ok(None) => (true, false, None),
                    Err(e) => (false, false, Some(e.to_string())),
                }
            }
            OperationType::Set => {
                let value = Self::generate_test_value_static(&config);
                match cache_manager.set(&key, &value, None).await {
                    Ok(()) => (true, false, None),
                    Err(e) => (false, false, Some(e.to_string())),
                }
            }
            OperationType::Delete => {
                match cache_manager.delete(&key).await {
                    Ok(()) => (true, false, None),
                    Err(e) => (false, false, Some(e.to_string())),
                }
            }
            OperationType::Exists => {
                match cache_manager.exists(&key).await {
                    Ok(exists) => (true, exists, None),
                    Err(e) => (false, false, Some(e.to_string())),
                }
            }
        };
        
        let latency = start_time.elapsed();
        
        OperationResult {
            operation_type,
            success,
            latency,
            cache_hit,
            error_type,
            timestamp: start_time,
        }
    }
    
    /// Generate test value
    fn generate_test_value(&self) -> serde_json::Value {
        Self::generate_test_value_static(&self.config)
    }
    
    /// Generate test value (static version)
    fn generate_test_value_static(config: &LoadTestConfig) -> serde_json::Value {
        let mut rng = rand::thread_rng();
        let size = rng.gen_range(config.value_size_bytes.min_bytes..=config.value_size_bytes.max_bytes);
        let data: String = (0..size).map(|_| rng.gen::<char>()).collect();
        
        serde_json::json!({
            "id": Uuid::new_v4(),
            "data": data,
            "timestamp": Utc::now(),
            "size": size
        })
    }
    
    /// Analyze test results
    async fn analyze_results(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        test_start: Instant,
    ) -> Result<LoadTestResults, ServiceError> {
        let results = self.results.read().await;
        
        if results.is_empty() {
            return Err(ServiceError::InternalError("No test results available".to_string()));
        }
        
        let total_operations = results.len() as u64;
        let successful_operations = results.iter().filter(|r| r.success).count() as u64;
        let failed_operations = total_operations - successful_operations;
        
        let test_duration = test_start.elapsed().as_secs_f64();
        let operations_per_second = total_operations as f64 / test_duration;
        
        // Calculate latency percentiles
        let mut latencies: Vec<f64> = results.iter()
            .map(|r| r.latency.as_secs_f64() * 1000.0)
            .collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let average_latency_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let p50_latency_ms = latencies[latencies.len() / 2];
        let p95_latency_ms = latencies[(latencies.len() as f64 * 0.95) as usize];
        let p99_latency_ms = latencies[(latencies.len() as f64 * 0.99) as usize];
        let max_latency_ms = latencies.last().copied().unwrap_or(0.0);
        
        // Calculate cache hit rate
        let cache_hits = results.iter().filter(|r| r.cache_hit).count() as u64;
        let get_operations = results.iter().filter(|r| r.operation_type == OperationType::Get).count() as u64;
        let cache_hit_rate = if get_operations > 0 { cache_hits as f64 / get_operations as f64 } else { 0.0 };
        
        let error_rate = failed_operations as f64 / total_operations as f64;
        
        // Generate error breakdown
        let mut error_breakdown = HashMap::new();
        for result in results.iter() {
            if let Some(error_type) = &result.error_type {
                *error_breakdown.entry(error_type.clone()).or_insert(0) += 1;
            }
        }
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(
            operations_per_second,
            average_latency_ms,
            cache_hit_rate,
            error_rate,
        );
        
        Ok(LoadTestResults {
            config: self.config.clone(),
            start_time,
            end_time,
            total_operations,
            successful_operations,
            failed_operations,
            operations_per_second,
            average_latency_ms,
            p50_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            max_latency_ms,
            cache_hit_rate,
            error_rate,
            throughput_timeline: Vec::new(), // Would be populated with real-time data
            latency_timeline: Vec::new(),    // Would be populated with real-time data
            error_breakdown,
            recommendations,
        })
    }
    
    /// Generate performance recommendations
    fn generate_recommendations(
        &self,
        ops_per_second: f64,
        avg_latency_ms: f64,
        hit_rate: f64,
        error_rate: f64,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if avg_latency_ms > 50.0 {
            recommendations.push("High average latency detected. Consider optimizing cache serialization or increasing cache capacity.".to_string());
        }
        
        if hit_rate < 0.7 {
            recommendations.push("Low cache hit rate. Consider increasing cache TTL or improving cache warming strategies.".to_string());
        }
        
        if error_rate > 0.01 {
            recommendations.push("High error rate detected. Check cache connectivity and error handling.".to_string());
        }
        
        if ops_per_second < self.config.operations_per_second as f64 * 0.8 {
            recommendations.push("Target throughput not achieved. Consider scaling cache infrastructure or optimizing operations.".to_string());
        }
        
        recommendations
    }
}

impl KeyGenerator {
    fn new(config: LoadTestConfig) -> Self {
        let mut key_pool = Vec::new();
        let mut hot_keys = Vec::new();
        
        // Generate key pool based on distribution
        for i in 0..10000 {
            let key = match i % 10 {
                0 => CacheKey::UserSession(Uuid::new_v4()),
                1 => CacheKey::WalletBalance(Uuid::new_v4()),
                2 => CacheKey::AssetPrice(format!("BTC-{}", i)),
                3 => CacheKey::TransactionHistory(Uuid::new_v4()),
                4 => CacheKey::KycStatus(Uuid::new_v4()),
                5 => CacheKey::CardLimits(Uuid::new_v4()),
                6 => CacheKey::MarketData(format!("ETH-{}", i)),
                7 => CacheKey::SpendingInsights(Uuid::new_v4()),
                8 => CacheKey::UserAnalytics(Uuid::new_v4()),
                _ => CacheKey::FeatureFlags(format!("flag-{}", i)),
            };
            
            key_pool.push(key.clone());
            
            // Add some keys as hot keys
            if i < 100 {
                hot_keys.push(key);
            }
        }
        
        Self {
            distribution: config.cache_key_distribution,
            key_pool,
            hot_keys,
            access_counter: 0,
        }
    }
    
    fn generate_key(&mut self) -> CacheKey {
        self.access_counter += 1;
        
        match &self.distribution {
            KeyDistribution::Uniform => {
                let index = (self.access_counter as usize) % self.key_pool.len();
                self.key_pool[index].clone()
            }
            KeyDistribution::Hotspot { hot_keys_percentage: _, hot_traffic_percentage } => {
                let mut rng = rand::thread_rng();
                if rng.gen::<f64>() < *hot_traffic_percentage {
                    let index = rng.gen_range(0..self.hot_keys.len());
                    self.hot_keys[index].clone()
                } else {
                    let index = rng.gen_range(0..self.key_pool.len());
                    self.key_pool[index].clone()
                }
            }
            KeyDistribution::Sequential => {
                let index = (self.access_counter as usize) % self.key_pool.len();
                self.key_pool[index].clone()
            }
            KeyDistribution::Zipfian { alpha: _ } => {
                // Simplified Zipfian distribution
                let mut rng = rand::thread_rng();
                let index = (rng.gen::<f64>().powf(2.0) * self.key_pool.len() as f64) as usize % self.key_pool.len();
                self.key_pool[index].clone()
            }
        }
    }
}
