//! Swap functionality

use crate::error::Result;
use crate::crypto::keys::KeyType;
use super::types::{Protocol, Token, TokenAmount, SwapRequest, SwapResult};
use super::provider::DeFiProviderFactory;
use crate::transaction::provider::ProviderConfig;

/// Swap tokens
pub fn swap_tokens(request: &SwapRequest, config: &ProviderConfig) -> Result<SwapResult> {
    let key_type = request.from.token.key_type;
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    provider.execute_swap(request)
}

/// Get swap quote
pub fn get_swap_quote(request: &SwapRequest, config: &ProviderConfig) -> Result<TokenAmount> {
    let key_type = request.from.token.key_type;
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    provider.get_swap_quote(request)
}

/// Get supported tokens
pub fn get_supported_tokens(key_type: KeyType, config: &ProviderConfig) -> Result<Vec<Token>> {
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    provider.get_supported_tokens()
}

/// Get supported protocols
pub fn get_supported_protocols(key_type: KeyType, config: &ProviderConfig) -> Result<Vec<Protocol>> {
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    Ok(provider.get_supported_protocols())
}
