//! Moonshot trading data models and repository traits
//! 
//! Provides data structures and repository interfaces for:
//! - Token entities with comprehensive metadata
//! - Vote entities with weighted voting system
//! - Proposal entities with lifecycle management
//! - Analytics and sentiment data
//! - Mock repository implementation for development

use std::collections::HashMap;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ServiceError;

/// Token entity representing a cryptocurrency token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEntity {
    pub token_id: String,
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub contract_address: String,
    pub blockchain: String,
    pub logo_url: Option<String>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub telegram_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: TokenStatus,
    pub metrics: TokenMetrics,
}

/// Token status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenStatus {
    Proposed,
    UnderReview,
    Approved,
    Rejected,
    Trending,
    Delisted,
}

/// Token metrics and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub current_price: String,
    pub market_cap: String,
    pub volume_24h: String,
    pub price_change_24h: String,
    pub price_change_percentage_24h: String,
    pub holders_count: i64,
    pub transactions_24h: i64,
    pub liquidity_score: f64,
    pub volatility_score: f64,
    pub community_score: f64,
    pub total_votes: i64,
    pub average_rating: f64,
}

/// Vote entity representing user votes on tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteEntity {
    pub vote_id: String,
    pub user_id: String,
    pub token_id: String,
    pub vote_type: VoteType,
    pub rating: i32,
    pub comment: Option<String>,
    pub weight: f64,
    pub created_at: DateTime<Utc>,
}

/// Vote type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    Bullish,
    Bearish,
    Neutral,
}

/// Token proposal entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalEntity {
    pub proposal_id: String,
    pub proposer_user_id: String,
    pub token_id: String,
    pub justification: String,
    pub status: ProposalStatus,
    pub votes_for: i64,
    pub votes_against: i64,
    pub created_at: DateTime<Utc>,
    pub voting_ends_at: DateTime<Utc>,
    pub supporting_documents: Vec<String>,
}

/// Proposal status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Submitted,
    Voting,
    Approved,
    Rejected,
    Expired,
}

/// Sentiment analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub token_id: String,
    pub overall_sentiment: f64,
    pub bullish_percentage: f64,
    pub bearish_percentage: f64,
    pub neutral_percentage: f64,
    pub total_mentions: i64,
    pub sentiment_trend: f64,
    pub sources: Vec<SentimentSource>,
    pub updated_at: DateTime<Utc>,
}

/// Sentiment source data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentSource {
    pub source_type: String,
    pub sentiment_score: f64,
    pub mention_count: i64,
    pub influence_weight: f64,
}

/// Price prediction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionData {
    pub token_id: String,
    pub time_horizon: String,
    pub predicted_price: String,
    pub price_range_low: String,
    pub price_range_high: String,
    pub confidence: f64,
    pub methodology: String,
    pub created_at: DateTime<Utc>,
}

/// Repository trait for moonshot trading operations
#[async_trait]
pub trait MoonshotRepository: Send + Sync {
    /// Get trending tokens with pagination and filters
    async fn get_trending_tokens(
        &self,
        page: i32,
        page_size: i32,
        time_frame: Option<String>,
        sort_by: Option<String>,
        blockchain_filter: Option<String>,
        min_market_cap: Option<f64>,
        max_market_cap: Option<f64>,
    ) -> Result<(Vec<TokenEntity>, i32), ServiceError>;

    /// Create a new token proposal
    async fn create_token_proposal(
        &self,
        proposal: ProposalEntity,
    ) -> Result<String, ServiceError>;

    /// Submit a vote for a token
    async fn submit_vote(
        &self,
        vote: VoteEntity,
    ) -> Result<String, ServiceError>;

    /// Get token rankings
    async fn get_token_rankings(
        &self,
        page: i32,
        page_size: i32,
        ranking_type: Option<String>,
        time_frame: Option<String>,
        blockchain_filter: Option<String>,
    ) -> Result<(Vec<(TokenEntity, f64)>, i32), ServiceError>;

    /// Get user voting history
    async fn get_user_voting_history(
        &self,
        user_id: &str,
        page: i32,
        page_size: i32,
        time_frame: Option<String>,
    ) -> Result<(Vec<VoteEntity>, i32), ServiceError>;

    /// Get token details by ID
    async fn get_token_details(
        &self,
        token_id: &str,
    ) -> Result<Option<TokenEntity>, ServiceError>;

