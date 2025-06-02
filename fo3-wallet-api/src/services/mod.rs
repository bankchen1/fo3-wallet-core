//! gRPC service implementations

pub mod wallet;
pub mod transaction;
pub mod defi;
pub mod health;
pub mod auth;
pub mod events;
pub mod kyc;
pub mod fiat_gateway;
pub mod payment_providers;
pub mod pricing;
pub mod notifications;
pub mod cards;
pub mod spending_insights;

#[cfg(feature = "solana")]
pub mod solana;
