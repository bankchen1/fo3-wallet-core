//! Phase 5B Integration Tests
//! 
//! Tests for ML infrastructure, automated trading, and enhanced market intelligence

use std::sync::Arc;
use tokio;
use chrono::Utc;

use fo3_wallet_api::ml::{
    ModelManager, MLConfig, InferenceRequest, 
    SentimentAnalyzer, YieldPredictor, MarketPredictor, RiskAssessor, TradingSignalsGenerator
};
use fo3_wallet_api::services::automated_trading::{
    AutomatedTradingServiceImpl, CreateStrategyRequest
};
use fo3_wallet_api::middleware::{
    auth::AuthService,
    audit::AuditLogger,
    rate_limit::RateLimiter,
    trading_guard::TradingGuard,
};
use fo3_wallet_api::state::AppState;

#[tokio::test]
async fn test_ml_model_manager_initialization() {
    let config = MLConfig::default();
    let model_manager = Arc::new(ModelManager::new(config));
    
    // Test model loading
    let result = model_manager.load_model("test_sentiment", "/tmp/test_model").await;
    assert!(result.is_ok(), "Model loading should succeed");
    
    // Test model listing
    let models = model_manager.list_models().await;
    assert!(models.contains(&"test_sentiment".to_string()), "Model should be listed");
    
    // Test health check
    let health = model_manager.health_check().await;
    assert!(health.contains_key("test_sentiment"), "Health check should include loaded model");
}

#[tokio::test]
async fn test_sentiment_analyzer() {
    let analyzer = SentimentAnalyzer::new("/tmp/sentiment_model").await;
    assert!(analyzer.is_ok(), "Sentiment analyzer should initialize successfully");
    
    let analyzer = analyzer.unwrap();
    
    // Test sentiment analysis
    let result = analyzer.analyze_text("Bitcoin is going to the moon! 🚀", "twitter").await;
    assert!(result.is_ok(), "Sentiment analysis should succeed");
    
    let sentiment = result.unwrap();
    assert!(sentiment.overall_score > 0.0, "Should detect positive sentiment");
    assert!(sentiment.bullish_probability > 0.5, "Should have high bullish probability");
    assert!(sentiment.confidence > 0.0, "Should have confidence score");
}

#[tokio::test]
async fn test_yield_predictor() {
    let predictor = YieldPredictor::new("/tmp/yield_model").await;
    assert!(predictor.is_ok(), "Yield predictor should initialize successfully");
    
    let predictor = predictor.unwrap();
    
    // Test yield prediction
    let result = predictor.predict_yield("Aave", "USDC", 30).await;
    assert!(result.is_ok(), "Yield prediction should succeed");
    
    let prediction = result.unwrap();
    assert!(prediction.predicted_apy > 0.0, "Should predict positive APY");
    assert!(prediction.confidence > 0.0, "Should have confidence score");
    assert!(!prediction.risk_factors.is_empty(), "Should identify risk factors");
}

#[tokio::test]
async fn test_market_predictor() {
    let predictor = MarketPredictor::new("/tmp/market_model").await;
    assert!(predictor.is_ok(), "Market predictor should initialize successfully");
    
    let predictor = predictor.unwrap();
    
    // Test market prediction
    let horizons = vec![1, 4, 24, 168]; // 1h, 4h, 1d, 1w
    let result = predictor.predict_market("BTC", &horizons).await;
    assert!(result.is_ok(), "Market prediction should succeed");
    
    let prediction = result.unwrap();
    assert_eq!(prediction.asset, "BTC");
    assert_eq!(prediction.price_predictions.len(), horizons.len());
    assert!(prediction.current_price > 0.0, "Should have current price");
}

#[tokio::test]
async fn test_risk_assessor() {
    let assessor = RiskAssessor::new("/tmp/risk_model").await;
    assert!(assessor.is_ok(), "Risk assessor should initialize successfully");
    
    let assessor = assessor.unwrap();
    
    // Test risk assessment
    let result = assessor.assess_risk("BTC", None).await;
    assert!(result.is_ok(), "Risk assessment should succeed");
    
    let assessment = result.unwrap();
    assert!(assessment.overall_risk_score >= 0.0 && assessment.overall_risk_score <= 1.0);
    assert!(!assessment.risk_decomposition.market_risk.is_nan());
    assert!(!assessment.recommendations.is_empty(), "Should provide recommendations");
}