    /// Get user proposals
    async fn get_user_proposals(
        &self,
        user_id: &str,
        page: i32,
        page_size: i32,
        status_filter: Option<ProposalStatus>,
    ) -> Result<(Vec<ProposalEntity>, i32), ServiceError>;

    /// Get token sentiment data
    async fn get_token_sentiment(
        &self,
        token_id: &str,
        time_frame: Option<String>,
    ) -> Result<Option<SentimentData>, ServiceError>;

    /// Get token predictions
    async fn get_token_predictions(
        &self,
        token_id: &str,
        prediction_horizon: Option<String>,
    ) -> Result<Vec<PredictionData>, ServiceError>;

    /// Update token metrics
    async fn update_token_metrics(
        &self,
        token_id: &str,
        metrics: TokenMetrics,
    ) -> Result<(), ServiceError>;
}

/// In-memory implementation of MoonshotRepository for development and testing
pub struct InMemoryMoonshotRepository {
    tokens: tokio::sync::RwLock<HashMap<String, TokenEntity>>,
    votes: tokio::sync::RwLock<HashMap<String, VoteEntity>>,
    proposals: tokio::sync::RwLock<HashMap<String, ProposalEntity>>,
    sentiment_data: tokio::sync::RwLock<HashMap<String, SentimentData>>,
    predictions: tokio::sync::RwLock<HashMap<String, Vec<PredictionData>>>,
}

