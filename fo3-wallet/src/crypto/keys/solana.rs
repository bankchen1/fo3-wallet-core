//! Solana key derivation

use ed25519_dalek::{SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use hmac::digest::KeyInit;
use sha2::Sha512;

use crate::error::{Error, Result};
use super::derivation::{KeyPair, PrivateKey, PublicKey, KeyType};

/// Derive a Solana key pair from a seed and derivation path
pub fn derive_solana_key_pair(seed: &[u8], path: &str) -> Result<KeyPair> {
    // Parse the derivation path
    let path_components = parse_derivation_path(path)?;
    
    // Derive the master key
    let (mut secret_key, mut chain_code) = derive_master_key(seed)?;
    
    // Derive the child keys
    for component in path_components {
        (secret_key, chain_code) = derive_child_key(secret_key, chain_code, component)?;
    }
    
    // Create the key pair
    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    let private_key = PrivateKey::new(signing_key.to_bytes().to_vec(), KeyType::Solana);
    let public_key = PublicKey::new(verifying_key.to_bytes().to_vec(), KeyType::Solana);
    
    KeyPair::new(private_key, public_key)
}

/// Parse a BIP-32 derivation path
fn parse_derivation_path(path: &str) -> Result<Vec<u32>> {
    if !path.starts_with("m/") {
        return Err(Error::KeyDerivation(format!("Invalid derivation path: {}", path)));
    }
    
    let components = path.trim_start_matches("m/").split('/');
    let mut result = Vec::new();
    
    for component in components {
        if component.is_empty() {
            continue;
        }
        
        let hardened = component.ends_with('\'');
        let index = if hardened {
            let index = component.trim_end_matches('\'').parse::<u32>()
                .map_err(|_| Error::KeyDerivation(format!("Invalid derivation path component: {}", component)))?;
            0x80000000 + index
        } else {
            component.parse::<u32>()
                .map_err(|_| Error::KeyDerivation(format!("Invalid derivation path component: {}", component)))?
        };
        
        result.push(index);
    }
    
    Ok(result)
}

/// Derive the master key from a seed
fn derive_master_key(seed: &[u8]) -> Result<([u8; 32], [u8; 32])> {
    let mut hmac = <Hmac::<Sha512> as KeyInit>::new_from_slice(b"ed25519 seed")
        .map_err(|_| Error::KeyDerivation("HMAC error".to_string()))?;
    
    hmac.update(seed);
    let result = hmac.finalize().into_bytes();
    
    let mut secret_key = [0u8; 32];
    let mut chain_code = [0u8; 32];
    
    secret_key.copy_from_slice(&result[0..32]);
    chain_code.copy_from_slice(&result[32..64]);
    
    Ok((secret_key, chain_code))
}

/// Derive a child key from a parent key
fn derive_child_key(parent_key: [u8; 32], parent_chain_code: [u8; 32], index: u32) -> Result<([u8; 32], [u8; 32])> {
    let mut data = Vec::with_capacity(37);
    
    if index >= 0x80000000 {
        // Hardened derivation
        data.push(0);
        data.extend_from_slice(&parent_key);
    } else {
        // Normal derivation
        let signing_key = SigningKey::from_bytes(&parent_key);
        let verifying_key = VerifyingKey::from(&signing_key);
        data.extend_from_slice(&verifying_key.to_bytes());
    }
    
    // Append the index
    data.extend_from_slice(&index.to_be_bytes());
    
    // Calculate HMAC-SHA512
    let mut hmac = <Hmac::<Sha512> as KeyInit>::new_from_slice(&parent_chain_code)
        .map_err(|_| Error::KeyDerivation("HMAC error".to_string()))?;
    
    hmac.update(&data);
    let result = hmac.finalize().into_bytes();
    
    let mut child_key = [0u8; 32];
    let mut child_chain_code = [0u8; 32];
    
    child_key.copy_from_slice(&result[0..32]);
    child_chain_code.copy_from_slice(&result[32..64]);
    
    Ok((child_key, child_chain_code))
}

/// Get the Solana address from a public key
pub fn public_key_to_address(public_key: &PublicKey) -> Result<String> {
    if public_key.key_type() != KeyType::Solana {
        return Err(Error::KeyDerivation("Not a Solana public key".to_string()));
    }
    
    let public_key = public_key.as_bytes();
    
    // The public key should be 32 bytes
    if public_key.len() != 32 {
        return Err(Error::KeyDerivation("Invalid Solana public key length".to_string()));
    }
    
    // Encode the public key as base58
    Ok(bs58::encode(public_key).into_string())
}
