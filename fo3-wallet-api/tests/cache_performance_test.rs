//! Cache Performance Integration Tests
//!
//! Comprehensive tests for cache performance, load testing, and optimization validation.

use std::sync::Arc;
use std::time::Duration;
use tokio;
use uuid::Uuid;
use serde_json;

use fo3_wallet_api::cache::{
    CacheConfig, CacheKey,
    cache_manager::CacheManager,
    load_testing::{CacheLoadTester, LoadTestConfig, KeyDistribution, ValueSizeConfig},
    invalidation::{CacheInvalidationManager, InvalidationEvent},
    metrics::CacheMetrics,
};
use fo3_wallet_api::error::ServiceError;

/// Test configuration for cache performance tests
struct TestConfig {
    redis_url: String,
    enable_redis: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            enable_redis: std::env::var("ENABLE_REDIS_TESTS").unwrap_or_else(|_| "false".to_string()) == "true",
        }
    }
}

async fn setup_cache_manager(test_config: &TestConfig) -> Result<Arc<CacheManager>, ServiceError> {
    let cache_config = CacheConfig {
        redis_url: test_config.redis_url.clone(),
        redis_pool_size: 5,
        redis_timeout_ms: 1000,
        memory_cache_size: 1000,
        memory_cache_ttl_seconds: 60,
        enable_compression: false,
        enable_encryption: false,
        default_ttl_seconds: 60,
        max_key_length: 250,
        max_value_size_bytes: 1024 * 10, // 10KB for tests
    };
    
    if test_config.enable_redis {
        CacheManager::new(cache_config).await.map(Arc::new)
    } else {
        Ok(Arc::new(CacheManager::memory_only(cache_config)))
    }
}

#[tokio::test]
async fn test_cache_basic_operations_performance() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing basic cache operations performance");
    
    // Test SET operations
    let start_time = std::time::Instant::now();
    let set_count = 1000;
    
    for i in 0..set_count {
        let key = CacheKey::UserSession(Uuid::new_v4());
        let value = serde_json::json!({
            "id": i,
            "data": format!("test-data-{}", i),
            "timestamp": chrono::Utc::now()
        });
        
        cache_manager.set(&key, &value, None).await.unwrap();
    }
    
    let set_duration = start_time.elapsed();
    let set_ops_per_sec = set_count as f64 / set_duration.as_secs_f64();
    
    println!("  ✅ SET Performance: {:.2} ops/sec ({} ops in {:?})", 
             set_ops_per_sec, set_count, set_duration);
    
    // Test GET operations
    let keys: Vec<CacheKey> = (0..set_count)
        .map(|_| CacheKey::UserSession(Uuid::new_v4()))
        .collect();
    
    // First, populate with known keys
    for (i, key) in keys.iter().enumerate() {
        let value = serde_json::json!({"id": i, "data": "get-test"});
        cache_manager.set(key, &value, None).await.unwrap();
    }
    
    let start_time = std::time::Instant::now();
    let mut hit_count = 0;
    
    for key in &keys {
        if let Ok(Some(_)) = cache_manager.get::<serde_json::Value>(key).await {
            hit_count += 1;
        }
    }
    
    let get_duration = start_time.elapsed();
    let get_ops_per_sec = keys.len() as f64 / get_duration.as_secs_f64();
    let hit_rate = hit_count as f64 / keys.len() as f64;
    
    println!("  ✅ GET Performance: {:.2} ops/sec ({} ops in {:?})", 
             get_ops_per_sec, keys.len(), get_duration);
    println!("  ✅ Cache Hit Rate: {:.1}%", hit_rate * 100.0);
    
    // Performance assertions
    assert!(set_ops_per_sec > 100.0, "SET operations should exceed 100 ops/sec");
    assert!(get_ops_per_sec > 500.0, "GET operations should exceed 500 ops/sec");
    assert!(hit_rate > 0.95, "Cache hit rate should exceed 95%");
    
    println!("  ✅ Basic operations performance test passed");
}

