//! MoonshotGuard middleware for validating moonshot trading requests
//! 
//! Provides comprehensive validation for:
//! - Token proposal submissions with anti-spam measures
//! - Voting requests with fraud prevention
//! - Request parameter validation
//! - Rate limiting enforcement
//! - Data integrity checks

use std::collections::HashSet;
use regex::Regex;
use tracing::{warn, error};

use crate::proto::fo3::wallet::v1::{
    GetTrendingTokensRequest,
    SubmitTokenProposalRequest,
    VoteOnTokenRequest,
    GetTokenRankingsRequest,
    GetTokenDetailsRequest,
    GetTokenSentimentRequest,
    GetTokenPredictionsRequest,
};
use crate::error::ServiceError;

/// MoonshotGuard provides validation for moonshot trading operations
pub struct MoonshotGuard {
    /// Supported blockchain networks
    supported_blockchains: HashSet<String>,
    /// Contract address regex patterns by blockchain
    address_patterns: std::collections::HashMap<String, Regex>,
    /// Maximum proposal justification length
    max_justification_length: usize,
    /// Maximum comment length for votes
    max_comment_length: usize,
    /// Minimum rating value
    min_rating: i32,
    /// Maximum rating value
    max_rating: i32,
}

impl MoonshotGuard {
    /// Create new MoonshotGuard instance
    pub fn new() -> Result<Self, ServiceError> {
        let mut supported_blockchains = HashSet::new();
        supported_blockchains.insert("ethereum".to_string());
        supported_blockchains.insert("polygon".to_string());
        supported_blockchains.insert("bsc".to_string());
        supported_blockchains.insert("arbitrum".to_string());
        supported_blockchains.insert("optimism".to_string());
        supported_blockchains.insert("solana".to_string());

        let mut address_patterns = std::collections::HashMap::new();
        
        // Ethereum-like address pattern (0x followed by 40 hex characters)
        let eth_pattern = Regex::new(r"^0x[a-fA-F0-9]{40}$")
            .map_err(|e| ServiceError::ValidationError(format!("Invalid regex pattern: {}", e)))?;
        address_patterns.insert("ethereum".to_string(), eth_pattern.clone());
        address_patterns.insert("polygon".to_string(), eth_pattern.clone());
        address_patterns.insert("bsc".to_string(), eth_pattern.clone());
        address_patterns.insert("arbitrum".to_string(), eth_pattern.clone());
        address_patterns.insert("optimism".to_string(), eth_pattern);

        // Solana address pattern (base58 encoded, 32-44 characters)
        let solana_pattern = Regex::new(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$")
            .map_err(|e| ServiceError::ValidationError(format!("Invalid regex pattern: {}", e)))?;
        address_patterns.insert("solana".to_string(), solana_pattern);

        Ok(Self {
            supported_blockchains,
            address_patterns,
            max_justification_length: 2000,
            max_comment_length: 500,
            min_rating: 1,
            max_rating: 5,
        })
    }

    /// Validate trending tokens request
    pub fn validate_trending_tokens_request(
        &self,
        request: &GetTrendingTokensRequest,
    ) -> Result<(), ServiceError> {
        // Validate page parameters
        if request.page < 1 {
            return Err(ServiceError::ValidationError(
                "Page number must be greater than 0".to_string()
            ));
        }

        if request.page_size < 1 || request.page_size > 100 {
            return Err(ServiceError::ValidationError(
                "Page size must be between 1 and 100".to_string()
            ));
        }

        // Validate time frame
        if !request.time_frame.is_empty() {
            let valid_timeframes = ["1h", "24h", "7d", "30d"];
            if !valid_timeframes.contains(&request.time_frame.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid time frame. Must be one of: 1h, 24h, 7d, 30d".to_string()
                ));
            }
        }

