//! Wallet implementation

use serde::{Serialize, Deserialize};
use crate::error::{Error, Result};
use crate::crypto::mnemonic::{generate_mnemonic, validate_mnemonic, mnemonic_to_seed, MnemonicStrength};
use crate::crypto::keys::{KeyType, PrivateKey, PublicKey, KeyPair, derive_key_pair};

/// A wallet that can manage accounts across multiple blockchains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// The wallet's unique identifier
    id: String,
    /// The wallet's name
    name: String,
    /// The encrypted mnemonic phrase (in a real implementation, this would be encrypted)
    #[serde(skip_serializing)]
    encrypted_mnemonic: Option<String>,
    /// Whether the wallet is backed up
    is_backed_up: bool,
    /// The timestamp when the wallet was created
    created_at: u64,
}

impl Wallet {
    /// Create a new wallet with a generated mnemonic
    pub fn new(name: String) -> Result<(Self, String)> {
        let mnemonic = generate_mnemonic(MnemonicStrength::Words12)?;
        let id = format!("wallet_{}", hex::encode(&rand::random::<[u8; 8]>()));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(e.to_string()))?
            .as_secs();

        let wallet = Self {
            id,
            name,
            encrypted_mnemonic: Some(mnemonic.clone()), // In a real implementation, this would be encrypted
            is_backed_up: false,
            created_at: now,
        };

        Ok((wallet, mnemonic))
    }

    /// Create a wallet from an existing mnemonic
    pub fn from_mnemonic(name: String, mnemonic: &str) -> Result<Self> {
        if !validate_mnemonic(mnemonic)? {
            return Err(Error::Mnemonic("Invalid mnemonic phrase".to_string()));
        }

        let id = format!("wallet_{}", hex::encode(&rand::random::<[u8; 8]>()));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(e.to_string()))?
            .as_secs();

        let wallet = Self {
            id,
            name,
            encrypted_mnemonic: Some(mnemonic.to_string()), // In a real implementation, this would be encrypted
            is_backed_up: true, // Assuming the user has backed up the mnemonic since they're importing it
            created_at: now,
        };

        Ok(wallet)
    }

    /// Get the wallet's ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the wallet's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the wallet's name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Check if the wallet is backed up
    pub fn is_backed_up(&self) -> bool {
        self.is_backed_up
    }

    /// Mark the wallet as backed up
    pub fn mark_as_backed_up(&mut self) {
        self.is_backed_up = true;
    }

    /// Get the wallet's creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Get the wallet's seed
    pub fn seed(&self, passphrase: Option<&str>) -> Result<Vec<u8>> {
        let mnemonic = self.encrypted_mnemonic.as_ref()
            .ok_or_else(|| Error::Mnemonic("Mnemonic not available".to_string()))?;

        mnemonic_to_seed(mnemonic, passphrase)
    }

    /// Derive a key pair for a specific blockchain
    pub fn derive_key_pair(&self, key_type: KeyType, path: &str, passphrase: Option<&str>) -> Result<KeyPair> {
        let seed = self.seed(passphrase)?;
        derive_key_pair(&seed, key_type, path)
    }

    /// Get an Ethereum address for this wallet
    pub fn get_ethereum_address(&self, path: &str, passphrase: Option<&str>) -> Result<String> {
        let key_pair = self.derive_key_pair(KeyType::Ethereum, path, passphrase)?;
        crate::crypto::keys::ethereum::public_key_to_address(key_pair.public_key())
    }

    /// Get a Solana address for this wallet
    pub fn get_solana_address(&self, path: &str, passphrase: Option<&str>) -> Result<String> {
        let key_pair = self.derive_key_pair(KeyType::Solana, path, passphrase)?;
        crate::crypto::keys::solana::public_key_to_address(key_pair.public_key())
    }

    /// Get a Bitcoin address for this wallet
    pub fn get_bitcoin_address(&self, path: &str, network: crate::crypto::keys::bitcoin::Network, passphrase: Option<&str>) -> Result<String> {
        let key_pair = self.derive_key_pair(KeyType::Bitcoin, path, passphrase)?;
        crate::crypto::keys::bitcoin::public_key_to_address(key_pair.public_key(), network)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let (wallet, mnemonic) = Wallet::new("Test Wallet".to_string()).unwrap();

        assert_eq!(wallet.name(), "Test Wallet");
        assert!(!wallet.is_backed_up());
        assert!(validate_mnemonic(&mnemonic).unwrap());
    }

    #[test]
    fn test_wallet_from_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = Wallet::from_mnemonic("Imported Wallet".to_string(), mnemonic).unwrap();

        assert_eq!(wallet.name(), "Imported Wallet");
        assert!(wallet.is_backed_up());
    }

    #[test]
    fn test_wallet_name_update() {
        let (mut wallet, _) = Wallet::new("Test Wallet".to_string()).unwrap();

        assert_eq!(wallet.name(), "Test Wallet");

        wallet.set_name("Updated Name".to_string());
        assert_eq!(wallet.name(), "Updated Name");
    }
}
