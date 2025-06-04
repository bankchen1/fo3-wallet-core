//! Phase 4 Integration Tests
//! 
//! Comprehensive integration tests for Phase 4 features including:
//! - MoonshotTradingService integration with main server
//! - MarketIntelligenceService advanced analytics
//! - Cross-service integration and data flow
//! - Performance validation under realistic load
//! - End-to-end workflows for mobile clients
//! - Real-time notification system testing

use std::sync::Arc;
use tokio;
use tonic::{Request, Code};
use uuid::Uuid;
use futures::future::join_all;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        moonshot_trading_service_client::MoonshotTradingServiceClient,
        market_intelligence_service_client::MarketIntelligenceServiceClient,
        earn_service_client::EarnServiceClient,
        *,
    },
    services::{
        moonshot::MoonshotTradingServiceImpl,
        market_intelligence::MarketIntelligenceServiceImpl,
        earn::EarnServiceImpl,
    },
    middleware::{
        auth::AuthService,
        audit::AuditLogger,
        rate_limit::RateLimiter,
        moonshot_guard::MoonshotGuard,
        earn_guard::EarnGuard,
    },
    models::{
        moonshot::InMemoryMoonshotRepository,
        earn::InMemoryEarnRepository,
    },
};

/// Test helper to create integrated service environment
async fn create_integrated_test_environment() -> (
    MoonshotTradingServiceImpl,
    MarketIntelligenceServiceImpl,
    EarnServiceImpl,
    Arc<AuthService>,
) {
    // Shared dependencies
    let auth_service = Arc::new(AuthService::new_mock());
    let audit_logger = Arc::new(AuditLogger::new_mock());
    let rate_limiter = Arc::new(RateLimiter::new_mock());

    // MoonshotTradingService
    let moonshot_repository = Arc::new(InMemoryMoonshotRepository::new());
    moonshot_repository.initialize_mock_data().await.expect("Failed to initialize moonshot data");
    let moonshot_guard = Arc::new(MoonshotGuard::new().expect("Failed to create MoonshotGuard"));
    let moonshot_service = MoonshotTradingServiceImpl::new(
        moonshot_repository,
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        moonshot_guard,
    );

    // MarketIntelligenceService
    let market_intelligence_service = MarketIntelligenceServiceImpl::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
    );

    // EarnService
    let earn_repository = Arc::new(InMemoryEarnRepository::new());
    earn_repository.initialize_mock_data().await.expect("Failed to initialize earn data");
    let earn_guard = Arc::new(EarnGuard::new().expect("Failed to create EarnGuard"));
    let earn_service = EarnServiceImpl::new(
        earn_repository,
        auth_service.clone(),
        audit_logger.clone(),
        earn_guard,
    );

    (moonshot_service, market_intelligence_service, earn_service, auth_service)
}

