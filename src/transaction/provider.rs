//! Transaction provider

use crate::error::{Error, Result};
use crate::crypto::keys::KeyType;
use super::types::{TransactionRequest, TransactionManager, Transaction, TransactionReceipt, TransactionStatus};

/// Provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// HTTP provider
    Http,
    /// WebSocket provider
    WebSocket,
    /// IPC provider
    Ipc,
}

/// Provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider type
    pub provider_type: ProviderType,
    /// Provider URL
    pub url: String,
    /// API key (if required)
    pub api_key: Option<String>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

/// Provider factory
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a new provider
    pub fn create_provider(key_type: KeyType, config: ProviderConfig) -> Result<Box<dyn TransactionManager>> {
        match key_type {
            KeyType::Ethereum => {
                let provider = super::ethereum::EthereumProvider::new(config)?;
                Ok(Box::new(provider))
            }
            KeyType::Solana => {
                let provider = super::solana::SolanaProvider::new(config)?;
                Ok(Box::new(provider))
            }
            KeyType::Bitcoin => {
                let provider = super::bitcoin::BitcoinProvider::new(config)?;
                Ok(Box::new(provider))
            }
        }
    }
}
