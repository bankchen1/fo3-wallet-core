//! Error types for the wallet-core library

use thiserror::Error;

/// Custom error type for wallet-core operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("Mnemonic error: {0}")]
    Mnemonic(String),

    #[error("Key derivation error: {0}")]
    KeyDerivation(String),

    #[error("Signing error: {0}")]
    Signing(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Chain error: {0}")]
    Chain(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("DeFi error: {0}")]
    DeFi(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type for wallet-core operations
pub type Result<T> = std::result::Result<T, Error>;