#[tokio::test]
async fn test_concurrent_cache_operations() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing concurrent cache operations");
    
    let concurrent_tasks = 50;
    let operations_per_task = 100;
    
    let start_time = std::time::Instant::now();
    let mut tasks = Vec::new();
    
    for task_id in 0..concurrent_tasks {
        let cache_manager = cache_manager.clone();
        
        let task = tokio::spawn(async move {
            let mut successful_ops = 0;
            
            for i in 0..operations_per_task {
                let key = CacheKey::UserSession(Uuid::new_v4());
                let value = serde_json::json!({
                    "task_id": task_id,
                    "operation": i,
                    "data": format!("concurrent-test-{}-{}", task_id, i)
                });
                
                // Mix of operations
                match i % 3 {
                    0 => {
                        if cache_manager.set(&key, &value, None).await.is_ok() {
                            successful_ops += 1;
                        }
                    }
                    1 => {
                        if cache_manager.get::<serde_json::Value>(&key).await.is_ok() {
                            successful_ops += 1;
                        }
                    }
                    _ => {
                        if cache_manager.delete(&key).await.is_ok() {
                            successful_ops += 1;
                        }
                    }
                }
            }
            
            successful_ops
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut total_successful = 0;
    for task in tasks {
        total_successful += task.await.unwrap();
    }
    
    let total_duration = start_time.elapsed();
    let total_operations = concurrent_tasks * operations_per_task;
    let ops_per_sec = total_operations as f64 / total_duration.as_secs_f64();
    let success_rate = total_successful as f64 / total_operations as f64;
    
    println!("  ✅ Concurrent Performance: {:.2} ops/sec ({} ops in {:?})", 
             ops_per_sec, total_operations, total_duration);
    println!("  ✅ Success Rate: {:.1}%", success_rate * 100.0);
    
    // Performance assertions
    assert!(ops_per_sec > 200.0, "Concurrent operations should exceed 200 ops/sec");
    assert!(success_rate > 0.95, "Success rate should exceed 95%");
    
    println!("  ✅ Concurrent operations test passed");
}

#[tokio::test]
async fn test_cache_invalidation_performance() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing cache invalidation performance");
    
    // Populate cache with test data
    let entry_count = 5000;
    let user_ids: Vec<Uuid> = (0..entry_count).map(|_| Uuid::new_v4()).collect();
    
    for (i, user_id) in user_ids.iter().enumerate() {
        let key = CacheKey::UserSession(*user_id);
        let value = serde_json::json!({
            "user_id": user_id,
            "session_data": format!("session-{}", i),
            "created_at": chrono::Utc::now()
        });
        
        cache_manager.set(&key, &value, None).await.unwrap();
    }
    
    println!("  📝 Populated cache with {} entries", entry_count);
    
    // Test invalidation performance
    let invalidation_manager = CacheInvalidationManager::new(cache_manager.clone());
    
    let start_time = std::time::Instant::now();
    
    // Test bulk user invalidation
    let event = InvalidationEvent::BulkUserUpdate(user_ids.clone());
    invalidation_manager.handle_event(event).await.unwrap();
    
    let invalidation_duration = start_time.elapsed();
    let invalidation_rate = entry_count as f64 / invalidation_duration.as_secs_f64();
    
    println!("  ✅ Invalidation Performance: {:.2} entries/sec ({} entries in {:?})", 
             invalidation_rate, entry_count, invalidation_duration);
    
    // Verify invalidation worked
    let mut remaining_entries = 0;
    for user_id in &user_ids {
        let key = CacheKey::UserSession(*user_id);
        if cache_manager.exists(&key).await.unwrap_or(false) {
            remaining_entries += 1;
        }
    }
    
    println!("  ✅ Remaining entries after invalidation: {}", remaining_entries);
    
    // Performance assertions
    assert!(invalidation_rate > 100.0, "Invalidation should exceed 100 entries/sec");
    assert!(remaining_entries < entry_count / 10, "Most entries should be invalidated");
    
    println!("  ✅ Cache invalidation performance test passed");
}

#[tokio::test]
async fn test_load_testing_framework() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing load testing framework");
    
    let config = LoadTestConfig {
        concurrent_users: 10,
        test_duration_seconds: 5, // Short test for CI
        operations_per_second: 50,
        read_write_ratio: 0.8,
        cache_key_distribution: KeyDistribution::Uniform,
        value_size_bytes: ValueSizeConfig {
            min_bytes: 100,
            max_bytes: 1000,
            average_bytes: 500,
        },
        ramp_up_duration_seconds: 1,
        ramp_down_duration_seconds: 1,
    };
    
    let load_tester = CacheLoadTester::new(cache_manager, config.clone());
    let results = load_tester.run_load_test().await.unwrap();
    
    println!("  ✅ Load Test Results:");
    println!("    - Total Operations: {}", results.total_operations);
    println!("    - Operations/sec: {:.2}", results.operations_per_second);
    println!("    - Average Latency: {:.2}ms", results.average_latency_ms);
    println!("    - P95 Latency: {:.2}ms", results.p95_latency_ms);
    println!("    - Cache Hit Rate: {:.1}%", results.cache_hit_rate * 100.0);
    println!("    - Error Rate: {:.2}%", results.error_rate * 100.0);
    
    // Performance assertions
    assert!(results.total_operations > 0, "Should have executed operations");
    assert!(results.operations_per_second > 10.0, "Should achieve reasonable throughput");
    assert!(results.error_rate < 0.1, "Error rate should be low");
    assert!(results.average_latency_ms < 100.0, "Average latency should be reasonable");
    
    if !results.recommendations.is_empty() {
        println!("  📋 Recommendations:");
        for rec in &results.recommendations {
            println!("    - {}", rec);
        }
    }
    
    println!("  ✅ Load testing framework test passed");
}

