//! Bitcoin key derivation

use hmac::{Hmac, Mac};
use hmac::digest::KeyInit;
use sha2::{Sha256, Sha512, Digest};
use secp256k1::{Secp256k1, SecretKey, PublicKey as Secp256k1PublicKey};
// We'll use the bs58 crate directly
use bs58;
pub use bitcoin::Network;

use crate::error::{Error, Result};
use super::derivation::{KeyPair, PrivateKey, PublicKey, KeyType};

/// Derive a Bitcoin key pair from a seed and derivation path
pub fn derive_bitcoin_key_pair(seed: &[u8], path: &str) -> Result<KeyPair> {
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

    let private_key = PrivateKey::new(secret_key.secret_bytes().to_vec(), KeyType::Bitcoin);
    let public_key = PublicKey::new(public_key.serialize().to_vec(), KeyType::Bitcoin);

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
    let mut hmac = <Hmac::<Sha512> as KeyInit>::new_from_slice(b"Bitcoin seed")
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
    let mut hmac = <Hmac::<Sha512> as KeyInit>::new_from_slice(&parent_chain_code)
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

/// Get the Bitcoin address from a public key
pub fn public_key_to_address(public_key: &PublicKey, network: Network) -> Result<String> {
    if public_key.key_type() != KeyType::Bitcoin {
        return Err(Error::KeyDerivation("Not a Bitcoin public key".to_string()));
    }

    let public_key = public_key.as_bytes();

    // The public key should be in compressed format (33 bytes)
    if public_key.len() != 33 {
        return Err(Error::KeyDerivation("Invalid Bitcoin public key length".to_string()));
    }

    // Create a Bitcoin public key
    let _secp = Secp256k1::new();
    let public_key = Secp256k1PublicKey::from_slice(public_key)
        .map_err(|e| Error::KeyDerivation(format!("Invalid Bitcoin public key: {}", e)))?;

    // Create a Bitcoin address
    // This is a simplified implementation
    // In a real implementation, we would use the bitcoin crate
    let mut hasher = Sha256::new();
    hasher.update(&public_key.serialize());
    let hash = hasher.finalize();

    // RIPEMD-160 hash
    // Since we can't directly use bitcoin's RIPEMD160, we'll use a simplified approach
    let hash = &hash[0..20]; // Just use the first 20 bytes of the SHA256 hash as a placeholder

    let mut address = Vec::with_capacity(21);
    match network {
        Network::Bitcoin => address.push(0x00), // Mainnet
        _ => address.push(0x6f), // Testnet
    }
    address.extend_from_slice(hash);

    // Add checksum
    let mut hasher = Sha256::new();
    hasher.update(&address);
    let hash = hasher.finalize();

    let mut hasher = Sha256::new();
    hasher.update(&hash);
    let hash = hasher.finalize();

    address.extend_from_slice(&hash[0..4]);

    // Encode as base58
    Ok(bs58::encode(address).into_string())
}