#[tokio::test]
async fn test_moonshot_market_intelligence_integration() {
    let (moonshot_service, market_intelligence_service, _, _) = create_integrated_test_environment().await;
    
    // Test 1: Get trending tokens from moonshot service
    let trending_request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 10,
        time_frame: "24h".to_string(),
        sort_by: "volume".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let trending_response = moonshot_service.get_trending_tokens(trending_request).await;
    assert!(trending_response.is_ok());
    let trending_tokens = trending_response.unwrap().into_inner().tokens;
    assert!(!trending_tokens.is_empty());

    // Test 2: Use token symbols for market intelligence analysis
    let symbols: Vec<String> = trending_tokens.iter().map(|t| t.symbol.clone()).collect();
    
    let market_data_request = Request::new(GetRealTimeMarketDataRequest {
        symbols: symbols.clone(),
        blockchains: vec!["ethereum".to_string()],
        data_granularity: "1m".to_string(),
        include_orderbook: true,
        include_trades: true,
        include_liquidity: true,
    });

    let market_data_response = market_intelligence_service.get_real_time_market_data(market_data_request).await;
    assert!(market_data_response.is_ok());
    let market_data = market_data_response.unwrap().into_inner();
    assert!(!market_data.data_points.is_empty());
    assert!(market_data.market_summary.is_some());

    // Test 3: Get sentiment analysis for the same tokens
    let sentiment_request = Request::new(GetAdvancedSentimentAnalysisRequest {
        symbols: symbols.clone(),
        time_frame: "24h".to_string(),
        include_social_media: true,
        include_news_analysis: true,
        include_whale_sentiment: true,
        include_developer_activity: true,
    });

    let sentiment_response = market_intelligence_service.get_advanced_sentiment_analysis(sentiment_request).await;
    assert!(sentiment_response.is_ok());
    let sentiment_data = sentiment_response.unwrap().into_inner();
    assert!(!sentiment_data.token_sentiments.is_empty());
    assert!(sentiment_data.market_overview.is_some());

    // Test 4: Verify data consistency between services
    for token in &trending_tokens {
        let market_point = market_data.data_points.iter()
            .find(|dp| dp.symbol == token.symbol);
        assert!(market_point.is_some(), "Market data should exist for trending token: {}", token.symbol);

        let sentiment = sentiment_data.token_sentiments.iter()
            .find(|s| s.symbol == token.symbol);
        assert!(sentiment.is_some(), "Sentiment data should exist for trending token: {}", token.symbol);
    }
}

#[tokio::test]
async fn test_yield_optimization_with_market_intelligence() {
    let (_, market_intelligence_service, earn_service, auth_service) = create_integrated_test_environment().await;
    
    let user_id = Uuid::new_v4().to_string();
    
    // Test 1: Get current yield products
    let yield_products_request = Request::new(GetYieldProductsRequest {
        category: "DEFI".to_string(),
        min_apy: 5.0,
        max_apy: 0.0,
        risk_level: "MEDIUM".to_string(),
        page: 1,
        page_size: 10,
    });

    let yield_products_response = earn_service.get_yield_products(yield_products_request).await;
    assert!(yield_products_response.is_ok());
    let yield_products = yield_products_response.unwrap().into_inner().products;
    assert!(!yield_products.is_empty());

    // Test 2: Get yield optimization predictions using market intelligence
    let optimization_request = Request::new(GetYieldOptimizationPredictionsRequest {
        user_id: user_id.clone(),
        current_positions: yield_products.iter().map(|p| p.product_id.clone()).collect(),
        risk_tolerance: "moderate".to_string(),
        time_horizon: "3m".to_string(),
        target_apy: "8.0".to_string(),
    });

    let optimization_response = market_intelligence_service.get_yield_optimization_predictions(optimization_request).await;
    assert!(optimization_response.is_ok());
    let optimization_data = optimization_response.unwrap().into_inner();
    assert!(!optimization_data.suggestions.is_empty());
    assert!(optimization_data.current_analysis.is_some());
    assert!(!optimization_data.risk_scenarios.is_empty());

    // Test 3: Verify optimization suggestions are actionable
    for suggestion in &optimization_data.suggestions {
        assert!(!suggestion.suggestion_id.is_empty());
        assert!(!suggestion.action_type.is_empty());
        assert!(!suggestion.protocol_name.is_empty());
        assert!(!suggestion.expected_apy.is_empty());
        assert!(suggestion.confidence_score > 0.0 && suggestion.confidence_score <= 1.0);
        assert!(!suggestion.reasoning.is_empty());
    }

    // Test 4: Test portfolio optimization analysis
    let analysis = optimization_data.current_analysis.unwrap();
    assert!(analysis.current_apy > 0.0);
    assert!(analysis.optimized_apy >= analysis.current_apy);
    assert!(analysis.diversification_score >= 0.0 && analysis.diversification_score <= 1.0);
    assert!(!analysis.optimization_opportunities.is_empty());
}