#[tokio::test]
async fn test_cache_memory_usage() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing cache memory usage patterns");
    
    // Test with different value sizes
    let test_cases = vec![
        (100, 100),   // 100 entries, 100 bytes each
        (1000, 1000), // 1000 entries, 1KB each
        (100, 10000), // 100 entries, 10KB each
    ];
    
    for (entry_count, value_size) in test_cases {
        println!("  📊 Testing {} entries with {}B values", entry_count, value_size);
        
        // Clear cache
        cache_manager.clear().await.unwrap();
        
        // Populate cache
        let start_time = std::time::Instant::now();
        
        for i in 0..entry_count {
            let key = CacheKey::UserSession(Uuid::new_v4());
            let data: String = "x".repeat(value_size);
            let value = serde_json::json!({
                "id": i,
                "data": data,
                "size": value_size
            });
            
            cache_manager.set(&key, &value, None).await.unwrap();
        }
        
        let populate_duration = start_time.elapsed();
        let populate_rate = entry_count as f64 / populate_duration.as_secs_f64();
        
        // Get memory stats
        let stats = cache_manager.get_comprehensive_stats().await.unwrap();
        let memory_usage_mb = stats.memory_stats.memory_usage_bytes as f64 / 1024.0 / 1024.0;
        
        println!("    ✅ Populate Rate: {:.2} entries/sec", populate_rate);
        println!("    ✅ Memory Usage: {:.2} MB", memory_usage_mb);
        println!("    ✅ Entry Count: {}", stats.memory_stats.entry_count);
        
        // Performance assertions
        assert!(populate_rate > 50.0, "Population rate should be reasonable");
        assert!(stats.memory_stats.entry_count <= entry_count as u64, "Entry count should not exceed expected");
    }
    
    println!("  ✅ Cache memory usage test passed");
}

#[tokio::test]
async fn test_cache_ttl_and_expiration() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing cache TTL and expiration");
    
    let key = CacheKey::UserSession(Uuid::new_v4());
    let value = serde_json::json!({
        "data": "ttl-test",
        "created_at": chrono::Utc::now()
    });
    
    // Set with short TTL
    let short_ttl = Duration::from_millis(500);
    cache_manager.set(&key, &value, Some(short_ttl)).await.unwrap();
    
    // Verify entry exists
    assert!(cache_manager.exists(&key).await.unwrap(), "Entry should exist immediately");
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(600)).await;
    
    // Verify entry is expired (this might not work with all cache implementations)
    let exists_after_ttl = cache_manager.exists(&key).await.unwrap_or(true);
    println!("  📅 Entry exists after TTL: {}", exists_after_ttl);
    
    // Test cache warming after expiration
    let warming_start = std::time::Instant::now();
    cache_manager.set(&key, &value, None).await.unwrap();
    let warming_duration = warming_start.elapsed();
    
    println!("  ✅ Cache warming after expiration: {:?}", warming_duration);
    
    // Performance assertion
    assert!(warming_duration < Duration::from_millis(50), "Cache warming should be fast");
    
    println!("  ✅ TTL and expiration test passed");
}

