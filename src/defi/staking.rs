//! Staking functionality

use crate::error::Result;
use crate::crypto::keys::KeyType;
use super::types::{Protocol, Token, TokenAmount, StakingRequest, StakingResult, StakingAction};
use super::provider::{DeFiProviderFactory, EthereumDeFiProvider, SolanaDeFiProvider};
use crate::transaction::provider::ProviderConfig;

/// Execute staking action
pub fn execute_staking(request: &StakingRequest, config: &ProviderConfig) -> Result<StakingResult> {
    let key_type = match &request.action {
        StakingAction::Stake(token_amount) => token_amount.token.key_type,
        StakingAction::Unstake(token_amount) => token_amount.token.key_type,
        StakingAction::ClaimRewards => {
            // For claim rewards, we need to determine the key type from the protocol
            match request.protocol {
                Protocol::Lido => KeyType::Ethereum,
                Protocol::Marinade => KeyType::Solana,
                _ => return Err(crate::error::Error::DeFi(format!("Unsupported protocol for claim rewards: {:?}", request.protocol))),
            }
        }
    };
    
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    provider.execute_staking(request)
}

/// Get supported staking protocols
pub fn get_supported_staking_protocols(key_type: KeyType, config: &ProviderConfig) -> Result<Vec<Protocol>> {
    let provider = DeFiProviderFactory::create_provider(key_type, config.clone())?;
    
    let all_protocols = provider.get_supported_protocols();
    let staking_protocols = all_protocols.into_iter()
        .filter(|p| matches!(p, Protocol::Lido | Protocol::Marinade))
        .collect();
    
    Ok(staking_protocols)
}