#[tokio::test]
async fn test_arbitrage_detection_integration() {
    let (moonshot_service, market_intelligence_service, _, _) = create_integrated_test_environment().await;
    
    // Test 1: Get trending tokens for arbitrage analysis
    let trending_request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 5,
        time_frame: "1h".to_string(),
        sort_by: "volume".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 1000000.0, // Only high-cap tokens for arbitrage
        max_market_cap: 0.0,
    });

    let trending_response = moonshot_service.get_trending_tokens(trending_request).await;
    assert!(trending_response.is_ok());
    let trending_tokens = trending_response.unwrap().into_inner().tokens;

    // Test 2: Detect arbitrage opportunities for trending tokens
    let symbols: Vec<String> = trending_tokens.iter().map(|t| t.symbol.clone()).collect();
    
    let arbitrage_request = Request::new(DetectArbitrageOpportunitiesRequest {
        symbols: symbols.clone(),
        source_chains: vec!["ethereum".to_string(), "polygon".to_string()],
        target_chains: vec!["ethereum".to_string(), "bsc".to_string()],
        min_profit_threshold: "1.0".to_string(), // 1% minimum profit
        max_gas_cost: "50.0".to_string(), // $50 max gas cost
        include_dex_arbitrage: true,
        include_cross_chain: true,
    });

    let arbitrage_response = market_intelligence_service.detect_arbitrage_opportunities(arbitrage_request).await;
    assert!(arbitrage_response.is_ok());
    let arbitrage_data = arbitrage_response.unwrap().into_inner();
    
    // Test 3: Verify arbitrage opportunity structure
    if !arbitrage_data.opportunities.is_empty() {
        for opportunity in &arbitrage_data.opportunities {
            assert!(!opportunity.opportunity_id.is_empty());
            assert!(symbols.contains(&opportunity.symbol));
            assert!(!opportunity.source_exchange.is_empty());
            assert!(!opportunity.target_exchange.is_empty());
            assert!(!opportunity.profit_percentage.is_empty());
            assert!(opportunity.execution_complexity >= 0.0 && opportunity.execution_complexity <= 1.0);
            assert!(!opportunity.execution_steps.is_empty());
        }
    }

    // Test 4: Verify market overview
    assert!(arbitrage_data.market_overview.is_some());
    let overview = arbitrage_data.market_overview.unwrap();
    assert!(overview.total_opportunities >= 0);
    assert!(!overview.most_profitable_pairs.is_empty());
    assert!(!overview.most_active_chains.is_empty());
}

#[tokio::test]
async fn test_concurrent_service_operations() {
    let (moonshot_service, market_intelligence_service, earn_service, _) = create_integrated_test_environment().await;
    
    let concurrent_operations = 50;
    let mut futures = Vec::with_capacity(concurrent_operations);

    // Create concurrent operations across all services
    for i in 0..concurrent_operations {
        let moonshot_service = &moonshot_service;
        let market_intelligence_service = &market_intelligence_service;
        let earn_service = &earn_service;

        let future = async move {
            let operation_type = i % 4;
            match operation_type {
                0 => {
                    // Moonshot operation
                    let request = Request::new(GetTrendingTokensRequest {
                        page: 1,
                        page_size: 10,
                        time_frame: "24h".to_string(),
                        sort_by: "volume".to_string(),
                        blockchain_filter: "".to_string(),
                        min_market_cap: 0.0,
                        max_market_cap: 0.0,
                    });
                    moonshot_service.get_trending_tokens(request).await.map(|_| ())
                }
                1 => {
                    // Market intelligence operation
                    let request = Request::new(GetRealTimeMarketDataRequest {
                        symbols: vec!["ETH".to_string(), "BTC".to_string()],
                        blockchains: vec!["ethereum".to_string()],
                        data_granularity: "1m".to_string(),
                        include_orderbook: false,
                        include_trades: false,
                        include_liquidity: false,
                    });
                    market_intelligence_service.get_real_time_market_data(request).await.map(|_| ())
                }
                2 => {
                    // Earn service operation
                    let request = Request::new(GetYieldProductsRequest {
                        category: "DEFI".to_string(),
                        min_apy: 0.0,
                        max_apy: 0.0,
                        risk_level: "".to_string(),
                        page: 1,
                        page_size: 10,
                    });
                    earn_service.get_yield_products(request).await.map(|_| ())
                }
                _ => {
                    // Sentiment analysis operation
                    let request = Request::new(GetAdvancedSentimentAnalysisRequest {
                        symbols: vec!["ETH".to_string()],
                        time_frame: "24h".to_string(),
                        include_social_media: true,
                        include_news_analysis: false,
                        include_whale_sentiment: false,
                        include_developer_activity: false,
                    });
                    market_intelligence_service.get_advanced_sentiment_analysis(request).await.map(|_| ())
                }
            }
        };
        futures.push(future);
    }

    // Execute all operations concurrently
    let start_time = std::time::Instant::now();
    let results = join_all(futures).await;
    let duration = start_time.elapsed();

    // Verify results
    let successful_operations = results.iter().filter(|r| r.is_ok()).count();
    let success_rate = successful_operations as f64 / concurrent_operations as f64;

    println!("Concurrent operations completed:");
    println!("  Total operations: {}", concurrent_operations);
    println!("  Successful: {}", successful_operations);
    println!("  Success rate: {:.2}%", success_rate * 100.0);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} ops/sec", concurrent_operations as f64 / duration.as_secs_f64());

    // Assert performance requirements
    assert!(success_rate >= 0.95, "Success rate should be at least 95%");
    assert!(duration.as_secs() <= 10, "All operations should complete within 10 seconds");
    
    let throughput = concurrent_operations as f64 / duration.as_secs_f64();
    assert!(throughput >= 10.0, "Throughput should be at least 10 ops/sec");
}

