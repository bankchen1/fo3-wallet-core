//! Raydium API endpoints
//!
//! This module provides Raydium-specific API endpoints.

use axum::{
    extract::{Extension, Json},

};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use fo3_wallet_solana::{SolanaProvider, RaydiumSwapQuote};

use crate::{ApiError, AppState, Result};

/// Swap preview request
#[derive(Debug, Deserialize)]
pub struct SwapPreviewRequest {
    /// Input token mint address
    pub token_in_mint: String,
    /// Output token mint address
    pub token_out_mint: String,
    /// Input amount (in token's smallest unit)
    pub amount_in: String,
    /// Slippage tolerance in percentage (e.g., 0.5 for 0.5%)
    pub slippage: f64,
}

/// Swap preview response
#[derive(Debug, Serialize)]
pub struct SwapPreviewResponse {
    /// Input token symbol
    pub in_token_symbol: String,
    /// Output token symbol
    pub out_token_symbol: String,
    /// Input amount (in token's smallest unit)
    pub in_amount: String,
    /// Estimated output amount (in token's smallest unit)
    pub out_amount: String,
    /// Price impact percentage
    pub price_impact: f64,
    /// Minimum output amount with slippage (in token's smallest unit)
    pub min_out_amount: String,
    /// Fee amount (in SOL)
    pub fee: f64,
}

/// Swap execute request
#[derive(Debug, Deserialize)]
pub struct SwapExecuteRequest {
    /// Input token mint address
    pub token_in_mint: String,
    /// Output token mint address
    pub token_out_mint: String,
    /// Input amount (in token's smallest unit)
    pub amount_in: String,
    /// Minimum output amount (in token's smallest unit)
    pub min_amount_out: String,
    /// Wallet address
    pub wallet_address: String,
    /// Private key for signing (in a real app, this would be handled more securely)
    pub private_key: String,
}

/// Swap execute response
#[derive(Debug, Serialize)]
pub struct SwapExecuteResponse {
    /// Transaction signature
    pub signature: String,
}

/// Get supported token pairs
pub async fn get_token_pairs(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<(String, String)>>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let pairs = provider.get_raydium_token_pairs()
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(pairs))
}

/// Get swap preview
pub async fn get_swap_preview(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SwapPreviewRequest>,
) -> Result<Json<SwapPreviewResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Parse amount
    let amount_in = request.amount_in.parse::<u64>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid amount: {}", e)))?;

    // Get quote
    let quote: RaydiumSwapQuote = provider.get_raydium_swap_quote(
        &request.token_in_mint,
        &request.token_out_mint,
        amount_in,
        request.slippage,
    ).map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Convert to response
    let response = SwapPreviewResponse {
        in_token_symbol: quote.in_token_symbol,
        out_token_symbol: quote.out_token_symbol,
        in_amount: quote.in_amount.to_string(),
        out_amount: quote.out_amount.to_string(),
        price_impact: quote.price_impact,
        min_out_amount: quote.min_out_amount.to_string(),
        fee: quote.fee,
    };

    Ok(Json(response))
}

/// Execute swap
pub async fn execute_swap(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SwapExecuteRequest>,
) -> Result<Json<SwapExecuteResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Parse amounts
    let amount_in = request.amount_in.parse::<u64>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid input amount: {}", e)))?;

    let min_amount_out = request.min_amount_out.parse::<u64>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid minimum output amount: {}", e)))?;

    // Execute swap
    let signature = provider.execute_raydium_swap(
        &request.token_in_mint,
        &request.token_out_mint,
        amount_in,
        min_amount_out,
        &request.wallet_address,
        &request.private_key,
    ).map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(SwapExecuteResponse {
        signature,
    }))
}