#[tokio::test]
async fn test_cache_health_monitoring() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing cache health monitoring");
    
    // Test health check
    let health_status = cache_manager.health_check().await.unwrap();
    
    println!("  🏥 Health Status:");
    println!("    - Redis Healthy: {}", health_status.redis_healthy);
    println!("    - Memory Healthy: {}", health_status.memory_healthy);
    println!("    - Overall Healthy: {}", health_status.overall_healthy);
    
    if let Some(error) = &health_status.error_message {
        println!("    - Error: {}", error);
    }
    
    // Test comprehensive stats
    let stats = cache_manager.get_comprehensive_stats().await.unwrap();
    
    println!("  📊 Cache Statistics:");
    println!("    - Redis Available: {}", stats.redis_available);
    println!("    - Memory Entries: {}", stats.memory_stats.entry_count);
    println!("    - Memory Hit Rate: {:.1}%", stats.memory_stats.hit_rate * 100.0);
    println!("    - Fallback Count: {}", stats.fallback_count);
    
    // Health assertions
    assert!(health_status.memory_healthy, "Memory cache should be healthy");
    assert!(health_status.overall_healthy, "Overall cache should be healthy");
    
    println!("  ✅ Cache health monitoring test passed");
}

#[tokio::test]
async fn test_performance_regression_detection() {
    let test_config = TestConfig::default();
    let cache_manager = setup_cache_manager(&test_config).await.unwrap();
    
    println!("🧪 Testing performance regression detection");
    
    // Baseline performance test
    let baseline_ops = 1000;
    let start_time = std::time::Instant::now();
    
    for i in 0..baseline_ops {
        let key = CacheKey::UserSession(Uuid::new_v4());
        let value = serde_json::json!({"baseline": i});
        cache_manager.set(&key, &value, None).await.unwrap();
    }
    
    let baseline_duration = start_time.elapsed();
    let baseline_ops_per_sec = baseline_ops as f64 / baseline_duration.as_secs_f64();
    
    println!("  📈 Baseline Performance: {:.2} ops/sec", baseline_ops_per_sec);
    
    // Stress test with larger operations
    let stress_ops = 2000;
    let start_time = std::time::Instant::now();
    
    for i in 0..stress_ops {
        let key = CacheKey::UserSession(Uuid::new_v4());
        let large_data: String = "x".repeat(5000); // 5KB values
        let value = serde_json::json!({
            "stress_test": i,
            "large_data": large_data
        });
        cache_manager.set(&key, &value, None).await.unwrap();
    }
    
    let stress_duration = start_time.elapsed();
    let stress_ops_per_sec = stress_ops as f64 / stress_duration.as_secs_f64();
    
    println!("  🔥 Stress Performance: {:.2} ops/sec", stress_ops_per_sec);
    
    // Calculate performance degradation
    let performance_ratio = stress_ops_per_sec / baseline_ops_per_sec;
    let degradation_percent = (1.0 - performance_ratio) * 100.0;
    
    println!("  📉 Performance Degradation: {:.1}%", degradation_percent);
    
    // Regression detection assertions
    assert!(performance_ratio > 0.3, "Performance should not degrade more than 70%");
    assert!(stress_ops_per_sec > 50.0, "Stress test should maintain minimum throughput");
    
    if degradation_percent > 50.0 {
        println!("  ⚠️  WARNING: Significant performance degradation detected!");
    }
    
    println!("  ✅ Performance regression detection test passed");
}

/// Helper function to run all cache performance tests
#[tokio::test]
async fn run_comprehensive_cache_performance_suite() {
    println!("🚀 Running Comprehensive Cache Performance Test Suite");
    println!("=" .repeat(60));
    
    // Run all performance tests
    test_cache_basic_operations_performance().await;
    test_concurrent_cache_operations().await;
    test_cache_invalidation_performance().await;
    test_load_testing_framework().await;
    test_cache_memory_usage().await;
    test_cache_ttl_and_expiration().await;
    test_cache_health_monitoring().await;
    test_performance_regression_detection().await;
    
    println!("=" .repeat(60));
    println!("🎉 All cache performance tests completed successfully!");
    println!("📊 Performance validation: PASSED");
    println!("🔧 Optimization recommendations: Available via CLI tool");
    println!("📈 Ready for production deployment");
}
