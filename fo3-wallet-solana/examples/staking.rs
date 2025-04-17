//! Staking example for fo3-wallet-solana
//!
//! This example demonstrates how to use the fo3-wallet-solana crate
//! to stake SOL to a validator on the Solana blockchain.

use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};
use fo3_wallet_solana::{SolanaProvider, StakingParams};
use solana_sdk::signature::{Keypair, Signer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://api.devnet.solana.com".to_string(), // Use devnet for testing
        api_key: None,
        timeout: Some(30),
    };

    // Create Solana provider
    let provider = SolanaProvider::new(config)?;
    println!("Solana provider initialized");

    // Create a test keypair
    let keypair = Keypair::new();
    let public_key = keypair.pubkey();
    let private_key = bs58::encode(keypair.to_bytes()).into_string();

    println!("Generated keypair:");
    println!("  Public key: {}", public_key);
    println!("  Private key: {}", private_key);

    // Check SOL balance
    // Note: This would be 0 for a new keypair unless you fund it
    println!("\nTo check your SOL balance and use this example:");
    println!("1. Fund your address using Solana devnet faucet:");
    println!("   https://faucet.solana.com/?defaultAddress={}", public_key);
    println!("2. Wait for the transaction to confirm");
    println!("3. Run this example again with your funded address");

    // Example of how to create a staking transaction
    // This is just for demonstration and won't work without a funded account
    println!("\nExample of creating a staking transaction:");
    
    // A validator vote account on devnet (this is just an example, may not exist)
    let validator = "5p8qKVyKthA9DUb1rwQDzjcmTkaZdwN97J3LiaEhDs4b";
    
    // Create staking parameters
    let params = StakingParams {
        from: public_key.to_string(),
        validator: validator.to_string(),
        amount: 1_000_000_000, // 1 SOL
    };

    println!("Staking parameters:");
    println!("  From: {}", params.from);
    println!("  Validator: {}", params.validator);
    println!("  Amount: {} lamports ({} SOL)", params.amount, params.amount as f64 / 1_000_000_000.0);

    // Create a new stake account keypair
    let stake_account = Keypair::new();
    let stake_account_pubkey = stake_account.pubkey();
    println!("  Stake account: {}", stake_account_pubkey);

    println!("\nNote: This example doesn't actually send any transactions.");
    println!("To stake SOL in a real application, you would need to:");
    println!("1. Create the transaction using provider.create_stake_transaction()");
    println!("2. Sign it with your keypair and the stake account keypair");
    println!("3. Broadcast it using provider.broadcast_transaction()");
    println!("4. Check the status of your stake using provider.get_stake_info()");

    Ok(())
}
