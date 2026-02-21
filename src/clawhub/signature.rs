use crate::clawhub::{ClawHubError, Result};
use ed25519_dalek::{Signature as EdSignature, Signer, SigningKey, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Signature verification for ClawHub skills
///
/// Uses Ed25519 for cryptographic signatures to verify:
/// - Author signatures (signed by skill author)
/// - Market signatures (signed by trusted marketplace)

/// A key pair for signing skills
#[derive(Clone)]
pub struct SkillKeyPair {
    pub signing_key: SigningKey,
}

impl SkillKeyPair {
    /// Generate a new key pair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Get the verifying key
    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key().as_bytes())
    }

    /// Get the public key as base64 string
    pub fn public_key_base64(&self) -> String {
        base64::encode(self.verifying_key().as_bytes())
    }

    /// Export secret key as bytes
    pub fn to_secret_key_bytes(&self) -> [u8; 32] {
        *self.signing_key.as_bytes()
    }
}

/// Verify a signature on a skill
pub fn verify_skill_signature(
    skill_data: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool> {
    // Decode signature
    let signature_bytes = hex::decode(signature_hex).map_err(|e| {
        ClawHubError::InvalidFormat {
            file: "signature".to_string(),
            reason: format!("Invalid hex encoding: {}", e),
        }
    })?;

    let signature = EdSignature::from_slice(&signature_bytes).map_err(|_| {
        ClawHubError::InvalidFormat {
            file: "signature".to_string(),
            reason: "Invalid signature format".to_string(),
        }
    })?;

    // Decode public key
    let public_key_bytes = hex::decode(public_key_hex).map_err(|e| {
        ClawHubError::InvalidFormat {
            file: "public_key".to_string(),
            reason: format!("Invalid hex encoding: {}", e),
        }
    })?;

    if public_key_bytes.len() != 32 {
        return Err(ClawHubError::InvalidFormat {
            file: "public_key".to_string(),
            reason: format!("Invalid public key length: {}", public_key_bytes.len()),
        });
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&public_key_bytes);

    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&key_array).map_err(|_| {
        ClawHubError::InvalidFormat {
            file: "public_key".to_string(),
            reason: "Invalid public key format".to_string(),
        }
    })?;

    // Verify signature
    verifying_key
        .verify(skill_data, &signature)
        .map_err(|_| ClawHubError::InvalidFormat {
            file: "signature".to_string(),
            reason: "Signature verification failed".to_string(),
        })?;

    Ok(true)
}

/// Sign a skill with author key
pub fn sign_skill(skill_data: &[u8], keypair: &SkillKeyPair) -> String {
    let signature = keypair.signing_key.sign(skill_data);
    hex::encode(signature.to_bytes())
}

/// Verify SIF signature from SIF structure
pub fn verify_sif_signature(
    sif: &crate::clawhub::sif::SkillSIF,
) -> Result<Vec<SignatureVerification>> {
    let mut results = Vec::new();

    let signature = match &sif.signature {
        Some(s) => s,
        None => return Ok(results), // No signature to verify
    };

    // Serialize SIF for verification (excluding signature field itself)
    let _sif_json = serde_json::to_string(sif).map_err(|e| ClawHubError::InvalidFormat {
        file: "SIF".to_string(),
        reason: format!("Failed to serialize SIF: {}", e),
    })?;

    // Verify author signature if present
    if let Some(_author_sig) = &signature.author_signature {
        // Note: In real implementation, we'd need a way to get the author's public key
        // For now, we just return a verification result indicating the key is needed
        results.push(SignatureVerification {
            verified: false,
            signer: "author".to_string(),
            message: "Author public key needed for verification".to_string(),
        });
    }

    // Verify market signature if present
    if let Some(_market_sig) = &signature.market_signature {
        // Note: Market public key would come from trusted marketplace config
        results.push(SignatureVerification {
            verified: false,
            signer: "market".to_string(),
            message: "Market public key needed for verification".to_string(),
        });
    }

    Ok(results)
}

/// Result of signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVerification {
    pub verified: bool,
    pub signer: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = SkillKeyPair::generate();
        let hex = keypair.public_key_hex();
        assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_keypair_export() {
        let keypair = SkillKeyPair::generate();
        let secret_bytes = keypair.to_secret_key_bytes();
        assert_eq!(secret_bytes.len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = SkillKeyPair::generate();
        let skill_data = b"test skill content";

        let signature = sign_skill(skill_data, &keypair);
        let verified = verify_skill_signature(skill_data, &signature, &keypair.public_key_hex()).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_verify_invalid_signature_fails() {
        let keypair = SkillKeyPair::generate();
        let skill_data = b"test skill content";

        let signature = "invalidsignature";
        let result = verify_skill_signature(skill_data, signature, &keypair.public_key_hex());

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_tampered_data_fails() {
        let keypair = SkillKeyPair::generate();
        let skill_data = b"test skill content";

        let signature = sign_skill(skill_data, &keypair);
        let tampered_data = b"tampered content";

        let result = verify_skill_signature(tampered_data, &signature, &keypair.public_key_hex());

        assert!(result.is_err());
    }
}
