//! Ethereum key derivation

use hmac::{Hmac, Mac};
use hmac::digest::KeyInit;
use sha2::Sha512;
use secp256k1::{Secp256k1, SecretKey, PublicKey as Secp256k1PublicKey};

use crate::error::{Error, Result};
use super::derivation::{KeyPair, PrivateKey, PublicKey, KeyType};

/// Derive an Ethereum key pair from a seed and derivation path
pub fn derive_ethereum_key_pair(seed: &[u8], path: &str) -> Result<KeyPair> {
    // Parse the derivation path
    let path_components = parse_derivation_path(path)?;

    // Derive the master key
    let (mut secret_key, mut chain_code) = derive_master_key(seed)?;

    // Derive the child keys
    for component in path_components {
        (secret_key, chain_code) = derive_child_key(secret_key, chain_code, component)?;
    }

    // Create the key pair
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&secret_key)
        .map_err(|e| Error::KeyDerivation(format!("Invalid secret key: {}", e)))?;
    let public_key = Secp256k1PublicKey::from_secret_key(&secp, &secret_key);

    let private_key = PrivateKey::new(secret_key.secret_bytes().to_vec(), KeyType::Ethereum);
    let public_key = PublicKey::new(public_key.serialize_uncompressed().to_vec(), KeyType::Ethereum);

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
    let mut hmac = Hmac::<Sha512>::new_from_slice(b"Bitcoin seed")
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
    let secp = Secp256k1::new();
    let parent_secret_key = SecretKey::from_slice(&parent_key)
        .map_err(|e| Error::KeyDerivation(format!("Invalid parent key: {}", e)))?;

    let mut data = Vec::with_capacity(37);

    if index >= 0x80000000 {
        // Hardened derivation
        data.push(0);
        data.extend_from_slice(&parent_key);
    } else {
        // Normal derivation
        let parent_public_key = Secp256k1PublicKey::from_secret_key(&secp, &parent_secret_key);
        data.extend_from_slice(&parent_public_key.serialize());
    }

    // Append the index
    data.extend_from_slice(&index.to_be_bytes());

    // Calculate HMAC-SHA512
    let mut hmac = Hmac::<Sha512>::new_from_slice(&parent_chain_code)
        .map_err(|_| Error::KeyDerivation("HMAC error".to_string()))?;

    hmac.update(&data);
    let result = hmac.finalize().into_bytes();

    let mut child_key = [0u8; 32];
    let mut child_chain_code = [0u8; 32];

    child_key.copy_from_slice(&result[0..32]);
    child_chain_code.copy_from_slice(&result[32..64]);

    // Add the parent key to the child key (mod n)
    let child_secret_key = SecretKey::from_slice(&child_key)
        .map_err(|e| Error::KeyDerivation(format!("Invalid child key: {}", e)))?;

    let child_secret_key = child_secret_key.add_tweak(&parent_secret_key.into())
        .map_err(|e| Error::KeyDerivation(format!("Key addition error: {}", e)))?;

    Ok((child_secret_key.secret_bytes(), child_chain_code))
}

/// Get the Ethereum address from a public key
pub fn public_key_to_address(public_key: &PublicKey) -> Result<String> {
    if public_key.key_type() != KeyType::Ethereum {
        return Err(Error::KeyDerivation("Not an Ethereum public key".to_string()));
    }

    let public_key = public_key.as_bytes();

    // The public key should be in uncompressed format (65 bytes)
    if public_key.len() != 65 {
        return Err(Error::KeyDerivation("Invalid Ethereum public key length".to_string()));
    }

    // Skip the first byte (0x04) and hash the rest
    let key_hash = keccak256(&public_key[1..]);

    // Take the last 20 bytes of the hash
    let address = &key_hash[12..];

    // Format as a hex string with 0x prefix
    Ok(format!("0x{}", hex::encode(address)))
}

/// Calculate the Keccak-256 hash of data
fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}
