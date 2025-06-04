//! Cache Performance Testing CLI
//!
//! Command-line tool for testing and validating cache performance in FO3 Wallet Core.
//! Provides comprehensive load testing, performance analysis, and optimization recommendations.

use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::time::Duration;
use tokio;
use tracing::{info, error, Level};
use tracing_subscriber;
use serde_json;

use fo3_wallet_api::cache::{
    CacheConfig, CacheKey,
    cache_manager::CacheManager,
    load_testing::{CacheLoadTester, LoadTestConfig, KeyDistribution, ValueSizeConfig},
    metrics::CacheMetrics,
    invalidation::{CacheInvalidationManager, InvalidationEvent},
};
use fo3_wallet_api::error::ServiceError;

#[derive(Parser)]
#[command(name = "cache-perf")]
#[command(about = "FO3 Wallet Core Cache Performance Testing Tool")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Redis URL for cache testing
    #[arg(long, default_value = "redis://localhost:6379")]
    redis_url: String,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Output format (json, table, summary)
    #[arg(long, default_value = "summary")]
    output_format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run comprehensive load test
    LoadTest {
        /// Number of concurrent users
        #[arg(short, long, default_value = "10")]
        concurrent_users: usize,
        
        /// Test duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
        
        /// Target operations per second
        #[arg(short, long, default_value = "100")]
        ops_per_second: u64,
        
        /// Read/write ratio (0.0-1.0)
        #[arg(short, long, default_value = "0.8")]
        read_ratio: f64,
        
        /// Value size in bytes
        #[arg(short, long, default_value = "1024")]
        value_size: usize,
        
        /// Key distribution pattern
        #[arg(long, default_value = "uniform")]
        key_distribution: String,
    },
    
    /// Test cache invalidation performance
    InvalidationTest {
        /// Number of cache entries to create
        #[arg(short, long, default_value = "10000")]
        entries: usize,
        
        /// Invalidation pattern to test
        #[arg(short, long, default_value = "user")]
        pattern: String,
    },
    
    /// Benchmark cache operations
    Benchmark {
        /// Operation type (get, set, delete, mixed)
        #[arg(short, long, default_value = "mixed")]
        operation: String,
        
        /// Number of operations to perform
        #[arg(short, long, default_value = "10000")]
        count: u64,
        
        /// Batch size for operations
        #[arg(short, long, default_value = "100")]
        batch_size: usize,
    },
    
    /// Analyze cache performance metrics
    Metrics {
        /// Duration to collect metrics (seconds)
        #[arg(short, long, default_value = "30")]
        duration: u64,
        
        /// Metrics collection interval (seconds)
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
    
    /// Test cache warming strategies
    WarmingTest {
        /// Number of entries to warm
        #[arg(short, long, default_value = "5000")]
        entries: usize,
        
        /// Warming strategy (sequential, parallel, batch)
        #[arg(short, long, default_value = "parallel")]
        strategy: String,
    },
    
    /// Generate performance report
    Report {
        /// Include detailed analysis
        #[arg(long)]
        detailed: bool,
        
        /// Include recommendations
        #[arg(long)]
        recommendations: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("Starting FO3 Wallet Core Cache Performance Testing");
    
    // Initialize cache manager
    let cache_config = CacheConfig {
        redis_url: cli.redis_url.clone(),
        redis_pool_size: 20,
        redis_timeout_ms: 5000,
        memory_cache_size: 10000,
        memory_cache_ttl_seconds: 300,
        enable_compression: true,
        enable_encryption: false,
        default_ttl_seconds: 300,
        max_key_length: 250,
        max_value_size_bytes: 1024 * 1024,
    };
    
    let cache_manager = Arc::new(
        CacheManager::new(cache_config.clone()).await
            .map_err(|e| format!("Failed to initialize cache manager: {}", e))?
    );
    
    // Execute command
    match cli.command {
        Commands::LoadTest { 
            concurrent_users, 
            duration, 
            ops_per_second, 
            read_ratio, 
            value_size,
            key_distribution,
        } => {
            run_load_test(
                cache_manager,
                concurrent_users,
                duration,
                ops_per_second,
                read_ratio,
                value_size,
                &key_distribution,
                &cli.output_format,
            ).await?;
        }
        
        Commands::InvalidationTest { entries, pattern } => {
            run_invalidation_test(cache_manager, entries, &pattern, &cli.output_format).await?;
        }
        
        Commands::Benchmark { operation, count, batch_size } => {
            run_benchmark(cache_manager, &operation, count, batch_size, &cli.output_format).await?;
        }
        
        Commands::Metrics { duration, interval } => {
            run_metrics_collection(cache_manager, duration, interval, &cli.output_format).await?;
        }
        
        Commands::WarmingTest { entries, strategy } => {
            run_warming_test(cache_manager, entries, &strategy, &cli.output_format).await?;
        }
        
        Commands::Report { detailed, recommendations } => {
            generate_performance_report(cache_manager, detailed, recommendations, &cli.output_format).await?;
        }
    }
    
    info!("Cache performance testing completed");
    Ok(())
}

async fn run_load_test(
    cache_manager: Arc<CacheManager>,
    concurrent_users: usize,
    duration: u64,
    ops_per_second: u64,
    read_ratio: f64,
    value_size: usize,
    key_distribution: &str,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running load test: {} users, {} ops/sec, {}s duration", 
          concurrent_users, ops_per_second, duration);
    
    let distribution = match key_distribution {
        "uniform" => KeyDistribution::Uniform,
        "hotspot" => KeyDistribution::Hotspot { 
            hot_keys_percentage: 0.1, 
            hot_traffic_percentage: 0.8 
        },
        "zipfian" => KeyDistribution::Zipfian { alpha: 1.0 },
        "sequential" => KeyDistribution::Sequential,
        _ => KeyDistribution::Uniform,
    };
    
    let config = LoadTestConfig {
        concurrent_users,
        test_duration_seconds: duration,
        operations_per_second: ops_per_second,
        read_write_ratio: read_ratio,
        cache_key_distribution: distribution,
        value_size_bytes: ValueSizeConfig {
            min_bytes: value_size / 2,
            max_bytes: value_size * 2,
            average_bytes: value_size,
        },
        ramp_up_duration_seconds: 10,
        ramp_down_duration_seconds: 10,
    };
    
    let load_tester = CacheLoadTester::new(cache_manager, config);
    let results = load_tester.run_load_test().await?;
    
    // Output results
    match output_format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        "table" => {
            print_load_test_table(&results);
        }
        _ => {
            print_load_test_summary(&results);
        }
    }
    
    Ok(())
}