impl InMemoryMoonshotRepository {
    /// Create new in-memory repository
    pub fn new() -> Self {
        Self {
            tokens: tokio::sync::RwLock::new(HashMap::new()),
            votes: tokio::sync::RwLock::new(HashMap::new()),
            proposals: tokio::sync::RwLock::new(HashMap::new()),
            sentiment_data: tokio::sync::RwLock::new(HashMap::new()),
            predictions: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Initialize with mock data
    pub async fn initialize_mock_data(&self) -> Result<(), ServiceError> {
        let mut tokens = self.tokens.write().await;
        
        // Add some mock tokens
        for i in 0..10 {
            let token_id = Uuid::new_v4().to_string();
            let token = TokenEntity {
                token_id: token_id.clone(),
                symbol: format!("TOKEN{}", i),
                name: format!("Test Token {}", i),
                description: format!("Mock token for testing purposes {}", i),
                contract_address: format!("0x{:040x}", i),
                blockchain: "ethereum".to_string(),
                logo_url: Some(format!("https://example.com/logo{}.png", i)),
                website_url: Some(format!("https://token{}.com", i)),
                twitter_url: Some(format!("https://twitter.com/token{}", i)),
                telegram_url: Some(format!("https://t.me/token{}", i)),
                created_at: Utc::now() - chrono::Duration::days(i as i64),
                updated_at: Utc::now(),
                status: TokenStatus::Trending,
                metrics: TokenMetrics {
                    current_price: format!("{:.6}", 0.001 + (i as f64 * 0.01)),
                    market_cap: format!("{:.2}", 1000000.0 + (i as f64 * 100000.0)),
                    volume_24h: format!("{:.2}", 100000.0 + (i as f64 * 10000.0)),
                    price_change_24h: format!("{:.6}", (i as f64 - 5.0) * 0.0001),
                    price_change_percentage_24h: format!("{:.2}", (i as f64 - 5.0) * 2.0),
                    holders_count: 1000 + (i as i64 * 100),
                    transactions_24h: 100 + (i as i64 * 10),
                    liquidity_score: 0.5 + (i as f64 * 0.05),
                    volatility_score: 0.3 + (i as f64 * 0.02),
                    community_score: 0.7 + (i as f64 * 0.03),
                    total_votes: 50 + (i as i64 * 5),
                    average_rating: 3.0 + (i as f64 * 0.2),
                },
            };
            tokens.insert(token_id, token);
        }

        Ok(())
    }
}

#[async_trait]
impl MoonshotRepository for InMemoryMoonshotRepository {
    async fn get_trending_tokens(
        &self,
        page: i32,
        page_size: i32,
        _time_frame: Option<String>,
        _sort_by: Option<String>,
        _blockchain_filter: Option<String>,
        _min_market_cap: Option<f64>,
        _max_market_cap: Option<f64>,
    ) -> Result<(Vec<TokenEntity>, i32), ServiceError> {
        let tokens = self.tokens.read().await;
        let mut token_list: Vec<TokenEntity> = tokens.values().cloned().collect();
        
        // Sort by creation date (newest first)
        token_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let total_count = token_list.len() as i32;
        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(token_list.len());
        
        let page_tokens = if start_idx < token_list.len() {
            token_list[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        Ok((page_tokens, total_count))
    }

    async fn create_token_proposal(
        &self,
        proposal: ProposalEntity,
    ) -> Result<String, ServiceError> {
        let mut proposals = self.proposals.write().await;
        let proposal_id = proposal.proposal_id.clone();
        proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }

    async fn submit_vote(
        &self,
        vote: VoteEntity,
    ) -> Result<String, ServiceError> {
        let mut votes = self.votes.write().await;
        let vote_id = vote.vote_id.clone();
        votes.insert(vote_id.clone(), vote);
        Ok(vote_id)
    }

    async fn get_token_rankings(
        &self,
        page: i32,
        page_size: i32,
        _ranking_type: Option<String>,
        _time_frame: Option<String>,
        _blockchain_filter: Option<String>,
    ) -> Result<(Vec<(TokenEntity, f64)>, i32), ServiceError> {
        let tokens = self.tokens.read().await;
        let mut ranked_tokens: Vec<(TokenEntity, f64)> = tokens
            .values()
            .map(|token| {
                let score = token.metrics.community_score * 0.4 +
                           token.metrics.liquidity_score * 0.3 +
                           (token.metrics.average_rating / 5.0) * 0.3;
                (token.clone(), score)
            })
            .collect();
        
        // Sort by score (highest first)
        ranked_tokens.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let total_count = ranked_tokens.len() as i32;
        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(ranked_tokens.len());
        
        let page_tokens = if start_idx < ranked_tokens.len() {
            ranked_tokens[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        Ok((page_tokens, total_count))
    }

    async fn get_user_voting_history(
        &self,
        user_id: &str,
        page: i32,
        page_size: i32,
        _time_frame: Option<String>,
    ) -> Result<(Vec<VoteEntity>, i32), ServiceError> {
        let votes = self.votes.read().await;
        let mut user_votes: Vec<VoteEntity> = votes
            .values()
            .filter(|vote| vote.user_id == user_id)
            .cloned()
            .collect();
        
        // Sort by creation date (newest first)
        user_votes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let total_count = user_votes.len() as i32;
        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(user_votes.len());
        
        let page_votes = if start_idx < user_votes.len() {
            user_votes[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        Ok((page_votes, total_count))
    }

    async fn get_token_details(
        &self,
        token_id: &str,
    ) -> Result<Option<TokenEntity>, ServiceError> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(token_id).cloned())
    }

    async fn get_user_proposals(
        &self,
        user_id: &str,
        page: i32,
        page_size: i32,
        _status_filter: Option<ProposalStatus>,
    ) -> Result<(Vec<ProposalEntity>, i32), ServiceError> {
        let proposals = self.proposals.read().await;
        let mut user_proposals: Vec<ProposalEntity> = proposals
            .values()
            .filter(|proposal| proposal.proposer_user_id == user_id)
            .cloned()
            .collect();
        
        // Sort by creation date (newest first)
        user_proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let total_count = user_proposals.len() as i32;
        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(user_proposals.len());
        
        let page_proposals = if start_idx < user_proposals.len() {
            user_proposals[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        Ok((page_proposals, total_count))
    }

    async fn get_token_sentiment(
        &self,
        token_id: &str,
        _time_frame: Option<String>,
    ) -> Result<Option<SentimentData>, ServiceError> {
        let sentiment_data = self.sentiment_data.read().await;
        Ok(sentiment_data.get(token_id).cloned())
    }

    async fn get_token_predictions(
        &self,
        token_id: &str,
        _prediction_horizon: Option<String>,
    ) -> Result<Vec<PredictionData>, ServiceError> {
        let predictions = self.predictions.read().await;
        Ok(predictions.get(token_id).cloned().unwrap_or_default())
    }

    async fn update_token_metrics(
        &self,
        token_id: &str,
        metrics: TokenMetrics,
    ) -> Result<(), ServiceError> {
        let mut tokens = self.tokens.write().await;
        if let Some(token) = tokens.get_mut(token_id) {
            token.metrics = metrics;
            token.updated_at = Utc::now();
        }
        Ok(())
    }
}

impl Default for InMemoryMoonshotRepository {
    fn default() -> Self {
        Self::new()
    }
}
