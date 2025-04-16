//! Tests for transaction functionality

use fo3_wallet::crypto::keys::KeyType;
use fo3_wallet::transaction::{
    TransactionRequest, TransactionStatus,
    provider::{ProviderConfig, ProviderType, ProviderFactory},
};

#[test]
fn test_ethereum_transaction() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Create a provider
    let provider = ProviderFactory::create_provider(KeyType::Ethereum, config).unwrap();
    
    // Create a transaction request
    let request = TransactionRequest {
        key_type: KeyType::Ethereum,
        from: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
        to: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
        value: "1000000000000000000".to_string(), // 1 ETH
        gas_price: Some("20000000000".to_string()), // 20 Gwei
        gas_limit: Some("21000".to_string()),
        nonce: Some(0),
        data: None,
    };
    
    // Send the transaction
    let hash = provider.send_transaction(&request).unwrap();
    
    // Check that the hash is not empty
    assert!(!hash.is_empty());
    
    // Get the transaction status
    let status = provider.get_transaction_status(&hash).unwrap();
    
    // Check that the status is confirmed
    assert_eq!(status, TransactionStatus::Confirmed);
    
    // Get the transaction
    let transaction = provider.get_transaction(&hash).unwrap();
    
    // Check that the transaction has the correct values
    assert_eq!(transaction.hash, hash);
    assert_eq!(transaction.key_type, KeyType::Ethereum);
    assert_eq!(transaction.from, "0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
    assert_eq!(transaction.to, "0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
    assert_eq!(transaction.value, "1000000000000000000");
    assert_eq!(transaction.gas_price, Some("20000000000".to_string()));
    assert_eq!(transaction.gas_limit, Some("21000".to_string()));
    assert_eq!(transaction.nonce, Some(0));
    assert_eq!(transaction.status, TransactionStatus::Confirmed);
}

#[test]
fn test_solana_transaction() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://api.mainnet-beta.solana.com".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Create a provider
    let provider = ProviderFactory::create_provider(KeyType::Solana, config).unwrap();
    
    // Create a transaction request
    let request = TransactionRequest {
        key_type: KeyType::Solana,
        from: "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg".to_string(),
        to: "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg".to_string(),
        value: "1000000000".to_string(), // 1 SOL
        gas_price: None,
        gas_limit: None,
        nonce: None,
        data: None,
    };
    
    // Send the transaction
    let hash = provider.send_transaction(&request).unwrap();
    
    // Check that the hash is not empty
    assert!(!hash.is_empty());
    
    // Get the transaction status
    let status = provider.get_transaction_status(&hash).unwrap();
    
    // Check that the status is confirmed
    assert_eq!(status, TransactionStatus::Confirmed);
    
    // Get the transaction
    let transaction = provider.get_transaction(&hash).unwrap();
    
    // Check that the transaction has the correct values
    assert_eq!(transaction.hash, hash);
    assert_eq!(transaction.key_type, KeyType::Solana);
    assert_eq!(transaction.from, "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg");
    assert_eq!(transaction.to, "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg");
    assert_eq!(transaction.value, "1000000000");
    assert_eq!(transaction.status, TransactionStatus::Confirmed);
}

#[test]
fn test_bitcoin_transaction() {
    // Create a provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Http,
        url: "https://btc.getblock.io/mainnet".to_string(),
        api_key: None,
        timeout: Some(30),
    };
    
    // Create a provider
    let provider = ProviderFactory::create_provider(KeyType::Bitcoin, config).unwrap();
    
    // Create a transaction request
    let request = TransactionRequest {
        key_type: KeyType::Bitcoin,
        from: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
        to: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
        value: "100000000".to_string(), // 1 BTC
        gas_price: None,
        gas_limit: None,
        nonce: None,
        data: None,
    };
    
    // Send the transaction
    let hash = provider.send_transaction(&request).unwrap();
    
    // Check that the hash is not empty
    assert!(!hash.is_empty());
    
    // Get the transaction status
    let status = provider.get_transaction_status(&hash).unwrap();
    
    // Check that the status is confirmed
    assert_eq!(status, TransactionStatus::Confirmed);
    
    // Get the transaction
    let transaction = provider.get_transaction(&hash).unwrap();
    
    // Check that the transaction has the correct values
    assert_eq!(transaction.hash, hash);
    assert_eq!(transaction.key_type, KeyType::Bitcoin);
    assert_eq!(transaction.from, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
    assert_eq!(transaction.to, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
    assert_eq!(transaction.value, "100000000");
    assert_eq!(transaction.status, TransactionStatus::Confirmed);
}
