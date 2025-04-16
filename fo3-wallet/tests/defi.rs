//! Tests for DeFi functionality

use fo3_wallet::crypto::keys::KeyType;
use fo3_wallet::transaction::provider::{ProviderConfig, ProviderType};
use fo3_wallet::defi::{
    Protocol, TokenAmount,
    SwapRequest, LendingRequest, StakingRequest,
    LendingAction, StakingAction,
    swap_tokens, get_swap_quote, get_supported_tokens,
    execute_lending, get_supported_lending_protocols,
    execute_staking, get_supported_staking_protocols,
};

#[test]
fn test_ethereum_swap() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Get supported tokens
    let tokens = get_supported_tokens(KeyType::Ethereum, &config).unwrap();
    
    // Find ETH and USDC tokens
    let eth_token = tokens.iter().find(|t| t.symbol == "ETH").unwrap().clone();
    let usdc_token = tokens.iter().find(|t| t.symbol == "USDC").unwrap().clone();
    
    // Create a swap request
    let request = SwapRequest {
        from: TokenAmount {
            token: eth_token.clone(),
            amount: "1000000000000000000".to_string(), // 1 ETH
        },
        to: usdc_token.clone(),
        slippage: 0.5,
        protocol: Protocol::Uniswap,
        deadline: Some(1800), // 30 minutes
    };
    
    // Get a swap quote
    let quote = get_swap_quote(&request, &config).unwrap();
    
    // Check that the quote is not empty
    assert!(!quote.amount.is_empty());
    
    // Execute the swap
    let result = swap_tokens(&request, &config).unwrap();
    
    // Check that the result has the correct tokens
    assert_eq!(result.from.token.symbol, "ETH");
    assert_eq!(result.to.token.symbol, "USDC");
    
    // Check that the transaction hash is not empty
    assert!(!result.transaction_hash.is_empty());
}

#[test]
fn test_ethereum_lending() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Get supported tokens
    let tokens = get_supported_tokens(KeyType::Ethereum, &config).unwrap();
    
    // Find ETH token
    let eth_token = tokens.iter().find(|t| t.symbol == "ETH").unwrap().clone();
    
    // Create a lending request
    let request = LendingRequest {
        action: LendingAction::Supply(TokenAmount {
            token: eth_token.clone(),
            amount: "1000000000000000000".to_string(), // 1 ETH
        }),
        protocol: Protocol::Aave,
    };
    
    // Execute the lending action
    let result = execute_lending(&request, &config).unwrap();
    
    // Check that the transaction hash is not empty
    assert!(!result.transaction_hash.is_empty());
    
    // Get supported lending protocols
    let protocols = get_supported_lending_protocols(KeyType::Ethereum, &config).unwrap();
    
    // Check that Aave and Compound are supported
    assert!(protocols.contains(&Protocol::Aave));
    assert!(protocols.contains(&Protocol::Compound));
}

#[test]
fn test_ethereum_staking() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Get supported tokens
    let tokens = get_supported_tokens(KeyType::Ethereum, &config).unwrap();
    
    // Find ETH token
    let eth_token = tokens.iter().find(|t| t.symbol == "ETH").unwrap().clone();
    
    // Create a staking request
    let request = StakingRequest {
        action: StakingAction::Stake(TokenAmount {
            token: eth_token.clone(),
            amount: "1000000000000000000".to_string(), // 1 ETH
        }),
        protocol: Protocol::Lido,
    };
    
    // Execute the staking action
    let result = execute_staking(&request, &config).unwrap();
    
    // Check that the transaction hash is not empty
    assert!(!result.transaction_hash.is_empty());
    
    // Get supported staking protocols
    let protocols = get_supported_staking_protocols(KeyType::Ethereum, &config).unwrap();
    
    // Check that Lido is supported
    assert!(protocols.contains(&Protocol::Lido));
}

#[test]
fn test_solana_swap() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://api.mainnet-beta.solana.com".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Get supported tokens
    let tokens = get_supported_tokens(KeyType::Solana, &config).unwrap();
    
    // Find SOL and USDC tokens
    let sol_token = tokens.iter().find(|t| t.symbol == "SOL").unwrap().clone();
    let usdc_token = tokens.iter().find(|t| t.symbol == "USDC").unwrap().clone();
    
    // Create a swap request
    let request = SwapRequest {
        from: TokenAmount {
            token: sol_token.clone(),
            amount: "1000000000".to_string(), // 1 SOL
        },
        to: usdc_token.clone(),
        slippage: 0.5,
        protocol: Protocol::Raydium,
        deadline: Some(1800), // 30 minutes
    };
    
    // Get a swap quote
    let quote = get_swap_quote(&request, &config).unwrap();
    
    // Check that the quote is not empty
    assert!(!quote.amount.is_empty());
    
    // Execute the swap
    let result = swap_tokens(&request, &config).unwrap();
    
    // Check that the result has the correct tokens
    assert_eq!(result.from.token.symbol, "SOL");
    assert_eq!(result.to.token.symbol, "USDC");
    
    // Check that the transaction hash is not empty
    assert!(!result.transaction_hash.is_empty());
}

#[test]
fn test_solana_staking() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://api.mainnet-beta.solana.com".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Get supported tokens
    let tokens = get_supported_tokens(KeyType::Solana, &config).unwrap();
    
    // Find SOL token
    let sol_token = tokens.iter().find(|t| t.symbol == "SOL").unwrap().clone();
    
    // Create a staking request
    let request = StakingRequest {
        action: StakingAction::Stake(TokenAmount {
            token: sol_token.clone(),
            amount: "1000000000".to_string(), // 1 SOL
        }),
        protocol: Protocol::Marinade,
    };
    
    // Execute the staking action
    let result = execute_staking(&request, &config).unwrap();
    
    // Check that the transaction hash is not empty
    assert!(!result.transaction_hash.is_empty());
    
    // Get supported staking protocols
    let protocols = get_supported_staking_protocols(KeyType::Solana, &config).unwrap();
    
    // Check that Marinade is supported
    assert!(protocols.contains(&Protocol::Marinade));
}
