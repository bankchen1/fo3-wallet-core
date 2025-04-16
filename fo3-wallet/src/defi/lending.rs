//! Lending functionality

use crate::error::Result;
use crate::crypto::keys::KeyType;
use super::types::{Protocol, LendingRequest, LendingResult, LendingAction};
use super::provider::DeFiProviderFactory;
use crate::transaction::provider::ProviderConfig;

/// Execute lending action
pub fn execute_lending(request: &LendingRequest, config: &ProviderConfig) -> Result<LendingResult> {
    let key_type = match &request.action {
        LendingAction::Supply(token_amount) => token_amount.token.key_type,
        LendingAction::Withdraw(token_amount) => token_amount.token.key_type,
        LendingAction::Borrow(token_amount) => token_amount.token.key_type,
        LendingAction::Repay(token_amount) => token_amount.token.key_type,
    };
    
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    provider.execute_lending(request)
}

/// Get supported lending protocols
pub fn get_supported_lending_protocols(key_type: KeyType, config: &ProviderConfig) -> Result<Vec<Protocol>> {
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    let all_protocols = provider.get_supported_protocols();
    let lending_protocols = all_protocols.into_iter()
        .filter(|p| matches!(p, Protocol::Aave | Protocol::Compound))
        .collect();
    
    Ok(lending_protocols)
}
