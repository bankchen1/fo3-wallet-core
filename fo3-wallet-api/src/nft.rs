//! NFT API endpoints
//!
//! This module provides NFT-specific API endpoints.

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use fo3_wallet_solana::{SolanaProvider, NftToken, NftMetadata};

use crate::{ApiError, AppState, Result};

/// Get NFTs by owner
pub async fn get_nfts_by_owner(
    Extension(state): Extension<Arc<AppState>>,
    Path(wallet_address): Path<String>,
) -> Result<Json<Vec<NftToken>>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let nfts = provider.get_nfts_by_owner(&wallet_address).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(nfts))
}

/// Get NFT metadata
pub async fn get_nft_metadata(
    Extension(state): Extension<Arc<AppState>>,
    Path(mint): Path<String>,
) -> Result<Json<NftMetadata>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let metadata = provider.get_nft_metadata(&mint).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(metadata))
}

/// NFT transfer request
#[derive(Debug, Deserialize)]
pub struct NftTransferRequest {
    /// From wallet address
    pub from: String,
    /// To wallet address
    pub to: String,
    /// NFT mint address
    pub mint: String,
    /// Private key for signing (in a real app, this would be handled more securely)
    pub private_key: String,
}

/// NFT transfer response
#[derive(Debug, Serialize)]
pub struct NftTransferResponse {
    /// Transaction signature
    pub signature: String,
}

/// Transfer an NFT
pub async fn transfer_nft(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<NftTransferRequest>,
) -> Result<Json<NftTransferResponse>> {
    let provider = SolanaProvider::new(state.get_solana_config())
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let signature = provider.transfer_nft(
        &request.from,
        &request.to,
        &request.mint,
        &request.private_key,
    ).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(NftTransferResponse {
        signature,
    }))
}
