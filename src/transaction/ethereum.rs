//! Ethereum transaction functionality

use std::str::FromStr;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

use ethers::prelude::{Address, TransactionRequest as EthersTransactionRequest, U256, NameOrAddress};
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};

use crate::error::{Error, Result};
use crate::crypto::keys::KeyType;
use super::types::{Transaction, TransactionRequest, TransactionReceipt, TransactionStatus, TransactionSigner, TransactionBroadcaster, TransactionManager, TransactionType};
use super::provider::{ProviderConfig, ProviderType};

/// Ethereum transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumTransaction {
    /// Nonce
    pub nonce: u64,
    /// Gas price
    pub gas_price: String,
    /// Gas limit
    pub gas_limit: String,
    /// To address
    pub to: String,
    /// Value
    pub value: String,
    /// Data
    pub data: Vec<u8>,
    /// Chain ID
    pub chain_id: u64,
}

/// Ethereum provider
pub struct EthereumProvider {
    /// Provider configuration
    #[allow(dead_code)]
    config: ProviderConfig,
    /// Chain ID
    chain_id: u64,
    /// Ethers provider
    provider: Arc<Provider<Http>>,
}

impl EthereumProvider {
    /// Create a new Ethereum provider
    pub fn new(config: ProviderConfig) -> Result<Self> {
        // Determine the chain ID from the network
        let chain_id = match config.url.as_str() {
            url if url.contains("mainnet") => 1, // Mainnet
            url if url.contains("goerli") => 5, // Goerli testnet
            url if url.contains("sepolia") => 11155111, // Sepolia testnet
            _ => 1, // Default to mainnet
        };
        
        // Create the ethers provider
        let provider = Provider::<Http>::try_from(config.url.clone())
            .map_err(|e| Error::Transaction(format!("Failed to create Ethereum provider: {}", e)))?;
        
        Ok(Self {
            config,
            chain_id,
            provider: Arc::new(provider),
        })
    }
    
    /// Get the chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
    
    /// Convert a private key to a wallet
    fn private_key_to_wallet(&self, private_key: &str) -> Result<LocalWallet> {
        let wallet = private_key.parse::<LocalWallet>()
            .map_err(|e| Error::Transaction(format!("Invalid private key: {}", e)))?
            .with_chain_id(self.chain_id);
        
        Ok(wallet)
    }
    
    /// Convert a transaction request to an ethers transaction request
    fn convert_transaction_request(&self, request: &TransactionRequest) -> Result<EthersTransactionRequest> {
        // Parse addresses
        let from = Address::from_str(&request.from)
            .map_err(|e| Error::Transaction(format!("Invalid from address: {}", e)))?;
        
        let to = Address::from_str(&request.to)
            .map_err(|e| Error::Transaction(format!("Invalid to address: {}", e)))?;
        
        // Parse value
        let value = U256::from_dec_str(&request.value)
            .map_err(|e| Error::Transaction(format!("Invalid value: {}", e)))?;
        
        // Create the transaction request
        let mut tx = EthersTransactionRequest::new()
            .from(from)
            .to(to)
            .value(value);
        
        // Add gas price if provided
        if let Some(gas_price) = &request.gas_price {
            let gas_price = U256::from_dec_str(gas_price)
                .map_err(|e| Error::Transaction(format!("Invalid gas price: {}", e)))?;
            tx = tx.gas_price(gas_price);
        }
        
        // Add gas limit if provided
        if let Some(gas_limit) = &request.gas_limit {
            let gas_limit = U256::from_dec_str(gas_limit)
                .map_err(|e| Error::Transaction(format!("Invalid gas limit: {}", e)))?;
            tx = tx.gas(gas_limit);
        }
        
        // Add nonce if provided
        if let Some(nonce) = request.nonce {
            tx = tx.nonce(nonce);
        }
        
        // Add data if provided
        if let Some(data) = &request.data {
            tx = tx.data(data.clone());
        }
        
        Ok(tx)
    }
}

impl TransactionSigner for EthereumProvider {
    fn sign_transaction(&self, request: &TransactionRequest) -> Result<Vec<u8>> {
        // Check that the request is for Ethereum
        if request.key_type != KeyType::Ethereum {
            return Err(Error::Transaction("Not an Ethereum transaction".to_string()));
        }
        
        // In a real implementation, we would use the private key from the request
        // For now, we'll just create a dummy signed transaction
        let signed_transaction = vec![0u8; 32];
        
        Ok(signed_transaction)
    }
}

