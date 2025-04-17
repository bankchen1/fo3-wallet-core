//! DeFi functionality
//!
//! This module provides functionality for interacting with DeFi protocols
//! across multiple blockchains.

mod types;
mod swap;
mod lending;
mod staking;
mod provider;
mod ethereum;
mod aave;
mod lido;

pub use types::*;
pub use swap::*;
pub use lending::*;
pub use staking::*;
pub use provider::*;

// We'll use mock implementations for now
