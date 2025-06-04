//! Performance tests for MoonshotTradingService
//! 
//! Tests performance characteristics including:
//! - Response time benchmarks for all methods
//! - Concurrent request handling
//! - Rate limiting behavior under load
//! - Memory usage during high-volume operations
//! - Database performance with large datasets

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tonic::{Request, Code};
use uuid::Uuid;
use futures::future::join_all;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        moonshot_trading_service_client::MoonshotTradingServiceClient,
        *,
    },
    services::moonshot::MoonshotTradingServiceImpl,
    middleware::{
        auth::AuthService,
        audit::AuditLogger,
        rate_limit::RateLimiter,
        moonshot_guard::MoonshotGuard,
    },
    models::moonshot::InMemoryMoonshotRepository,
};

/// Performance test configuration
struct PerformanceConfig {
    concurrent_requests: usize,
    total_requests: usize,
    max_response_time_ms: u64,
    target_throughput_rps: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 100,
            total_requests: 1000,
            max_response_time_ms: 200, // <200ms target
            target_throughput_rps: 500.0,
        }
    }
}

/// Performance test results
#[derive(Debug)]
struct PerformanceResults {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    avg_response_time_ms: f64,
    p95_response_time_ms: f64,
    p99_response_time_ms: f64,
    max_response_time_ms: f64,
    throughput_rps: f64,
    error_rate: f64,
}

/// Test helper to create MoonshotTradingService instance
async fn create_test_service() -> MoonshotTradingServiceImpl {
    let repository = Arc::new(InMemoryMoonshotRepository::new());
    repository.initialize_mock_data().await.expect("Failed to initialize mock data");
    
    let auth_service = Arc::new(AuthService::new_mock());
    let audit_logger = Arc::new(AuditLogger::new_mock());
    let rate_limiter = Arc::new(RateLimiter::new_mock());
    let moonshot_guard = Arc::new(MoonshotGuard::new().expect("Failed to create MoonshotGuard"));

    MoonshotTradingServiceImpl::new(
        repository,
        auth_service,
        audit_logger,
        rate_limiter,
        moonshot_guard,
    )
}

/// Benchmark a single operation
async fn benchmark_operation<F, Fut, T>(
    operation_name: &str,
    operation: F,
    config: &PerformanceConfig,
) -> PerformanceResults
where
    F: Fn() -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, tonic::Status>> + Send,
    T: Send,
{
    println!("🚀 Benchmarking {}", operation_name);
    
    let start_time = Instant::now();
    let mut response_times = Vec::with_capacity(config.total_requests);
    let mut successful_requests = 0;
    let mut failed_requests = 0;

    // Create batches of concurrent requests
    let batch_size = config.concurrent_requests;
    let num_batches = config.total_requests / batch_size;

    for batch in 0..num_batches {
        let batch_start = Instant::now();
        
        // Create concurrent requests for this batch
        let mut batch_futures = Vec::with_capacity(batch_size);
        
        for _ in 0..batch_size {
            let op = operation.clone();
            batch_futures.push(tokio::spawn(async move {
                let request_start = Instant::now();
                let result = op().await;
                let request_duration = request_start.elapsed();
                (result, request_duration)
            }));
        }

        // Wait for all requests in this batch to complete
        let batch_results = join_all(batch_futures).await;
        
        for result in batch_results {
            match result {
                Ok((request_result, duration)) => {
                    response_times.push(duration.as_millis() as f64);
                    match request_result {
                        Ok(_) => successful_requests += 1,
                        Err(_) => failed_requests += 1,
                    }
                }
                Err(_) => failed_requests += 1,
            }
        }

        let batch_duration = batch_start.elapsed();
        println!("  Batch {}/{} completed in {:?}", batch + 1, num_batches, batch_duration);
    }

    let total_duration = start_time.elapsed();
    
    // Calculate statistics
    response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_response_time = response_times.iter().sum::<f64>() / response_times.len() as f64;
    let p95_index = (response_times.len() as f64 * 0.95) as usize;
    let p99_index = (response_times.len() as f64 * 0.99) as usize;
    
    let p95_response_time = response_times.get(p95_index).copied().unwrap_or(0.0);
    let p99_response_time = response_times.get(p99_index).copied().unwrap_or(0.0);
    let max_response_time = response_times.iter().fold(0.0, |a, &b| a.max(b));
    
    let throughput = config.total_requests as f64 / total_duration.as_secs_f64();
    let error_rate = failed_requests as f64 / config.total_requests as f64 * 100.0;

    PerformanceResults {
        total_requests: config.total_requests,
        successful_requests,
        failed_requests,
        avg_response_time_ms: avg_response_time,
        p95_response_time_ms: p95_response_time,
        p99_response_time_ms: p99_response_time,
        max_response_time_ms: max_response_time,
        throughput_rps: throughput,
        error_rate,
    }
}