#[tokio::test]
async fn test_trading_signals_generator() {
    let generator = TradingSignalsGenerator::new("/tmp/signals_model").await;
    assert!(generator.is_ok(), "Trading signals generator should initialize successfully");
    
    let generator = generator.unwrap();
    
    // Create mock market data
    let market_data = fo3_wallet_api::ml::MarketDataPoint {
        symbol: "ETH".to_string(),
        price: 3000.0,
        volume: 1000000.0,
        market_cap: 360000000000.0,
        volatility: 0.025,
        timestamp: Utc::now(),
        technical_indicators: fo3_wallet_api::ml::TechnicalIndicators {
            rsi: 65.0,
            macd: 0.5,
            bollinger_upper: 3100.0,
            bollinger_lower: 2900.0,
            sma_20: 2980.0,
            ema_20: 2990.0,
            volume_sma: 800000.0,
            momentum: 5.2,
        },
    };
    
    // Test signal generation
    let result = generator.generate_signals("ETH", "1h", &market_data).await;
    assert!(result.is_ok(), "Signal generation should succeed");
    
    let signals = result.unwrap();
    assert_eq!(signals.asset, "ETH");
    assert_eq!(signals.timeframe, "1h");
    assert!(signals.signal_summary.total_signals > 0, "Should generate signals");
}

#[tokio::test]
async fn test_automated_trading_service() {
    // Initialize dependencies
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());
    let trading_guard = Arc::new(TradingGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone()
    ));
    
    let ml_config = MLConfig::default();
    let model_manager = Arc::new(ModelManager::new(ml_config));
    
    // Initialize automated trading service
    let trading_service = AutomatedTradingServiceImpl::new(
        auth_service,
        audit_logger,
        trading_guard,
        model_manager
    );
    
    // Test strategy creation
    let create_request = CreateStrategyRequest {
        strategy_type: "portfolio_rebalancing".to_string(),
        name: "Test Strategy".to_string(),
        description: "Test portfolio rebalancing strategy".to_string(),
        config: serde_json::json!({
            "target_assets": ["BTC", "ETH", "USDC"],
            "rebalance_frequency": "daily",
            "max_position_size": 0.3
        }),
        risk_parameters: serde_json::json!({
            "max_portfolio_risk": 0.2,
            "max_leverage": 2.0
        }),
    };
    
    let result = trading_service.create_strategy(create_request).await;
    assert!(result.is_ok(), "Strategy creation should succeed");
    
    let strategy = result.unwrap();
    assert!(!strategy.strategy_id.is_empty(), "Should have strategy ID");
    assert_eq!(strategy.name, "Test Strategy");
}

#[tokio::test]
async fn test_ml_inference_pipeline() {
    let config = MLConfig::default();
    let model_manager = Arc::new(ModelManager::new(config));
    
    // Load test models
    let _ = model_manager.load_model("sentiment_v1", "/tmp/sentiment").await;
    let _ = model_manager.load_model("yield_v1", "/tmp/yield").await;
    
    // Test sentiment inference
    let sentiment_request = InferenceRequest {
        model_id: "sentiment_v1".to_string(),
        input_data: serde_json::json!({
            "text": "Ethereum 2.0 is revolutionary!",
            "source": "twitter"
        }),
        request_id: "test_sentiment_1".to_string(),
        timestamp: Utc::now(),
    };
    
    let sentiment_result = model_manager.predict(sentiment_request).await;
    assert!(sentiment_result.is_ok(), "Sentiment inference should succeed");
    
    // Test yield inference
    let yield_request = InferenceRequest {
        model_id: "yield_v1".to_string(),
        input_data: serde_json::json!({
            "protocol": "Compound",
            "asset": "USDC",
            "time_horizon_days": 30
        }),
        request_id: "test_yield_1".to_string(),
        timestamp: Utc::now(),
    };
    
    let yield_result = model_manager.predict(yield_request).await;
    assert!(yield_result.is_ok(), "Yield inference should succeed");
    
    // Test batch prediction
    let batch_requests = vec![sentiment_request, yield_request];
    let batch_result = model_manager.batch_predict(batch_requests).await;
    assert!(batch_result.is_ok(), "Batch prediction should succeed");
    
    let responses = batch_result.unwrap();
    assert_eq!(responses.len(), 2, "Should return two responses");
}

#[tokio::test]
async fn test_trading_guard_validation() {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());
    
    let trading_guard = TradingGuard::new(
        auth_service,
        audit_logger,
        rate_limiter
    );
    
    // Test user limits setup
    let user_limits = fo3_wallet_api::middleware::trading_guard::UserTradingLimits {
        user_id: "test_user".to_string(),
        tier: fo3_wallet_api::middleware::trading_guard::TradingTier::Basic,
        daily_trade_limit: 50,
        max_position_size: rust_decimal::Decimal::from(1000),
        max_portfolio_value: rust_decimal::Decimal::from(10000),
        allowed_assets: vec!["BTC".to_string(), "ETH".to_string()],
        restricted_strategies: vec![],
        risk_tolerance: fo3_wallet_api::middleware::trading_guard::RiskTolerance::Conservative,
        last_updated: Utc::now(),
    };
    
    let result = trading_guard.set_user_limits("test_user", user_limits).await;
    assert!(result.is_ok(), "Setting user limits should succeed");
    
    // Test market conditions update
    let market_conditions = fo3_wallet_api::middleware::trading_guard::MarketConditions {
        volatility_index: 0.25,
        liquidity_index: 0.8,
        market_stress_level: fo3_wallet_api::middleware::trading_guard::StressLevel::Low,
        circuit_breaker_active: false,
        trading_halted: false,
        last_updated: Utc::now(),
    };
    
    let result = trading_guard.update_market_conditions(market_conditions).await;
    assert!(result.is_ok(), "Updating market conditions should succeed");
}

