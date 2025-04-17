//! Basic usage example for fo3-wallet-solana
//!
//! This example demonstrates how to use the fo3-wallet-solana crate
//! to interact with the Solana blockchain.

use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};
use fo3_wallet_solana::{SolanaProvider, TokenTransferParams};
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

    // Example of how to create a token transfer transaction
    // This is just for demonstration and won't work without a funded account
    println!("\nExample of creating a token transfer transaction:");
    
    // USDC token mint on devnet (this is just an example, may not exist)
    let token_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    
    // Create token transfer parameters
    let params = TokenTransferParams {
        token_mint: token_mint.to_string(),
        from: public_key.to_string(),
        to: "83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri".to_string(), // Example recipient
        amount: 1000000, // 1 USDC (assuming 6 decimals)
        decimals: 6,
    };

    println!("Token transfer parameters:");
    println!("  Token mint: {}", params.token_mint);
    println!("  From: {}", params.from);
    println!("  To: {}", params.to);
    println!("  Amount: {} (raw units)", params.amount);
    println!("  Decimals: {}", params.decimals);

    println!("\nNote: This example doesn't actually send any transactions.");
    println!("To send real transactions, you would need to:");
    println!("1. Create the transaction using provider.create_token_transfer_transaction()");
    println!("2. Sign it with your keypair");
    println!("3. Broadcast it using provider.broadcast_transaction()");

    Ok(())
}
