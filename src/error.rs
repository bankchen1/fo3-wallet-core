//! Error types

use thiserror::Error;

/// Custom error type
#[derive(Error, Debug)]
pub enum Error {
    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),

    /// Mnemonic error
    #[error("Mnemonic error: {0}")]
    Mnemonic(String),

    /// Key derivation error
    #[error("Key derivation error: {0}")]
    KeyDerivation(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Provider error
    #[error("Provider error: {0}")]
    Provider(String),

    /// DeFi error
    #[error("DeFi error: {0}")]
    DeFi(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