#[tokio::test]
async fn test_feature_engineering() {
    use fo3_wallet_api::ml::feature_engineering::{FeatureEngineer, FeatureEngineeringConfig};
    use fo3_wallet_api::ml::{MarketDataPoint, TechnicalIndicators};
    
    let config = FeatureEngineeringConfig::default();
    let mut feature_engineer = FeatureEngineer::new(config);
    
    // Create mock market data
    let market_data = vec![
        MarketDataPoint {
            symbol: "BTC".to_string(),
            price: 45000.0,
            volume: 1000000.0,
            market_cap: 900000000000.0,
            volatility: 0.025,
            timestamp: Utc::now(),
            technical_indicators: TechnicalIndicators {
                rsi: 65.0,
                macd: 0.5,
                bollinger_upper: 46000.0,
                bollinger_lower: 44000.0,
                sma_20: 44800.0,
                ema_20: 44900.0,
                volume_sma: 800000.0,
                momentum: 5.2,
            },
        }
    ];
    
    // Test feature extraction
    let result = feature_engineer.extract_features(&market_data, None).await;
    assert!(result.is_ok(), "Feature extraction should succeed");
    
    let features = result.unwrap();
    assert!(!features.features.is_empty(), "Should extract features");
    assert!(!features.feature_names.is_empty(), "Should have feature names");
    assert_eq!(features.features.len(), features.feature_names.len(), "Features and names should match");
}

#[tokio::test]
async fn test_data_pipeline() {
    use fo3_wallet_api::ml::data_pipeline::{DataPipeline, DataPipelineConfig};
    
    let config = DataPipelineConfig::default();
    let pipeline = DataPipeline::new(config).await;
    assert!(pipeline.is_ok(), "Data pipeline should initialize successfully");
    
    let pipeline = pipeline.unwrap();
    
    // Test empty data processing
    let result = pipeline.process_data(vec![]).await;
    assert!(result.is_ok(), "Empty data processing should succeed");
    
    let pipeline_result = result.unwrap();
    assert_eq!(pipeline_result.input_records, 0);
    assert_eq!(pipeline_result.output_records, 0);
}

#[tokio::test]
async fn test_end_to_end_ml_workflow() {
    // This test simulates a complete ML workflow from data ingestion to trading signals
    
    // 1. Initialize ML infrastructure
    let config = MLConfig::default();
    let model_manager = Arc::new(ModelManager::new(config));
    
    // 2. Load models
    let _ = model_manager.load_model("sentiment_v1", "/tmp/sentiment").await;
    let _ = model_manager.load_model("market_predictor_v1", "/tmp/market").await;
    let _ = model_manager.load_model("trading_signals_v1", "/tmp/signals").await;
    
    // 3. Generate market prediction
    let market_request = InferenceRequest {
        model_id: "market_predictor_v1".to_string(),
        input_data: serde_json::json!({
            "asset": "BTC",
            "horizons": [1, 4, 24]
        }),
        request_id: "market_pred_1".to_string(),
        timestamp: Utc::now(),
    };
    
    let market_result = model_manager.predict(market_request).await;
    assert!(market_result.is_ok(), "Market prediction should succeed");
    
    // 4. Generate trading signals
    let signals_request = InferenceRequest {
        model_id: "trading_signals_v1".to_string(),
        input_data: serde_json::json!({
            "asset": "BTC",
            "timeframe": "1h"
        }),
        request_id: "signals_1".to_string(),
        timestamp: Utc::now(),
    };
    
    let signals_result = model_manager.predict(signals_request).await;
    assert!(signals_result.is_ok(), "Trading signals should succeed");
    
    // 5. Verify workflow completion
    let market_response = market_result.unwrap();
    let signals_response = signals_result.unwrap();
    
    assert!(market_response.processing_time_ms > 0, "Should have processing time");
    assert!(signals_response.processing_time_ms > 0, "Should have processing time");
    assert!(market_response.confidence > 0.0, "Should have confidence");
    assert!(signals_response.confidence > 0.0, "Should have confidence");
}
