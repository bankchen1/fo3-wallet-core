//! Common transaction types

use serde::{Serialize, Deserialize};
use crate::crypto::keys::KeyType;
use crate::error::{Error, Result};

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    /// Transaction is confirmed
    Confirmed,
    /// Transaction failed
    Failed,
}

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Transfer of native tokens
    Transfer,
    /// Contract call
    ContractCall,
    /// Token transfer
    TokenTransfer,
    /// Swap
    Swap,
    /// Liquidity provision
    LiquidityProvision,
    /// Staking
    Staking,
    /// Other
    Other,
}

/// Transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction hash
    pub hash: String,
    /// Transaction type
    pub transaction_type: TransactionType,
    /// Blockchain type
    pub key_type: KeyType,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Value in the smallest unit (e.g., wei, lamports, satoshis)
    pub value: String,
    /// Gas price (for EVM chains)
    pub gas_price: Option<String>,
    /// Gas limit (for EVM chains)
    pub gas_limit: Option<String>,
    /// Nonce (for EVM chains)
    pub nonce: Option<u64>,
    /// Data (for contract calls)
    pub data: Option<Vec<u8>>,
    /// Status
    pub status: TransactionStatus,
    /// Block number
    pub block_number: Option<u64>,
    /// Timestamp
    pub timestamp: Option<u64>,
    /// Fee paid
    pub fee: Option<String>,
}

/// Transaction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Blockchain type
    pub key_type: KeyType,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Value in the smallest unit (e.g., wei, lamports, satoshis)
    pub value: String,
    /// Gas price (for EVM chains)
    pub gas_price: Option<String>,
    /// Gas limit (for EVM chains)
    pub gas_limit: Option<String>,
    /// Nonce (for EVM chains)
    pub nonce: Option<u64>,
    /// Data (for contract calls)
    pub data: Option<Vec<u8>>,
}

/// Transaction receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub hash: String,
    /// Status
    pub status: TransactionStatus,
    /// Block number
    pub block_number: Option<u64>,
    /// Timestamp
    pub timestamp: Option<u64>,
    /// Fee paid
    pub fee: Option<String>,
    /// Logs
    pub logs: Vec<String>,
}

/// Transaction signer
pub trait TransactionSigner {
    /// Sign a transaction
    fn sign_transaction(&self, request: &TransactionRequest) -> Result<Vec<u8>>;
}

/// Transaction broadcaster
pub trait TransactionBroadcaster {
    /// Broadcast a signed transaction
    fn broadcast_transaction(&self, signed_transaction: &[u8]) -> Result<String>;
    
    /// Get transaction status
    fn get_transaction_status(&self, hash: &str) -> Result<TransactionStatus>;
    
    /// Get transaction receipt
    fn get_transaction_receipt(&self, hash: &str) -> Result<TransactionReceipt>;
}

/// Transaction manager
pub trait TransactionManager: TransactionSigner + TransactionBroadcaster {
    /// Create and sign a transaction
    fn create_and_sign_transaction(&self, request: &TransactionRequest) -> Result<Vec<u8>> {
        self.sign_transaction(request)
    }
    
    /// Create, sign, and broadcast a transaction
    fn send_transaction(&self, request: &TransactionRequest) -> Result<String> {
        let signed_transaction = self.create_and_sign_transaction(request)?;
        self.broadcast_transaction(&signed_transaction)
    }
    
    /// Get transaction by hash
    fn get_transaction(&self, hash: &str) -> Result<Transaction>;
    
    /// Get transactions for an address
    fn get_transactions(&self, address: &str, limit: usize, offset: usize) -> Result<Vec<Transaction>>;
}