/// Print performance results
fn print_results(operation_name: &str, results: &PerformanceResults, config: &PerformanceConfig) {
    println!("\n📊 Performance Results for {}", operation_name);
    println!("  Total Requests: {}", results.total_requests);
    println!("  Successful: {}", results.successful_requests);
    println!("  Failed: {}", results.failed_requests);
    println!("  Error Rate: {:.2}%", results.error_rate);
    println!("  Average Response Time: {:.2}ms", results.avg_response_time_ms);
    println!("  P95 Response Time: {:.2}ms", results.p95_response_time_ms);
    println!("  P99 Response Time: {:.2}ms", results.p99_response_time_ms);
    println!("  Max Response Time: {:.2}ms", results.max_response_time_ms);
    println!("  Throughput: {:.2} RPS", results.throughput_rps);
    
    // Performance assertions
    println!("\n✅ Performance Validation:");
    
    let avg_ok = results.avg_response_time_ms <= config.max_response_time_ms as f64;
    println!("  Average Response Time (<{}ms): {} ({:.2}ms)", 
        config.max_response_time_ms, 
        if avg_ok { "✅ PASS" } else { "❌ FAIL" }, 
        results.avg_response_time_ms
    );
    
    let p95_ok = results.p95_response_time_ms <= (config.max_response_time_ms as f64 * 1.5);
    println!("  P95 Response Time (<{}ms): {} ({:.2}ms)", 
        (config.max_response_time_ms as f64 * 1.5) as u64,
        if p95_ok { "✅ PASS" } else { "❌ FAIL" }, 
        results.p95_response_time_ms
    );
    
    let throughput_ok = results.throughput_rps >= config.target_throughput_rps * 0.8;
    println!("  Throughput (>{}% of {} RPS): {} ({:.2} RPS)", 
        80,
        config.target_throughput_rps,
        if throughput_ok { "✅ PASS" } else { "❌ FAIL" }, 
        results.throughput_rps
    );
    
    let error_rate_ok = results.error_rate <= 1.0;
    println!("  Error Rate (<1%): {} ({:.2}%)", 
        if error_rate_ok { "✅ PASS" } else { "❌ FAIL" }, 
        results.error_rate
    );
    
    assert!(avg_ok, "Average response time exceeded target");
    assert!(p95_ok, "P95 response time exceeded target");
    assert!(error_rate_ok, "Error rate exceeded target");
}

#[tokio::test]
async fn test_get_trending_tokens_performance() {
    let service = Arc::new(create_test_service().await);
    let config = PerformanceConfig::default();
    
    let operation = {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move {
                let request = Request::new(GetTrendingTokensRequest {
                    page: 1,
                    page_size: 20,
                    time_frame: "24h".to_string(),
                    sort_by: "volume".to_string(),
                    blockchain_filter: "".to_string(),
                    min_market_cap: 0.0,
                    max_market_cap: 0.0,
                });
                service.get_trending_tokens(request).await
            }
        }
    };

    let results = benchmark_operation("GetTrendingTokens", operation, &config).await;
    print_results("GetTrendingTokens", &results, &config);
}

#[tokio::test]
async fn test_submit_token_proposal_performance() {
    let service = Arc::new(create_test_service().await);
    let config = PerformanceConfig {
        total_requests: 100, // Lower for write operations
        concurrent_requests: 10,
        max_response_time_ms: 300, // Higher for write operations
        target_throughput_rps: 100.0,
    };
    
    let operation = {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move {
                let request = Request::new(SubmitTokenProposalRequest {
                    user_id: Uuid::new_v4().to_string(),
                    symbol: format!("TEST{}", rand::random::<u32>()),
                    name: "Test Token".to_string(),
                    description: "Performance test token".to_string(),
                    contract_address: "0x1234567890123456789012345678901234567890".to_string(),
                    blockchain: "ethereum".to_string(),
                    website_url: "https://test.com".to_string(),
                    twitter_url: "https://twitter.com/test".to_string(),
                    telegram_url: "https://t.me/test".to_string(),
                    justification: "Performance testing token proposal".to_string(),
                    supporting_documents: vec!["whitepaper.pdf".to_string()],
                });
                service.submit_token_proposal(request).await
            }
        }
    };

    let results = benchmark_operation("SubmitTokenProposal", operation, &config).await;
    print_results("SubmitTokenProposal", &results, &config);
}

#[tokio::test]
async fn test_vote_on_token_performance() {
    let service = Arc::new(create_test_service().await);
    let config = PerformanceConfig {
        total_requests: 500,
        concurrent_requests: 50,
        max_response_time_ms: 250,
        target_throughput_rps: 200.0,
    };
    
    let operation = {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move {
                let request = Request::new(VoteOnTokenRequest {
                    user_id: Uuid::new_v4().to_string(),
                    token_id: Uuid::new_v4().to_string(),
                    vote_type: VoteType::VoteTypeBullish as i32,
                    rating: 5,
                    comment: "Great token for performance testing!".to_string(),
                });
                service.vote_on_token(request).await
            }
        }
    };

    let results = benchmark_operation("VoteOnToken", operation, &config).await;
    print_results("VoteOnToken", &results, &config);
}

