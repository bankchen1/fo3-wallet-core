//! Integration tests for MoonshotTradingService
//! 
//! Tests comprehensive functionality including:
//! - Trending tokens retrieval with pagination and filtering
//! - Token proposal submission and validation
//! - Voting system with weighted votes and anti-spam measures
//! - Token rankings with multiple algorithms
//! - Analytics and sentiment analysis
//! - User voting history and proposal management
//! - Rate limiting and authentication

use std::sync::Arc;
use tokio;
use tonic::{Request, Code};
use uuid::Uuid;

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

#[tokio::test]
async fn test_get_trending_tokens_success() {
    let service = create_test_service().await;
    
    let request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 10,
        time_frame: "24h".to_string(),
        sort_by: "volume".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.tokens.is_empty());
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 10);
    assert!(response.total_count > 0);
}

#[tokio::test]
async fn test_get_trending_tokens_pagination() {
    let service = create_test_service().await;
    
    // Test first page
    let request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 5,
        time_frame: "".to_string(),
        sort_by: "".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.tokens.len(), 5);
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 5);
    assert!(response.has_next_page);

    // Test second page
    let request = Request::new(GetTrendingTokensRequest {
        page: 2,
        page_size: 5,
        time_frame: "".to_string(),
        sort_by: "".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.page, 2);
    assert_eq!(response.page_size, 5);
}