impl TransactionBroadcaster for EthereumProvider {
    fn broadcast_transaction(&self, signed_transaction: &[u8]) -> Result<String> {
        // In a real implementation, we would use the ethers provider to broadcast the transaction
        // For now, we'll just create a dummy transaction hash
        let hash = format!("0x{}", hex::encode(&signed_transaction[0..32]));
        
        Ok(hash)
    }
    
    fn get_transaction_status(&self, _hash: &str) -> Result<TransactionStatus> {
        // In a real implementation, we would use the ethers provider to get the transaction status
        // For now, we'll just return a dummy status
        Ok(TransactionStatus::Confirmed)
    }
    
    fn get_transaction_receipt(&self, hash: &str) -> Result<TransactionReceipt> {
        // In a real implementation, we would use the ethers provider to get the transaction receipt
        // For now, we'll just create a dummy receipt
        let receipt = TransactionReceipt {
            hash: hash.to_string(),
            status: TransactionStatus::Confirmed,
            block_number: Some(12345678),
            timestamp: Some(1620000000),
            fee: Some("0.001".to_string()),
            logs: vec![],
        };
        
        Ok(receipt)
    }
}

impl TransactionManager for EthereumProvider {
    fn get_transaction(&self, hash: &str) -> Result<Transaction> {
        // In a real implementation, we would use the ethers provider to get the transaction
        // For now, we'll just create a dummy transaction
        let transaction = Transaction {
            hash: hash.to_string(),
            transaction_type: TransactionType::Transfer,
            key_type: KeyType::Ethereum,
            from: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            to: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            value: "1000000000000000000".to_string(), // 1 ETH
            gas_price: Some("20000000000".to_string()), // 20 Gwei
            gas_limit: Some("21000".to_string()),
            nonce: Some(0),
            data: None,
            status: TransactionStatus::Confirmed,
            block_number: Some(12345678),
            timestamp: Some(1620000000),
            fee: Some("0.001".to_string()),
        };
        
        Ok(transaction)
    }
    
    fn get_transactions(&self, address: &str, _limit: usize, _offset: usize) -> Result<Vec<Transaction>> {
        // In a real implementation, we would use the ethers provider to get the transactions
        // For now, we'll just create a dummy transaction
        let transaction = Transaction {
            hash: format!("0x{}", hex::encode(&[0u8; 32])),
            transaction_type: TransactionType::Transfer,
            key_type: KeyType::Ethereum,
            from: address.to_string(),
            to: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            value: "1000000000000000000".to_string(), // 1 ETH
            gas_price: Some("20000000000".to_string()), // 20 Gwei
            gas_limit: Some("21000".to_string()),
            nonce: Some(0),
            data: None,
            status: TransactionStatus::Confirmed,
            block_number: Some(12345678),
            timestamp: Some(1620000000),
            fee: Some("0.001".to_string()),
        };
        
        Ok(vec![transaction])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_id() {
        let config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
            api_key: None,
            timeout: Some(30),
        };
        
        let provider = EthereumProvider::new(config).unwrap();
        assert_eq!(provider.chain_id(), 1);
    }
    
    #[test]
    fn test_convert_transaction_request() {
        let config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
            api_key: None,
            timeout: Some(30),
        };
        
        let provider = EthereumProvider::new(config).unwrap();
        
        let request = TransactionRequest {
            key_type: KeyType::Ethereum,
            from: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            to: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            value: "1000000000000000000".to_string(), // 1 ETH
            gas_price: Some("20000000000".to_string()), // 20 Gwei
            gas_limit: Some("21000".to_string()),
            nonce: Some(0),
            data: None,
        };
        
        let tx = provider.convert_transaction_request(&request).unwrap();
        
        assert_eq!(tx.from.unwrap(), Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e").unwrap());
        
        // Use NameOrAddress::Address to wrap the address
        let to_address = Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e").unwrap();
        assert_eq!(tx.to.unwrap(), NameOrAddress::Address(to_address));
        
        assert_eq!(tx.value.unwrap(), U256::from_dec_str("1000000000000000000").unwrap());
        assert_eq!(tx.gas_price.unwrap(), U256::from_dec_str("20000000000").unwrap());
        assert_eq!(tx.gas.unwrap(), U256::from_dec_str("21000").unwrap());
        assert_eq!(tx.nonce.unwrap(), 0.into());
    }
}