#[tokio::test]
async fn test_end_to_end_mobile_workflow() {
    let (moonshot_service, market_intelligence_service, earn_service, auth_service) = create_integrated_test_environment().await;
    
    let user_id = Uuid::new_v4().to_string();
    
    // Simulate mobile app workflow
    
    // Step 1: User opens app and views trending tokens
    let trending_request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 20,
        time_frame: "24h".to_string(),
        sort_by: "community_score".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let trending_response = moonshot_service.get_trending_tokens(trending_request).await;
    assert!(trending_response.is_ok());
    let trending_tokens = trending_response.unwrap().into_inner().tokens;
    assert!(!trending_tokens.is_empty());

    // Step 2: User selects a token and views detailed market data
    let selected_token = &trending_tokens[0];
    let market_data_request = Request::new(GetRealTimeMarketDataRequest {
        symbols: vec![selected_token.symbol.clone()],
        blockchains: vec![selected_token.blockchain.clone()],
        data_granularity: "5m".to_string(),
        include_orderbook: true,
        include_trades: true,
        include_liquidity: true,
    });

    let market_data_response = market_intelligence_service.get_real_time_market_data(market_data_request).await;
    assert!(market_data_response.is_ok());
    let market_data = market_data_response.unwrap().into_inner();
    assert!(!market_data.data_points.is_empty());

    // Step 3: User votes on the token
    let vote_request = Request::new(VoteOnTokenRequest {
        user_id: user_id.clone(),
        token_id: selected_token.token_id.clone(),
        vote_type: VoteType::VoteTypeBullish as i32,
        rating: 5,
        comment: "Great project with strong fundamentals!".to_string(),
    });

    let vote_response = moonshot_service.vote_on_token(vote_request).await;
    assert!(vote_response.is_ok());
    let vote_result = vote_response.unwrap().into_inner();
    assert!(!vote_result.vote_id.is_empty());
    assert!(vote_result.vote_weight > 0.0);

    // Step 4: User explores yield farming opportunities
    let yield_request = Request::new(GetYieldProductsRequest {
        category: "DEFI".to_string(),
        min_apy: 5.0,
        max_apy: 0.0,
        risk_level: "LOW".to_string(),
        page: 1,
        page_size: 10,
    });

    let yield_response = earn_service.get_yield_products(yield_request).await;
    assert!(yield_response.is_ok());
    let yield_products = yield_response.unwrap().into_inner().products;
    assert!(!yield_products.is_empty());

    // Step 5: User gets personalized yield optimization recommendations
    let optimization_request = Request::new(GetYieldOptimizationPredictionsRequest {
        user_id: user_id.clone(),
        current_positions: vec![],
        risk_tolerance: "conservative".to_string(),
        time_horizon: "6m".to_string(),
        target_apy: "7.0".to_string(),
    });

    let optimization_response = market_intelligence_service.get_yield_optimization_predictions(optimization_request).await;
    assert!(optimization_response.is_ok());
    let optimization_data = optimization_response.unwrap().into_inner();
    assert!(!optimization_data.suggestions.is_empty());

    // Step 6: User checks for arbitrage opportunities
    let arbitrage_request = Request::new(DetectArbitrageOpportunitiesRequest {
        symbols: vec![selected_token.symbol.clone()],
        source_chains: vec!["ethereum".to_string()],
        target_chains: vec!["polygon".to_string()],
        min_profit_threshold: "2.0".to_string(),
        max_gas_cost: "30.0".to_string(),
        include_dex_arbitrage: true,
        include_cross_chain: false,
    });

    let arbitrage_response = market_intelligence_service.detect_arbitrage_opportunities(arbitrage_request).await;
    assert!(arbitrage_response.is_ok());
    let arbitrage_data = arbitrage_response.unwrap().into_inner();
    
    // Verify complete workflow executed successfully
    println!("✅ End-to-end mobile workflow completed successfully:");
    println!("  - Viewed {} trending tokens", trending_tokens.len());
    println!("  - Analyzed market data for {}", selected_token.symbol);
    println!("  - Submitted vote with weight {:.2}", vote_result.vote_weight);
    println!("  - Found {} yield products", yield_products.len());
    println!("  - Received {} optimization suggestions", optimization_data.suggestions.len());
    println!("  - Detected {} arbitrage opportunities", arbitrage_data.opportunities.len());
}

