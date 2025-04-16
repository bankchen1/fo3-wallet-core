//! Address management

use serde::{Serialize, Deserialize};
use crate::error::{Error, Result};
use crate::crypto::keys::{KeyType, PublicKey};

/// A blockchain address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Address {
    /// The address string in the blockchain's native format
    pub address: String,
    /// The type of blockchain this address is for
    pub key_type: KeyType,
    /// The derivation path used to generate this address
    pub path: String,
}

impl Address {
    /// Create a new address
    pub fn new(address: String, key_type: KeyType, path: String) -> Self {
        Self {
            address,
            key_type,
            path,
        }
    }

    /// Get the address string
    pub fn as_str(&self) -> &str {
        &self.address
    }

    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    /// Get the derivation path
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Derive an Ethereum address from a public key
pub fn derive_ethereum_address(public_key: &PublicKey, path: &str) -> Result<Address> {
    if public_key.key_type() != KeyType::Ethereum {
        return Err(Error::KeyDerivation("Not an Ethereum public key".to_string()));
    }
    
    let address = crate::crypto::keys::ethereum::public_key_to_address(public_key)?;
    
    Ok(Address::new(address, KeyType::Ethereum, path.to_string()))
}

/// Derive a Solana address from a public key
pub fn derive_solana_address(public_key: &PublicKey, path: &str) -> Result<Address> {
    if public_key.key_type() != KeyType::Solana {
        return Err(Error::KeyDerivation("Not a Solana public key".to_string()));
    }
    
    let address = crate::crypto::keys::solana::public_key_to_address(public_key)?;
    
    Ok(Address::new(address, KeyType::Solana, path.to_string()))
}

/// Derive a Bitcoin address from a public key
pub fn derive_bitcoin_address(public_key: &PublicKey, path: &str, network: bitcoin::Network) -> Result<Address> {
    if public_key.key_type() != KeyType::Bitcoin {
        return Err(Error::KeyDerivation("Not a Bitcoin public key".to_string()));
    }
    
    let address = crate::crypto::keys::bitcoin::public_key_to_address(public_key, network)?;
    
    Ok(Address::new(address, KeyType::Bitcoin, path.to_string()))
}

/// Derive an address from a public key
pub fn derive_address(public_key: &PublicKey, path: &str) -> Result<Address> {
    match public_key.key_type() {
        KeyType::Ethereum => derive_ethereum_address(public_key, path),
        KeyType::Solana => derive_solana_address(public_key, path),
        KeyType::Bitcoin => derive_bitcoin_address(public_key, path, bitcoin::Network::Bitcoin),
    }
}

/// Validate an address for a specific blockchain
pub fn validate_address(address: &str, key_type: KeyType) -> Result<bool> {
    match key_type {
        KeyType::Ethereum => {
            // Basic validation: check if it starts with "0x" and has the correct length
            if !address.starts_with("0x") || address.len() != 42 {
                return Ok(false);
            }
            
            // Check if it's a valid hex string
            match hex::decode(&address[2..]) {
                Ok(bytes) => {
                    if bytes.len() != 20 {
                        return Ok(false);
                    }
                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        }
        KeyType::Solana => {
            // Basic validation: check if it has the correct length
            if address.len() != 44 && address.len() != 43 {
                return Ok(false);
            }
            
            // Check if it's a valid base58 string
            match bs58::decode(address).into_vec() {
                Ok(bytes) => {
                    if bytes.len() != 32 {
                        return Ok(false);
                    }
                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        }
        KeyType::Bitcoin => {
            // This is a simplified validation for Bitcoin addresses
            // In a real implementation, we would use the bitcoin crate
            if address.starts_with("1") || address.starts_with("3") || address.starts_with("bc1") {
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::{KeyPair, PrivateKey, PublicKey};

    #[test]
    fn test_address_creation() {
        let address = Address::new(
            "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            KeyType::Ethereum,
            "m/44'/60'/0'/0/0".to_string(),
        );
        
        assert_eq!(address.as_str(), "0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        assert_eq!(address.key_type(), KeyType::Ethereum);
        assert_eq!(address.path(), "m/44'/60'/0'/0/0");
    }

    #[test]
    fn test_validate_ethereum_address() {
        // Valid Ethereum address
        assert!(validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e", KeyType::Ethereum).unwrap());
        
        // Invalid Ethereum addresses
        assert!(!validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44", KeyType::Ethereum).unwrap()); // Too short
        assert!(!validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e1", KeyType::Ethereum).unwrap()); // Too long
        assert!(!validate_address("742d35Cc6634C0532925a3b844Bc454e4438f44e", KeyType::Ethereum).unwrap()); // Missing 0x
        assert!(!validate_address("0xZZZd35Cc6634C0532925a3b844Bc454e4438f44e", KeyType::Ethereum).unwrap()); // Invalid hex
    }

    #[test]
    fn test_validate_solana_address() {
        // Valid Solana address (this is a placeholder, not a real address)
        assert!(validate_address("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg", KeyType::Solana).unwrap());
        
        // Invalid Solana addresses
        assert!(!validate_address("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKP", KeyType::Solana).unwrap()); // Too short
        assert!(!validate_address("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTgXX", KeyType::Solana).unwrap()); // Too long
    }
}