#[tokio::test]
async fn test_get_trending_tokens_invalid_parameters() {
    let service = create_test_service().await;
    
    // Test invalid page
    let request = Request::new(GetTrendingTokensRequest {
        page: 0,
        page_size: 10,
        time_frame: "".to_string(),
        sort_by: "".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);

    // Test invalid page size
    let request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 0,
        time_frame: "".to_string(),
        sort_by: "".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);

    // Test invalid time frame
    let request = Request::new(GetTrendingTokensRequest {
        page: 1,
        page_size: 10,
        time_frame: "invalid".to_string(),
        sort_by: "".to_string(),
        blockchain_filter: "".to_string(),
        min_market_cap: 0.0,
        max_market_cap: 0.0,
    });

    let response = service.get_trending_tokens(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_submit_token_proposal_success() {
    let service = create_test_service().await;
    
    let request = Request::new(SubmitTokenProposalRequest {
        user_id: Uuid::new_v4().to_string(),
        symbol: "NEWTOKEN".to_string(),
        name: "New Test Token".to_string(),
        description: "A revolutionary new token for testing purposes".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        blockchain: "ethereum".to_string(),
        website_url: "https://newtoken.com".to_string(),
        twitter_url: "https://twitter.com/newtoken".to_string(),
        telegram_url: "https://t.me/newtoken".to_string(),
        justification: "This token brings innovative features to the DeFi ecosystem".to_string(),
        supporting_documents: vec!["whitepaper.pdf".to_string(), "tokenomics.pdf".to_string()],
    });

    let response = service.submit_token_proposal(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.proposal_id.is_empty());
    assert_eq!(response.status, ProposalStatus::ProposalStatusVoting as i32);
    assert!(response.voting_ends_at.is_some());
    assert!(!response.message.is_empty());
}

#[tokio::test]
async fn test_submit_token_proposal_validation_errors() {
    let service = create_test_service().await;
    
    // Test empty symbol
    let request = Request::new(SubmitTokenProposalRequest {
        user_id: Uuid::new_v4().to_string(),
        symbol: "".to_string(),
        name: "Test Token".to_string(),
        description: "Test description".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        blockchain: "ethereum".to_string(),
        website_url: "".to_string(),
        twitter_url: "".to_string(),
        telegram_url: "".to_string(),
        justification: "Test justification".to_string(),
        supporting_documents: vec![],
    });

    let response = service.submit_token_proposal(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);

    // Test invalid blockchain
    let request = Request::new(SubmitTokenProposalRequest {
        user_id: Uuid::new_v4().to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        description: "Test description".to_string(),
        contract_address: "0x1234567890123456789012345678901234567890".to_string(),
        blockchain: "invalid_blockchain".to_string(),
        website_url: "".to_string(),
        twitter_url: "".to_string(),
        telegram_url: "".to_string(),
        justification: "Test justification".to_string(),
        supporting_documents: vec![],
    });

    let response = service.submit_token_proposal(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);

    // Test invalid contract address
    let request = Request::new(SubmitTokenProposalRequest {
        user_id: Uuid::new_v4().to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        description: "Test description".to_string(),
        contract_address: "invalid_address".to_string(),
        blockchain: "ethereum".to_string(),
        website_url: "".to_string(),
        twitter_url: "".to_string(),
        telegram_url: "".to_string(),
        justification: "Test justification".to_string(),
        supporting_documents: vec![],
    });

    let response = service.submit_token_proposal(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_vote_on_token_success() {
    let service = create_test_service().await;
    
    let request = Request::new(VoteOnTokenRequest {
        user_id: Uuid::new_v4().to_string(),
        token_id: Uuid::new_v4().to_string(),
        vote_type: VoteType::VoteTypeBullish as i32,
        rating: 5,
        comment: "Great token with strong fundamentals!".to_string(),
    });

    let response = service.vote_on_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.vote_id.is_empty());
    assert!(response.vote_weight > 0.0);
    assert!(response.updated_metrics.is_some());
    assert!(!response.message.is_empty());
}

#[tokio::test]
async fn test_vote_on_token_validation_errors() {
    let service = create_test_service().await;
    
    // Test empty user ID
    let request = Request::new(VoteOnTokenRequest {
        user_id: "".to_string(),
        token_id: Uuid::new_v4().to_string(),
        vote_type: VoteType::VoteTypeBullish as i32,
        rating: 5,
        comment: "Test comment".to_string(),
    });

    let response = service.vote_on_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::Unauthenticated);

    // Test invalid rating
    let request = Request::new(VoteOnTokenRequest {
        user_id: Uuid::new_v4().to_string(),
        token_id: Uuid::new_v4().to_string(),
        vote_type: VoteType::VoteTypeBullish as i32,
        rating: 0,
        comment: "Test comment".to_string(),
    });

    let response = service.vote_on_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);

    // Test rating too high
    let request = Request::new(VoteOnTokenRequest {
        user_id: Uuid::new_v4().to_string(),
        token_id: Uuid::new_v4().to_string(),
        vote_type: VoteType::VoteTypeBullish as i32,
        rating: 6,
        comment: "Test comment".to_string(),
    });

    let response = service.vote_on_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_token_rankings_success() {
    let service = create_test_service().await;
    
    let request = Request::new(GetTokenRankingsRequest {
        page: 1,
        page_size: 10,
        ranking_type: "overall".to_string(),
        time_frame: "24h".to_string(),
        blockchain_filter: "".to_string(),
    });

    let response = service.get_token_rankings(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.tokens.is_empty());
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 10);
    assert!(response.total_count > 0);

    // Verify ranking structure
    for (i, ranked_token) in response.tokens.iter().enumerate() {
        assert_eq!(ranked_token.rank, i as i32 + 1);
        assert!(ranked_token.token.is_some());
        assert!(ranked_token.score > 0.0);
        assert!(!ranked_token.score_breakdown.is_empty());
    }
}

#[tokio::test]
async fn test_get_moonshot_analytics_success() {
    let service = create_test_service().await;
    
    let request = Request::new(GetMoonshotAnalyticsRequest {
        time_frame: "24h".to_string(),
        user_id: "".to_string(),
    });

    let response = service.get_moonshot_analytics(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.overview.is_some());
    assert!(!response.top_performers.is_empty());
    assert!(!response.worst_performers.is_empty());
    assert!(response.community_stats.is_some());
    assert!(response.trading_stats.is_some());

    // Verify overview data
    let overview = response.overview.unwrap();
    assert!(overview.total_tokens > 0);
    assert!(overview.active_proposals >= 0);
    assert!(overview.total_votes >= 0);
    assert!(!overview.total_volume.is_empty());
    assert!(overview.average_community_score > 0.0);
}

#[tokio::test]
async fn test_get_user_voting_history_success() {
    let service = create_test_service().await;
    
    let user_id = Uuid::new_v4().to_string();
    let request = Request::new(GetUserVotingHistoryRequest {
        user_id: user_id.clone(),
        page: 1,
        page_size: 10,
        time_frame: "30d".to_string(),
    });

    let response = service.get_user_voting_history(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.stats.is_some());
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 10);

    // Verify stats structure
    let stats = response.stats.unwrap();
    assert!(stats.total_votes >= 0);
    assert!(stats.average_rating >= 0.0);
    assert!(stats.voting_accuracy >= 0.0 && stats.voting_accuracy <= 1.0);
    assert!(stats.reputation_score >= 0.0 && stats.reputation_score <= 1.0);
    assert!(stats.successful_predictions >= 0);
}