async fn run_invalidation_test(
    cache_manager: Arc<CacheManager>,
    entries: usize,
    pattern: &str,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running invalidation test: {} entries, pattern: {}", entries, pattern);
    
    // Populate cache with test data
    for i in 0..entries {
        let key = match pattern {
            "user" => CacheKey::UserSession(uuid::Uuid::new_v4()),
            "wallet" => CacheKey::WalletBalance(uuid::Uuid::new_v4()),
            "price" => CacheKey::AssetPrice(format!("ASSET-{}", i)),
            _ => CacheKey::UserSession(uuid::Uuid::new_v4()),
        };
        
        let value = serde_json::json!({
            "id": i,
            "data": format!("test-data-{}", i),
            "timestamp": chrono::Utc::now()
        });
        
        cache_manager.set(&key, &value, None).await?;
    }
    
    info!("Cache populated with {} entries", entries);
    
    // Test invalidation performance
    let start_time = std::time::Instant::now();
    
    let invalidation_manager = CacheInvalidationManager::new(cache_manager.clone());
    
    let event = match pattern {
        "user" => InvalidationEvent::BulkUserUpdate(
            (0..entries).map(|_| uuid::Uuid::new_v4()).collect()
        ),
        "price" => InvalidationEvent::BulkPriceUpdate(
            (0..entries).map(|i| format!("ASSET-{}", i)).collect()
        ),
        _ => InvalidationEvent::SystemMaintenance,
    };
    
    invalidation_manager.handle_event(event).await?;
    
    let invalidation_time = start_time.elapsed();
    
    // Output results
    match output_format {
        "json" => {
            let result = serde_json::json!({
                "entries": entries,
                "pattern": pattern,
                "invalidation_time_ms": invalidation_time.as_millis(),
                "entries_per_second": entries as f64 / invalidation_time.as_secs_f64()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!("Invalidation Test Results:");
            println!("  Entries: {}", entries);
            println!("  Pattern: {}", pattern);
            println!("  Time: {:?}", invalidation_time);
            println!("  Rate: {:.2} entries/sec", entries as f64 / invalidation_time.as_secs_f64());
        }
    }
    
    Ok(())
}

async fn run_benchmark(
    cache_manager: Arc<CacheManager>,
    operation: &str,
    count: u64,
    batch_size: usize,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running benchmark: {} operations of type {}", count, operation);
    
    let start_time = std::time::Instant::now();
    let mut successful_ops = 0u64;
    
    for batch_start in (0..count).step_by(batch_size) {
        let batch_end = std::cmp::min(batch_start + batch_size as u64, count);
        let mut tasks = Vec::new();
        
        for i in batch_start..batch_end {
            let cache_manager = cache_manager.clone();
            let op = operation.to_string();
            
            let task = tokio::spawn(async move {
                let key = CacheKey::UserSession(uuid::Uuid::new_v4());
                
                match op.as_str() {
                    "get" => {
                        cache_manager.get::<serde_json::Value>(&key).await.is_ok()
                    }
                    "set" => {
                        let value = serde_json::json!({"id": i, "data": "benchmark"});
                        cache_manager.set(&key, &value, None).await.is_ok()
                    }
                    "delete" => {
                        cache_manager.delete(&key).await.is_ok()
                    }
                    "mixed" => {
                        match i % 3 {
                            0 => {
                                let value = serde_json::json!({"id": i, "data": "benchmark"});
                                cache_manager.set(&key, &value, None).await.is_ok()
                            }
                            1 => cache_manager.get::<serde_json::Value>(&key).await.is_ok(),
                            _ => cache_manager.delete(&key).await.is_ok(),
                        }
                    }
                    _ => false,
                }
            });
            
            tasks.push(task);
        }
        
        // Wait for batch completion
        for task in tasks {
            if let Ok(success) = task.await {
                if success {
                    successful_ops += 1;
                }
            }
        }
    }
    
    let total_time = start_time.elapsed();
    let ops_per_second = count as f64 / total_time.as_secs_f64();
    
    // Output results
    match output_format {
        "json" => {
            let result = serde_json::json!({
                "operation": operation,
                "total_operations": count,
                "successful_operations": successful_ops,
                "total_time_ms": total_time.as_millis(),
                "operations_per_second": ops_per_second,
                "success_rate": successful_ops as f64 / count as f64
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!("Benchmark Results:");
            println!("  Operation: {}", operation);
            println!("  Total Operations: {}", count);
            println!("  Successful: {}", successful_ops);
            println!("  Time: {:?}", total_time);
            println!("  Rate: {:.2} ops/sec", ops_per_second);
            println!("  Success Rate: {:.1}%", (successful_ops as f64 / count as f64) * 100.0);
        }
    }
    
    Ok(())
}

async fn run_metrics_collection(
    cache_manager: Arc<CacheManager>,
    duration: u64,
    interval: u64,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Collecting metrics for {}s with {}s intervals", duration, interval);
    
    let mut metrics_history = Vec::new();
    let end_time = std::time::Instant::now() + Duration::from_secs(duration);
    
    while std::time::Instant::now() < end_time {
        let stats = cache_manager.get_comprehensive_stats().await?;
        metrics_history.push((chrono::Utc::now(), stats));
        
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
    
    // Output metrics
    match output_format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&metrics_history)?);
        }
        _ => {
            println!("Metrics Collection Results:");
            for (timestamp, stats) in &metrics_history {
                println!("  {}: Redis={}, Memory={}", 
                        timestamp.format("%H:%M:%S"),
                        stats.redis_available,
                        stats.memory_stats.entry_count);
            }
        }
    }
    
    Ok(())
}

async fn run_warming_test(
    cache_manager: Arc<CacheManager>,
    entries: usize,
    strategy: &str,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running cache warming test: {} entries, strategy: {}", entries, strategy);
    
    let start_time = std::time::Instant::now();
    
    match strategy {
        "sequential" => {
            for i in 0..entries {
                let key = CacheKey::UserSession(uuid::Uuid::new_v4());
                let value = serde_json::json!({"id": i, "data": "warming"});
                cache_manager.set(&key, &value, None).await?;
            }
        }
        "parallel" => {
            let mut tasks = Vec::new();
            for i in 0..entries {
                let cache_manager = cache_manager.clone();
                let task = tokio::spawn(async move {
                    let key = CacheKey::UserSession(uuid::Uuid::new_v4());
                    let value = serde_json::json!({"id": i, "data": "warming"});
                    cache_manager.set(&key, &value, None).await
                });
                tasks.push(task);
            }
            
            for task in tasks {
                let _ = task.await;
            }
        }
        "batch" => {
            let batch_size = 100;
            for batch_start in (0..entries).step_by(batch_size) {
                let batch_end = std::cmp::min(batch_start + batch_size, entries);
                let mut tasks = Vec::new();
                
                for i in batch_start..batch_end {
                    let cache_manager = cache_manager.clone();
                    let task = tokio::spawn(async move {
                        let key = CacheKey::UserSession(uuid::Uuid::new_v4());
                        let value = serde_json::json!({"id": i, "data": "warming"});
                        cache_manager.set(&key, &value, None).await
                    });
                    tasks.push(task);
                }
                
                for task in tasks {
                    let _ = task.await;
                }
            }
        }
        _ => return Err("Unknown warming strategy".into()),
    }
    
    let warming_time = start_time.elapsed();
    
    // Output results
    match output_format {
        "json" => {
            let result = serde_json::json!({
                "entries": entries,
                "strategy": strategy,
                "warming_time_ms": warming_time.as_millis(),
                "entries_per_second": entries as f64 / warming_time.as_secs_f64()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!("Cache Warming Results:");
            println!("  Entries: {}", entries);
            println!("  Strategy: {}", strategy);
            println!("  Time: {:?}", warming_time);
            println!("  Rate: {:.2} entries/sec", entries as f64 / warming_time.as_secs_f64());
        }
    }
    
    Ok(())
}

async fn generate_performance_report(
    cache_manager: Arc<CacheManager>,
    detailed: bool,
    recommendations: bool,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Generating performance report");
    
    let stats = cache_manager.get_comprehensive_stats().await?;
    let health = cache_manager.health_check().await?;
    
    match output_format {
        "json" => {
            let report = serde_json::json!({
                "stats": stats,
                "health": health,
                "detailed": detailed,
                "recommendations": recommendations
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            println!("Cache Performance Report:");
            println!("  Redis Available: {}", health.redis_healthy);
            println!("  Memory Cache Healthy: {}", health.memory_healthy);
            println!("  Overall Health: {}", health.overall_healthy);
            
            if let Some(redis_stats) = &stats.redis_stats {
                println!("  Redis Hit Rate: {:.1}%", redis_stats.hit_rate * 100.0);
                println!("  Redis Entries: {}", redis_stats.entry_count);
            }
            
            println!("  Memory Hit Rate: {:.1}%", stats.memory_stats.hit_rate * 100.0);
            println!("  Memory Entries: {}", stats.memory_stats.entry_count);
            
            if recommendations {
                println!("\nRecommendations:");
                println!("  - Monitor cache hit rates regularly");
                println!("  - Consider increasing cache TTL for stable data");
                println!("  - Implement cache warming for critical data");
            }
        }
    }
    
    Ok(())
}

fn print_load_test_summary(results: &fo3_wallet_api::cache::load_testing::LoadTestResults) {
    println!("Load Test Summary:");
    println!("  Duration: {:.1}s", (results.end_time - results.start_time).num_seconds());
    println!("  Total Operations: {}", results.total_operations);
    println!("  Operations/sec: {:.2}", results.operations_per_second);
    println!("  Average Latency: {:.2}ms", results.average_latency_ms);
    println!("  P95 Latency: {:.2}ms", results.p95_latency_ms);
    println!("  P99 Latency: {:.2}ms", results.p99_latency_ms);
    println!("  Cache Hit Rate: {:.1}%", results.cache_hit_rate * 100.0);
    println!("  Error Rate: {:.2}%", results.error_rate * 100.0);
    
    if !results.recommendations.is_empty() {
        println!("\nRecommendations:");
        for rec in &results.recommendations {
            println!("  - {}", rec);
        }
    }
}

fn print_load_test_table(results: &fo3_wallet_api::cache::load_testing::LoadTestResults) {
    println!("┌─────────────────────┬──────────────┐");
    println!("│ Metric              │ Value        │");
    println!("├─────────────────────┼──────────────┤");
    println!("│ Total Operations    │ {:>12} │", results.total_operations);
    println!("│ Operations/sec      │ {:>12.2} │", results.operations_per_second);
    println!("│ Average Latency     │ {:>9.2}ms │", results.average_latency_ms);
    println!("│ P95 Latency         │ {:>9.2}ms │", results.p95_latency_ms);
    println!("│ P99 Latency         │ {:>9.2}ms │", results.p99_latency_ms);
    println!("│ Cache Hit Rate      │ {:>9.1}% │", results.cache_hit_rate * 100.0);
    println!("│ Error Rate          │ {:>9.2}% │", results.error_rate * 100.0);
    println!("└─────────────────────┴──────────────┘");
}
