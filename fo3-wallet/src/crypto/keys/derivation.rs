//! Common key derivation functionality

use crate::error::{Error, Result};

/// Supported key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyType {
    /// Ethereum and EVM compatible chains
    Ethereum,
    /// Solana
    Solana,
    /// Bitcoin
    Bitcoin,
}

/// A private key for a specific blockchain
#[derive(Debug, Clone)]
pub struct PrivateKey {
    /// The raw private key bytes
    bytes: Vec<u8>,
    /// The type of key
    key_type: KeyType,
}

impl PrivateKey {
    /// Create a new private key from bytes
    pub fn new(bytes: Vec<u8>, key_type: KeyType) -> Self {
        Self { bytes, key_type }
    }

    /// Get the raw private key bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }
}

/// A public key for a specific blockchain
#[derive(Debug, Clone)]
pub struct PublicKey {
    /// The raw public key bytes
    bytes: Vec<u8>,
    /// The type of key
    key_type: KeyType,
}

impl PublicKey {
    /// Create a new public key from bytes
    pub fn new(bytes: Vec<u8>, key_type: KeyType) -> Self {
        Self { bytes, key_type }
    }

    /// Get the raw public key bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }
}

/// A key pair for a specific blockchain
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// The private key
    private_key: PrivateKey,
    /// The public key
    public_key: PublicKey,
}

impl KeyPair {
    /// Create a new key pair
    pub fn new(private_key: PrivateKey, public_key: PublicKey) -> Result<Self> {
        if private_key.key_type() != public_key.key_type() {
            return Err(Error::KeyDerivation("Key type mismatch".to_string()));
        }
        Ok(Self { private_key, public_key })
    }

    /// Get the private key
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        self.private_key.key_type()
    }
}

/// Derive a key pair from a seed for a specific blockchain
pub fn derive_key_pair(seed: &[u8], key_type: KeyType, path: &str) -> Result<KeyPair> {
    match key_type {
        KeyType::Ethereum => crate::crypto::keys::ethereum::derive_ethereum_key_pair(seed, path),
        KeyType::Solana => crate::crypto::keys::solana::derive_solana_key_pair(seed, path),
        KeyType::Bitcoin => crate::crypto::keys::bitcoin::derive_bitcoin_key_pair(seed, path),
    }
}
