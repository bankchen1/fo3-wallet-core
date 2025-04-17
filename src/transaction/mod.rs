//! Transaction functionality
//!
//! This module provides functionality for creating, signing, and broadcasting
//! transactions across multiple blockchains.

mod types;
mod ethereum;

#[cfg(not(feature = "solana"))]
mod solana;

#[cfg(feature = "solana")]
use fo3_wallet_solana as solana_impl;

mod bitcoin;
mod provider;

pub use types::*;
pub use ethereum::*;

#[cfg(not(feature = "solana"))]
pub use solana::*;

#[cfg(feature = "solana")]
pub use solana_impl::*;

pub use bitcoin::*;
pub use provider::*;
