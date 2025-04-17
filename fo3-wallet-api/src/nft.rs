//! NFT API endpoints
//!
//! This module provides NFT-specific API endpoints.

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::Serialize;
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
