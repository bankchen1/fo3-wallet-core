//! Tests for key derivation

use fo3_wallet::crypto::mnemonic::*;
use fo3_wallet::crypto::keys::*;

#[test]
fn test_ethereum_key_derivation() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let seed = mnemonic_to_seed(mnemonic, None).unwrap();
    
    let key_pair = derive_key_pair(&seed, KeyType::Ethereum, "m/44'/60'/0'/0/0").unwrap();
    
    assert_eq!(key_pair.key_type(), KeyType::Ethereum);
    
    let address = ethereum::public_key_to_address(key_pair.public_key()).unwrap();
    assert!(address.starts_with("0x"));
    assert_eq!(address.len(), 42);
}

#[test]
fn test_solana_key_derivation() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let seed = mnemonic_to_seed(mnemonic, None).unwrap();
    
    let key_pair = derive_key_pair(&seed, KeyType::Solana, "m/44'/501'/0'/0'").unwrap();
    
    assert_eq!(key_pair.key_type(), KeyType::Solana);
    
    let address = solana::public_key_to_address(key_pair.public_key()).unwrap();
    assert_eq!(address.len(), 44);
}

#[test]
fn test_bitcoin_key_derivation() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let seed = mnemonic_to_seed(mnemonic, None).unwrap();
    
    let key_pair = derive_key_pair(&seed, KeyType::Bitcoin, "m/44'/0'/0'/0/0").unwrap();
    
    assert_eq!(key_pair.key_type(), KeyType::Bitcoin);
    
    let address = bitcoin::public_key_to_address(key_pair.public_key(), bitcoin::Network::Bitcoin).unwrap();
    assert!(address.len() > 0);
}