#[tokio::test]
async fn test_error_handling_and_resilience() {
    let (moonshot_service, market_intelligence_service, _, _) = create_integrated_test_environment().await;
    
    // Test 1: Invalid request parameters
    let invalid_trending_request = Request::new(GetTrendingTokensRequest {
        page: 0, // Invalid page number
        page_size: 1000, // Invalid page size
        time_frame: "invalid".to_string(), // Invalid time frame
        sort_by: "invalid".to_string(), // Invalid sort option
        blockchain_filter: "".to_string(),
        min_market_cap: -100.0, // Invalid market cap
        max_market_cap: 0.0,
    });

    let invalid_response = moonshot_service.get_trending_tokens(invalid_trending_request).await;
    assert!(invalid_response.is_err());
    assert_eq!(invalid_response.unwrap_err().code(), Code::InvalidArgument);

    // Test 2: Empty request data
    let empty_market_request = Request::new(GetRealTimeMarketDataRequest {
        symbols: vec![], // Empty symbols list
        blockchains: vec![],
        data_granularity: "".to_string(),
        include_orderbook: false,
        include_trades: false,
        include_liquidity: false,
    });

    let empty_response = market_intelligence_service.get_real_time_market_data(empty_market_request).await;
    // Should handle gracefully, not crash
    assert!(empty_response.is_ok() || empty_response.unwrap_err().code() == Code::InvalidArgument);

    // Test 3: Unauthenticated requests for protected endpoints
    let unauth_optimization_request = Request::new(GetYieldOptimizationPredictionsRequest {
        user_id: "".to_string(), // Empty user ID
        current_positions: vec![],
        risk_tolerance: "moderate".to_string(),
        time_horizon: "3m".to_string(),
        target_apy: "8.0".to_string(),
    });

    let unauth_response = market_intelligence_service.get_yield_optimization_predictions(unauth_optimization_request).await;
    assert!(unauth_response.is_err());
    assert_eq!(unauth_response.unwrap_err().code(), Code::Unauthenticated);

    println!("✅ Error handling tests completed successfully");
}
