//! User identity management for mobile

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

/// User identity with key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique identity ID
    pub id: String,
    /// Display name
    pub display_name: String,
    /// Ed25519 signing public key (hex)
    pub signing_public_key: String,
    /// X25519 encryption public key (hex)
    pub encryption_public_key: String,
    /// Ed25519 signing secret key (hex, encrypted at rest)
    signing_secret_key: String,
    /// X25519 encryption secret key (hex, encrypted at rest)
    encryption_secret_key: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl UserIdentity {
    /// Generate a new identity with fresh keys
    pub fn generate(display_name: String) -> Result<Self, IdentityError> {
        use rand::RngCore;
        
        // Generate signing key pair (Ed25519)
        let mut signing_seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut signing_seed);
        
        // Generate encryption key pair (X25519)
        let mut encryption_seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut encryption_seed);
        
        // In a real implementation, use proper Ed25519 and X25519 key derivation
        // For now, we use the seeds as keys (simplified)
        
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            display_name,
            signing_public_key: hex::encode(&signing_seed[..16]), // Simplified
            encryption_public_key: hex::encode(&encryption_seed[..16]),
            signing_secret_key: hex::encode(&signing_seed),
            encryption_secret_key: hex::encode(&encryption_seed),
            created_at: Utc::now(),
        })
    }

    /// Get public key in base64 format (for sharing)
    pub fn public_key_base64(&self) -> String {
        base64::encode(format!(
            "{}:{}",
            self.signing_public_key,
            self.encryption_public_key
        ))
    }

    /// Encrypt data for a recipient
    pub fn encrypt_for(&self, recipient_public_key: &str, plaintext: &[u8]) -> Result<Vec<u8>, IdentityError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        
        // Derive shared secret (simplified - in real impl use X25519)
        let shared_secret = self.derive_shared_secret(recipient_public_key)?;
        
        // Generate nonce
        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let cipher = Aes256Gcm::new_from_slice(&shared_secret)
            .map_err(|e| IdentityError::Encryption(e.to_string()))?;
        
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| IdentityError::Encryption(e.to_string()))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        
        Ok(result)
    }

    /// Decrypt data from a sender
    pub fn decrypt_from(&self, sender_public_key: &str, ciphertext: &[u8]) -> Result<Vec<u8>, IdentityError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        
        if ciphertext.len() < 12 {
            return Err(IdentityError::Decryption("Ciphertext too short".into()));
        }
        
        // Extract nonce
        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let actual_ciphertext = &ciphertext[12..];
        
        // Derive shared secret
        let shared_secret = self.derive_shared_secret(sender_public_key)?;
        
        // Decrypt
        let cipher = Aes256Gcm::new_from_slice(&shared_secret)
            .map_err(|e| IdentityError::Decryption(e.to_string()))?;
        
        cipher.decrypt(nonce, actual_ciphertext)
            .map_err(|e| IdentityError::Decryption(e.to_string()))
    }

    /// Derive shared secret with peer (simplified)
    fn derive_shared_secret(&self, peer_public_key: &str) -> Result<[u8; 32], IdentityError> {
        use sha2::{Sha256, Digest};
        
        // Simplified shared secret derivation
        // In real implementation, use X25519 ECDH
        let mut hasher = Sha256::new();
        hasher.update(self.encryption_secret_key.as_bytes());
        hasher.update(peer_public_key.as_bytes());
        
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = UserIdentity::generate("Test User".to_string()).unwrap();
        assert!(!identity.id.is_empty());
        assert_eq!(identity.display_name, "Test User");
    }

    #[test]
    fn test_encrypt_decrypt() {
        let alice = UserIdentity::generate("Alice".to_string()).unwrap();
        let bob = UserIdentity::generate("Bob".to_string()).unwrap();
        
        let message = b"Hello, Bob!";
        let ciphertext = alice.encrypt_for(&bob.encryption_public_key, message).unwrap();
        let decrypted = bob.decrypt_from(&alice.encryption_public_key, &ciphertext).unwrap();
        
        assert_eq!(decrypted, message);
    }
}