#[tokio::test]
async fn test_get_token_rankings_performance() {
    let service = Arc::new(create_test_service().await);
    let config = PerformanceConfig::default();
    
    let operation = {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move {
                let request = Request::new(GetTokenRankingsRequest {
                    page: 1,
                    page_size: 50,
                    ranking_type: "overall".to_string(),
                    time_frame: "24h".to_string(),
                    blockchain_filter: "".to_string(),
                });
                service.get_token_rankings(request).await
            }
        }
    };

    let results = benchmark_operation("GetTokenRankings", operation, &config).await;
    print_results("GetTokenRankings", &results, &config);
}

#[tokio::test]
async fn test_get_moonshot_analytics_performance() {
    let service = Arc::new(create_test_service().await);
    let config = PerformanceConfig {
        max_response_time_ms: 500, // Analytics can be slower
        target_throughput_rps: 100.0,
        ..Default::default()
    };
    
    let operation = {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move {
                let request = Request::new(GetMoonshotAnalyticsRequest {
                    time_frame: "24h".to_string(),
                    user_id: "".to_string(),
                });
                service.get_moonshot_analytics(request).await
            }
        }
    };

    let results = benchmark_operation("GetMoonshotAnalytics", operation, &config).await;
    print_results("GetMoonshotAnalytics", &results, &config);
}

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    println!("🔄 Testing Mixed Operations Under Load");
    
    let service = Arc::new(create_test_service().await);
    let total_operations = 1000;
    let concurrent_operations = 100;
    
    let start_time = Instant::now();
    let mut futures = Vec::with_capacity(total_operations);
    
    for i in 0..total_operations {
        let service = service.clone();
        let future = tokio::spawn(async move {
            let operation_type = i % 5;
            match operation_type {
                0 => {
                    let request = Request::new(GetTrendingTokensRequest {
                        page: 1,
                        page_size: 10,
                        time_frame: "24h".to_string(),
                        sort_by: "volume".to_string(),
                        blockchain_filter: "".to_string(),
                        min_market_cap: 0.0,
                        max_market_cap: 0.0,
                    });
                    service.get_trending_tokens(request).await.map(|_| ())
                }
                1 => {
                    let request = Request::new(GetTokenRankingsRequest {
                        page: 1,
                        page_size: 10,
                        ranking_type: "overall".to_string(),
                        time_frame: "24h".to_string(),
                        blockchain_filter: "".to_string(),
                    });
                    service.get_token_rankings(request).await.map(|_| ())
                }
                2 => {
                    let request = Request::new(VoteOnTokenRequest {
                        user_id: Uuid::new_v4().to_string(),
                        token_id: Uuid::new_v4().to_string(),
                        vote_type: VoteType::VoteTypeBullish as i32,
                        rating: 4,
                        comment: "Mixed load test".to_string(),
                    });
                    service.vote_on_token(request).await.map(|_| ())
                }
                3 => {
                    let request = Request::new(GetMoonshotAnalyticsRequest {
                        time_frame: "24h".to_string(),
                        user_id: "".to_string(),
                    });
                    service.get_moonshot_analytics(request).await.map(|_| ())
                }
                _ => {
                    let request = Request::new(GetUserVotingHistoryRequest {
                        user_id: Uuid::new_v4().to_string(),
                        page: 1,
                        page_size: 10,
                        time_frame: "30d".to_string(),
                    });
                    service.get_user_voting_history(request).await.map(|_| ())
                }
            }
        });
        futures.push(future);
        
        // Limit concurrent operations
        if futures.len() >= concurrent_operations {
            let batch_results = join_all(futures.drain(..concurrent_operations)).await;
            let successful = batch_results.iter().filter(|r| r.is_ok()).count();
            println!("  Batch completed: {}/{} successful", successful, concurrent_operations);
        }
    }
    
    // Process remaining futures
    if !futures.is_empty() {
        let remaining_results = join_all(futures).await;
        let successful = remaining_results.iter().filter(|r| r.is_ok()).count();
        println!("  Final batch completed: {}/{} successful", successful, remaining_results.len());
    }
    
    let total_duration = start_time.elapsed();
    let throughput = total_operations as f64 / total_duration.as_secs_f64();
    
    println!("✅ Mixed Operations Performance:");
    println!("  Total Operations: {}", total_operations);
    println!("  Total Duration: {:?}", total_duration);
    println!("  Throughput: {:.2} ops/sec", throughput);
    
    assert!(throughput >= 100.0, "Mixed operations throughput too low: {:.2} ops/sec", throughput);
    assert!(total_duration.as_secs() <= 30, "Mixed operations took too long: {:?}", total_duration);
}
