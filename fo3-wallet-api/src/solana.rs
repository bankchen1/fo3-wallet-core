//! Solana API endpoints
//!
//! This module provides Solana-specific API endpoints.

use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use fo3_wallet::transaction::provider::ProviderConfig;
use fo3_wallet_solana::{
    SolanaProvider, TokenInfo, TokenTransferParams, StakingParams, StakingInfo,
};

use crate::{ApiError, AppState, Result};

/// Token balance request
#[derive(Debug, Deserialize)]
pub struct TokenBalanceRequest {
    /// Address to check balance for
    pub address: String,
    /// Token mint address
    pub token_mint: String,
}

/// Token balance response
#[derive(Debug, Serialize)]
pub struct TokenBalanceResponse {
    /// Address
    pub address: String,
    /// Token mint address
    pub token_mint: String,
    /// Balance
    pub balance: String,
    /// Decimals
    pub decimals: u8,
}

/// Get token balance
pub async fn get_token_balance(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<TokenBalanceRequest>,
) -> Result<Json<TokenBalanceResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Get token info
    let token_info = provider.get_token_info(&request.token_mint)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Get token balance
    let balance = provider.get_token_balance(&request.address, &request.token_mint)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(TokenBalanceResponse {
        address: request.address,
        token_mint: request.token_mint,
        balance: balance.to_string(),
        decimals: token_info.decimals,
    }))
}

/// Get token info
pub async fn get_token_info(
    Extension(state): Extension<Arc<AppState>>,
    Path(token_mint): Path<String>,
) -> Result<Json<TokenInfo>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let token_info = provider.get_token_info(&token_mint)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(token_info))
}

/// Token transfer request
#[derive(Debug, Deserialize)]
pub struct TokenTransferRequest {
    /// Token mint address
    pub token_mint: String,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Amount of tokens to transfer
    pub amount: String,
    /// Private key for signing (in a real app, this would be handled more securely)
    pub private_key: String,
}

/// Token transfer response
#[derive(Debug, Serialize)]
pub struct TokenTransferResponse {
    /// Transaction signature
    pub signature: String,
}

/// Transfer tokens
pub async fn transfer_tokens(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<TokenTransferRequest>,
) -> Result<Json<TokenTransferResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Get token info for decimals
    let token_info = provider.get_token_info(&request.token_mint)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Parse amount
    let amount = request.amount.parse::<f64>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid amount: {}", e)))?;

    // Convert to raw amount based on decimals
    let raw_amount = (amount * 10f64.powi(token_info.decimals as i32)) as u64;

    // Create token transfer parameters
    let params = TokenTransferParams {
        token_mint: request.token_mint,
        from: request.from.clone(),
        to: request.to,
        amount: raw_amount,
        decimals: token_info.decimals,
    };

    // Convert private key to keypair
    let keypair = provider.private_key_to_keypair(&request.private_key)
        .map_err(|e| ApiError::BadRequest(format!("Invalid private key: {}", e)))?;

    // Create token transfer transaction
    let transaction = provider.create_token_transfer_transaction(&params, &keypair.pubkey())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Sign transaction
    let signed_transaction = transaction.sign(&[&keypair], transaction.message.recent_blockhash);

    // Serialize transaction
    let serialized = bincode::serialize(&signed_transaction)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize transaction: {}", e)))?;

    // Broadcast transaction
    let signature = provider.broadcast_transaction(&serialized)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(TokenTransferResponse {
        signature,
    }))
}

/// Staking request
#[derive(Debug, Deserialize)]
pub struct StakingRequest {
    /// From address (the staker)
    pub from: String,
    /// Validator vote account address
    pub validator: String,
    /// Amount to stake in SOL
    pub amount: String,
    /// Private key for signing (in a real app, this would be handled more securely)
    pub private_key: String,
}

/// Staking response
#[derive(Debug, Serialize)]
pub struct StakingResponse {
    /// Transaction signature
    pub signature: String,
    /// Stake account address
    pub stake_account: String,
}

/// Stake SOL
pub async fn stake_sol(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<StakingRequest>,
) -> Result<Json<StakingResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Parse amount
    let amount = request.amount.parse::<f64>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid amount: {}", e)))?;

    // Convert to lamports (1 SOL = 1,000,000,000 lamports)
    let lamports = (amount * 1_000_000_000f64) as u64;

    // Create staking parameters
    let params = StakingParams {
        from: request.from.clone(),
        validator: request.validator,
        amount: lamports,
    };

    // Convert private key to keypair
    let keypair = provider.private_key_to_keypair(&request.private_key)
        .map_err(|e| ApiError::BadRequest(format!("Invalid private key: {}", e)))?;

    // Create stake account keypair
    let stake_account = solana_sdk::signature::Keypair::new();
    let stake_account_pubkey = stake_account.pubkey();

    // Create staking transaction
    let transaction = provider.create_stake_transaction(&params, &keypair.pubkey())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Sign transaction
    let signed_transaction = transaction.sign(&[&keypair, &stake_account], transaction.message.recent_blockhash);

    // Serialize transaction
    let serialized = bincode::serialize(&signed_transaction)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize transaction: {}", e)))?;

    // Broadcast transaction
    let signature = provider.broadcast_transaction(&serialized)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(StakingResponse {
        signature,
        stake_account: stake_account_pubkey.to_string(),
    }))
}

/// Get staking info
pub async fn get_staking_info(
    Extension(state): Extension<Arc<AppState>>,
    Path(stake_account): Path<String>,
) -> Result<Json<StakingInfo>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let staking_info = provider.get_stake_info(&stake_account)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(staking_info))
}
