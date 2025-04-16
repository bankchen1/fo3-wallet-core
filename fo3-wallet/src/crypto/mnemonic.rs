//! Mnemonic phrase generation and handling

use bip39::Mnemonic;
use rand::{rngs::OsRng, RngCore};
use crate::error::{Error, Result};

/// Supported mnemonic strengths
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MnemonicStrength {
    /// 12 words (128 bits)
    Words12,
    /// 24 words (256 bits)
    Words24,
}

impl MnemonicStrength {
    /// Get entropy length in bytes
    fn entropy_bytes(&self) -> usize {
        match self {
            Self::Words12 => 16, // 128 bits = 16 bytes
            Self::Words24 => 32, // 256 bits = 32 bytes
        }
    }
}

/// Generate a new random mnemonic phrase with the specified strength
pub fn generate_mnemonic(strength: MnemonicStrength) -> Result<String> {
    let mut entropy = vec![0u8; strength.entropy_bytes()];
    OsRng.fill_bytes(&mut entropy);

    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| Error::Mnemonic(e.to_string()))?;

    Ok(mnemonic.to_string())
}

/// Validate a mnemonic phrase
pub fn validate_mnemonic(phrase: &str) -> Result<bool> {
    match Mnemonic::parse_normalized(phrase) {
        Ok(_) => Ok(true),
        Err(e) => Err(Error::Mnemonic(e.to_string())),
    }
}

/// Generate a seed from a mnemonic phrase and optional passphrase
pub fn mnemonic_to_seed(phrase: &str, passphrase: Option<&str>) -> Result<Vec<u8>> {
    let mnemonic = Mnemonic::parse_normalized(phrase)
        .map_err(|e| Error::Mnemonic(e.to_string()))?;

    let seed = mnemonic.to_seed(passphrase.unwrap_or(""));
    Ok(seed.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic() {
        let mnemonic = generate_mnemonic(MnemonicStrength::Words12).unwrap();
        assert!(validate_mnemonic(&mnemonic).unwrap());

        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_validate_mnemonic() {
        let valid = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let invalid = "invalid mnemonic phrase test test test test test test test test test";

        assert!(validate_mnemonic(valid).unwrap());
        assert!(validate_mnemonic(invalid).is_err());
    }

    #[test]
    fn test_mnemonic_to_seed() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = mnemonic_to_seed(mnemonic, None).unwrap();

        // Just check that we get a valid seed
        assert!(!seed.is_empty());
        assert_eq!(seed.len(), 64); // BIP39 seeds are 512 bits (64 bytes)
    }
}
