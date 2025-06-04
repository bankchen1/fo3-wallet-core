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
pub mod card_funding;
pub mod card_funding_methods;
pub mod card_funding_crypto;
pub mod card_funding_admin;
pub mod ledger;
pub mod ledger_methods;
pub mod ledger_journal;
pub mod ledger_reporting;
pub mod ledger_admin;
pub mod rewards;
pub mod rewards_methods;
pub mod referral;
pub mod referral_methods;
pub mod wallet_connect;
pub mod automated_trading;
pub mod dapp_browser;
pub mod dapp_signing;
pub mod earn;
pub mod moonshot;
pub mod market_intelligence;

// Phase 3: Service Integration & Real-time Features
pub mod integration;

#[cfg(test)]
pub mod rewards_test;
#[cfg(test)]
pub mod referral_test;
#[cfg(test)]
pub mod card_funding_test;
#[cfg(test)]
pub mod ledger_test;

#[cfg(feature = "solana")]
pub mod solana;
