//! Database repository implementations using SQLx
//! 
//! This module contains SQLx-based repository implementations that replace
//! the in-memory HashMap storage with persistent database operations.

pub mod kyc_repository;
pub mod wallet_repository;
pub mod card_repository;
pub mod fiat_repository;
pub mod production_wallet_repository;

pub use kyc_repository::SqlxKycRepository;
pub use wallet_repository::SqlxWalletRepository;
pub use card_repository::SqlxCardRepository;
pub use fiat_repository::SqlxFiatRepository;
pub use production_wallet_repository::{ProductionWalletRepository, ProductionWallet, WalletStatistics};
