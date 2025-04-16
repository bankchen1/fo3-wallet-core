//! Mnemonic phrase generation and handling

use bip39::{Mnemonic, MnemonicType, Language, Seed};
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
    /// Convert to BIP39 mnemonic type
    fn to_mnemonic_type(&self) -> MnemonicType {
        match self {
            Self::Words12 => MnemonicType::Words12,
            Self::Words24 => MnemonicType::Words24,
        }
    }
}

/// Generate a new random mnemonic phrase with the specified strength
pub fn generate_mnemonic(strength: MnemonicStrength) -> Result<String> {
    let mnemonic = Mnemonic::new(strength.to_mnemonic_type(), Language::English);
    Ok(mnemonic.phrase().to_string())
}

/// Validate a mnemonic phrase
pub fn validate_mnemonic(phrase: &str) -> Result<bool> {
    match Mnemonic::from_phrase(phrase, Language::English) {
        Ok(_) => Ok(true),
        Err(e) => Err(Error::Unknown(e.to_string())),
    }
}

/// Generate a seed from a mnemonic phrase and optional passphrase
pub fn mnemonic_to_seed(phrase: &str, passphrase: Option<&str>) -> Result<Vec<u8>> {
    let mnemonic = Mnemonic::from_phrase(phrase, Language::English)
        .map_err(|e| Error::Unknown(e.to_string()))?;

    let seed = Seed::new(&mnemonic, passphrase.unwrap_or(""));
    Ok(seed.as_bytes().to_vec())
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

        // Known test vector for this seed
        assert_eq!(hex::encode(&seed[0..8]), "5eb00bbddcf069b3");
    }
}