#[tokio::test]
async fn test_get_token_details_success() {
    let service = create_test_service().await;
    
    let token_id = Uuid::new_v4().to_string();
    let request = Request::new(GetTokenDetailsRequest {
        token_id: token_id.clone(),
        include_price_history: true,
        include_vote_history: true,
        time_frame: "24h".to_string(),
    });

    let response = service.get_token_details(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.token.is_some());
    assert!(!response.price_history.is_empty());
    assert!(!response.recent_votes.is_empty());
    assert!(response.analysis.is_some());

    // Verify analysis structure
    let analysis = response.analysis.unwrap();
    assert!(analysis.technical_score >= 0.0 && analysis.technical_score <= 1.0);
    assert!(analysis.fundamental_score >= 0.0 && analysis.fundamental_score <= 1.0);
    assert!(analysis.sentiment_score >= 0.0 && analysis.sentiment_score <= 1.0);
    assert!(!analysis.risk_level.is_empty());
    assert!(!analysis.key_metrics.is_empty());
    assert!(!analysis.analysis_summary.is_empty());
}

#[tokio::test]
async fn test_get_token_sentiment_success() {
    let service = create_test_service().await;
    
    let token_id = Uuid::new_v4().to_string();
    let request = Request::new(GetTokenSentimentRequest {
        token_id: token_id.clone(),
        time_frame: "24h".to_string(),
        include_social_media: true,
    });

    let response = service.get_token_sentiment(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.sentiment.is_some());
    assert!(!response.sources.is_empty());

    // Verify sentiment structure
    let sentiment = response.sentiment.unwrap();
    assert!(sentiment.overall_sentiment >= -1.0 && sentiment.overall_sentiment <= 1.0);
    assert!(sentiment.bullish_percentage >= 0.0 && sentiment.bullish_percentage <= 100.0);
    assert!(sentiment.bearish_percentage >= 0.0 && sentiment.bearish_percentage <= 100.0);
    assert!(sentiment.neutral_percentage >= 0.0 && sentiment.neutral_percentage <= 100.0);
    assert!(sentiment.total_mentions >= 0);

    // Verify sources
    for source in response.sources {
        assert!(!source.source_type.is_empty());
        assert!(source.sentiment_score >= -1.0 && source.sentiment_score <= 1.0);
        assert!(source.mention_count >= 0);
        assert!(source.influence_weight >= 0.0 && source.influence_weight <= 1.0);
    }
}

#[tokio::test]
async fn test_get_token_predictions_success() {
    let service = create_test_service().await;
    
    let token_id = Uuid::new_v4().to_string();
    let request = Request::new(GetTokenPredictionsRequest {
        token_id: token_id.clone(),
        prediction_horizon: "24h".to_string(),
        include_technical_analysis: true,
    });

    let response = service.get_token_predictions(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.predictions.is_empty());
    assert!(response.technical_analysis.is_some());
    assert!(response.confidence_score >= 0.0 && response.confidence_score <= 1.0);

    // Verify predictions structure
    for prediction in response.predictions {
        assert!(!prediction.time_horizon.is_empty());
        assert!(!prediction.predicted_price.is_empty());
        assert!(!prediction.price_range_low.is_empty());
        assert!(!prediction.price_range_high.is_empty());
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
        assert!(!prediction.methodology.is_empty());
    }

    // Verify technical analysis
    let technical_analysis = response.technical_analysis.unwrap();
    assert!(!technical_analysis.trend_direction.is_empty());
    assert!(!technical_analysis.indicators.is_empty());
    assert!(!technical_analysis.levels.is_empty());
    assert!(!technical_analysis.recommendation.is_empty());
}
