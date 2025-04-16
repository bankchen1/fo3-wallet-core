//! FO3 Wallet Core - Multi-chain wallet and DeFi SDK
//!
//! This library provides core functionality for managing crypto wallets across
//! multiple blockchains (EVM, Solana, Bitcoin), including mnemonic generation,
//! key derivation, transaction signing, and DeFi interactions.

pub mod error;
pub mod crypto;
pub mod account;
pub mod transaction;
pub mod defi;

// Re-export commonly used types for convenience
pub use error::{Error, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