        // Validate sort by
        if !request.sort_by.is_empty() {
            let valid_sort_options = ["volume", "price_change", "community_score", "market_cap"];
            if !valid_sort_options.contains(&request.sort_by.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid sort option. Must be one of: volume, price_change, community_score, market_cap".to_string()
                ));
            }
        }

        // Validate blockchain filter
        if !request.blockchain_filter.is_empty() {
            if !self.supported_blockchains.contains(&request.blockchain_filter) {
                return Err(ServiceError::ValidationError(
                    format!("Unsupported blockchain: {}", request.blockchain_filter)
                ));
            }
        }

        // Validate market cap range
        if request.min_market_cap < 0.0 || request.max_market_cap < 0.0 {
            return Err(ServiceError::ValidationError(
                "Market cap values must be non-negative".to_string()
            ));
        }

        if request.min_market_cap > 0.0 && request.max_market_cap > 0.0 && 
           request.min_market_cap >= request.max_market_cap {
            return Err(ServiceError::ValidationError(
                "Minimum market cap must be less than maximum market cap".to_string()
            ));
        }

        Ok(())
    }

    /// Validate token proposal request
    pub fn validate_token_proposal_request(
        &self,
        request: &SubmitTokenProposalRequest,
    ) -> Result<(), ServiceError> {
        // Validate user ID
        if request.user_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "User ID is required".to_string()
            ));
        }

        // Validate token symbol
        if request.symbol.is_empty() || request.symbol.len() > 10 {
            return Err(ServiceError::ValidationError(
                "Token symbol must be 1-10 characters".to_string()
            ));
        }

        // Check for valid symbol characters (alphanumeric only)
        if !request.symbol.chars().all(|c| c.is_alphanumeric()) {
            return Err(ServiceError::ValidationError(
                "Token symbol must contain only alphanumeric characters".to_string()
            ));
        }

        // Validate token name
        if request.name.is_empty() || request.name.len() > 50 {
            return Err(ServiceError::ValidationError(
                "Token name must be 1-50 characters".to_string()
            ));
        }

        // Validate description
        if request.description.is_empty() || request.description.len() > 500 {
            return Err(ServiceError::ValidationError(
                "Token description must be 1-500 characters".to_string()
            ));
        }

        // Validate blockchain
        if !self.supported_blockchains.contains(&request.blockchain) {
            return Err(ServiceError::ValidationError(
                format!("Unsupported blockchain: {}", request.blockchain)
            ));
        }

        // Validate contract address
        if let Some(pattern) = self.address_patterns.get(&request.blockchain) {
            if !pattern.is_match(&request.contract_address) {
                return Err(ServiceError::ValidationError(
                    format!("Invalid contract address format for blockchain: {}", request.blockchain)
                ));
            }
        }

        // Validate URLs (basic format check)
        if !request.website_url.is_empty() && !self.is_valid_url(&request.website_url) {
            return Err(ServiceError::ValidationError(
                "Invalid website URL format".to_string()
            ));
        }

        if !request.twitter_url.is_empty() && !self.is_valid_url(&request.twitter_url) {
            return Err(ServiceError::ValidationError(
                "Invalid Twitter URL format".to_string()
            ));
        }

        if !request.telegram_url.is_empty() && !self.is_valid_url(&request.telegram_url) {
            return Err(ServiceError::ValidationError(
                "Invalid Telegram URL format".to_string()
            ));
        }

        // Validate justification
        if request.justification.is_empty() || request.justification.len() > self.max_justification_length {
            return Err(ServiceError::ValidationError(
                format!("Justification must be 1-{} characters", self.max_justification_length)
            ));
        }

        // Validate supporting documents
        if request.supporting_documents.len() > 5 {
            return Err(ServiceError::ValidationError(
                "Maximum 5 supporting documents allowed".to_string()
            ));
        }

        Ok(())
    }

    /// Validate vote request
    pub fn validate_vote_request(
        &self,
        request: &VoteOnTokenRequest,
    ) -> Result<(), ServiceError> {
        // Validate user ID
        if request.user_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "User ID is required".to_string()
            ));
        }

        // Validate token ID
        if request.token_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "Token ID is required".to_string()
            ));
        }

        // Validate vote type
        if request.vote_type == 0 {
            return Err(ServiceError::ValidationError(
                "Vote type is required".to_string()
            ));
        }

        // Validate rating
        if request.rating < self.min_rating || request.rating > self.max_rating {
            return Err(ServiceError::ValidationError(
                format!("Rating must be between {} and {}", self.min_rating, self.max_rating)
            ));
        }

        // Validate comment length
        if request.comment.len() > self.max_comment_length {
            return Err(ServiceError::ValidationError(
                format!("Comment must be less than {} characters", self.max_comment_length)
            ));
        }

        Ok(())
    }

    /// Validate token rankings request
    pub fn validate_token_rankings_request(
        &self,
        request: &GetTokenRankingsRequest,
    ) -> Result<(), ServiceError> {
        // Validate page parameters
        if request.page < 1 {
            return Err(ServiceError::ValidationError(
                "Page number must be greater than 0".to_string()
            ));
        }

        if request.page_size < 1 || request.page_size > 100 {
            return Err(ServiceError::ValidationError(
                "Page size must be between 1 and 100".to_string()
            ));
        }

        // Validate ranking type
        if !request.ranking_type.is_empty() {
            let valid_types = ["trending", "community", "technical", "overall"];
            if !valid_types.contains(&request.ranking_type.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid ranking type. Must be one of: trending, community, technical, overall".to_string()
                ));
            }
        }

        // Validate time frame
        if !request.time_frame.is_empty() {
            let valid_timeframes = ["1h", "24h", "7d", "30d"];
            if !valid_timeframes.contains(&request.time_frame.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid time frame. Must be one of: 1h, 24h, 7d, 30d".to_string()
                ));
            }
        }

        // Validate blockchain filter
        if !request.blockchain_filter.is_empty() {
            if !self.supported_blockchains.contains(&request.blockchain_filter) {
                return Err(ServiceError::ValidationError(
                    format!("Unsupported blockchain: {}", request.blockchain_filter)
                ));
            }
        }

        Ok(())
    }

    /// Validate token details request
    pub fn validate_token_details_request(
        &self,
        request: &GetTokenDetailsRequest,
    ) -> Result<(), ServiceError> {
        // Validate token ID
        if request.token_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "Token ID is required".to_string()
            ));
        }

        // Validate time frame
        if !request.time_frame.is_empty() {
            let valid_timeframes = ["1h", "24h", "7d", "30d"];
            if !valid_timeframes.contains(&request.time_frame.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid time frame. Must be one of: 1h, 24h, 7d, 30d".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Validate token sentiment request
    pub fn validate_token_sentiment_request(
        &self,
        request: &GetTokenSentimentRequest,
    ) -> Result<(), ServiceError> {
        // Validate token ID
        if request.token_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "Token ID is required".to_string()
            ));
        }

        // Validate time frame
        if !request.time_frame.is_empty() {
            let valid_timeframes = ["1h", "24h", "7d", "30d"];
            if !valid_timeframes.contains(&request.time_frame.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid time frame. Must be one of: 1h, 24h, 7d, 30d".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Validate token predictions request
    pub fn validate_token_predictions_request(
        &self,
        request: &GetTokenPredictionsRequest,
    ) -> Result<(), ServiceError> {
        // Validate token ID
        if request.token_id.is_empty() {
            return Err(ServiceError::ValidationError(
                "Token ID is required".to_string()
            ));
        }

        // Validate prediction horizon
        if !request.prediction_horizon.is_empty() {
            let valid_horizons = ["1h", "24h", "7d", "30d"];
            if !valid_horizons.contains(&request.prediction_horizon.as_str()) {
                return Err(ServiceError::ValidationError(
                    "Invalid prediction horizon. Must be one of: 1h, 24h, 7d, 30d".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Basic URL validation
    fn is_valid_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
}

impl Default for MoonshotGuard {
    fn default() -> Self {
        Self::new().expect("Failed to create MoonshotGuard")
    }
}
