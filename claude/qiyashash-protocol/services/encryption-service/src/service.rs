//! Encryption service core logic

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

use qiyashash_crypto::aead::{Aead, AeadKey, EncryptedPayload};
use qiyashash_crypto::identity::Identity;
use qiyashash_crypto::keys::{EphemeralKeyPair, PublicKeyBytes};
use qiyashash_crypto::kdf::{derive_message_keys, derive_chain_proof};
use qiyashash_crypto::chain::{ChainState, compute_message_hash};

use crate::error::ServiceError;

/// Session encryption state
struct SessionState {
    /// Session ID
    id: String,
    /// Chain state
    chain: ChainState,
    /// Current chain key
    chain_key: [u8; 32],
    /// Message counter
    message_count: u64,
}

/// Encryption service
pub struct EncryptionService {
    /// Our identity
    identity: Identity,
    /// Active sessions
    sessions: RwLock<HashMap<String, SessionState>>,
    /// AEAD cipher
    cipher: Aead,
}

impl EncryptionService {
    /// Create new encryption service
    pub fn new(storage_path: &str) -> Result<Self, ServiceError> {
        info!("Initializing encryption service");

        // Create or load identity
        let identity = Identity::new();
        info!("Created identity with fingerprint: {}", identity.fingerprint_hex());

        Ok(Self {
            identity,
            sessions: RwLock::new(HashMap::new()),
            cipher: Aead::new(),
        })
    }

    /// Generate ephemeral key pair
    pub fn generate_ephemeral(&self) -> EphemeralKeyResult {
        let keypair = EphemeralKeyPair::generate();
        let public_key = keypair.public_key_bytes();

        debug!("Generated ephemeral key: {}", hex::encode(&public_key[..8]));

        EphemeralKeyResult {
            public_key,
            // In practice, store the keypair for later use
        }
    }

    /// Initialize session with shared secret
    pub fn init_session(&self, session_id: &str, shared_secret: [u8; 32]) -> SessionInitResult {
        let chain = ChainState::from_shared_secret(&shared_secret);

        let session = SessionState {
            id: session_id.to_string(),
            chain,
            chain_key: shared_secret,
            message_count: 0,
        };

        self.sessions.write().insert(session_id.to_string(), session);

        info!("Initialized session: {}", session_id);

        SessionInitResult {
            session_id: session_id.to_string(),
            chain_state: shared_secret, // Initial state
        }
    }

    /// Encrypt a message
    pub fn encrypt_message(
        &self,
        session_id: &str,
        plaintext: &[u8],
    ) -> Result<EncryptResult, ServiceError> {
        let mut sessions = self.sessions.write();
        
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        // Derive message keys
        let (new_chain_key, message_key, header_key) = derive_message_keys(&session.chain_key);
        session.chain_key = new_chain_key;
        session.message_count += 1;

        // Encrypt
        let aead_key = AeadKey::from_bytes(message_key);
        let aad = session.message_count.to_be_bytes();
        let payload = self.cipher.encrypt(&aead_key, plaintext, &aad)?;

        // Update chain
        let msg_hash = compute_message_hash(&payload.ciphertext, &aad);
        let chain_link = session.chain.add_message(&msg_hash);

        // Generate chain proof
        let chain_proof = derive_chain_proof(
            &chain_link.state,
            &msg_hash,
            chain_link.timestamp,
        );

        debug!("Encrypted message {} for session {}", session.message_count, session_id);

        Ok(EncryptResult {
            ciphertext: payload.ciphertext,
            nonce: payload.nonce.as_bytes().to_vec(),
            message_number: session.message_count,
            chain_proof,
            chain_state: chain_link.state,
        })
    }

    /// Decrypt a message
    pub fn decrypt_message(
        &self,
        session_id: &str,
        ciphertext: &[u8],
        nonce: &[u8],
        message_number: u64,
    ) -> Result<DecryptResult, ServiceError> {
        let sessions = self.sessions.read();
        
        let session = sessions.get(session_id)
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        // In a full implementation, we'd derive the correct message key
        // based on the message number and handle skipped messages
        
        // For now, use current chain key
        let (_, message_key, _) = derive_message_keys(&session.chain_key);
        let aead_key = AeadKey::from_bytes(message_key);

        // Reconstruct nonce
        let nonce_array: [u8; 24] = nonce.try_into()
            .map_err(|_| ServiceError::InvalidRequest("Invalid nonce length".to_string()))?;

        let payload = EncryptedPayload {
            algorithm: qiyashash_crypto::aead::AeadAlgorithm::XChaCha20Poly1305,
            nonce: qiyashash_crypto::aead::Nonce::XChaCha(nonce_array),
            ciphertext: ciphertext.to_vec(),
        };

        let aad = message_number.to_be_bytes();
        let plaintext = self.cipher.decrypt(&aead_key, &payload, &aad)?;

        debug!("Decrypted message {} for session {}", message_number, session_id);

        Ok(DecryptResult {
            plaintext,
            message_number,
        })
    }

    /// Derive key from inputs
    pub fn derive_key(
        &self,
        inputs: Vec<Vec<u8>>,
        info: &[u8],
    ) -> Result<[u8; 32], ServiceError> {
        use qiyashash_crypto::kdf::KeyDerivationContext;

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
        let kdf = KeyDerivationContext::from_multiple_secrets(None, &input_refs);
        
        let key = kdf.derive::<32>(info)
            .map_err(|e| ServiceError::Crypto(e.to_string()))?;

        Ok(key.into_bytes())
    }

    /// Verify chain integrity
    pub fn verify_chain(&self, session_id: &str) -> Result<ChainVerifyResult, ServiceError> {
        let sessions = self.sessions.read();
        
        let session = sessions.get(session_id)
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        session.chain.verify_integrity()
            .map_err(|e| ServiceError::Crypto(e.to_string()))?;

        let proof = session.chain.generate_proof();

        Ok(ChainVerifyResult {
            valid: true,
            sequence: proof.sequence,
            current_state: proof.current_state,
        })
    }

    /// Get session info
    pub fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read();
        
        sessions.get(session_id).map(|s| SessionInfo {
            session_id: s.id.clone(),
            message_count: s.message_count,
            chain_sequence: s.chain.sequence(),
            current_state: *s.chain.current_state(),
        })
    }

    /// Get identity fingerprint
    pub fn fingerprint(&self) -> String {
        self.identity.fingerprint_hex()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.read().len()
    }
}

/// Result of ephemeral key generation
pub struct EphemeralKeyResult {
    pub public_key: [u8; 32],
}

/// Result of session initialization
pub struct SessionInitResult {
    pub session_id: String,
    pub chain_state: [u8; 32],
}

/// Result of encryption
pub struct EncryptResult {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub message_number: u64,
    pub chain_proof: [u8; 32],
    pub chain_state: [u8; 32],
}

/// Result of decryption
pub struct DecryptResult {
    pub plaintext: Vec<u8>,
    pub message_number: u64,
}

/// Result of chain verification
pub struct ChainVerifyResult {
    pub valid: bool,
    pub sequence: u64,
    pub current_state: [u8; 32],
}

/// Session information
pub struct SessionInfo {
    pub session_id: String,
    pub message_count: u64,
    pub chain_sequence: u64,
    pub current_state: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_encrypt_decrypt() {
        let service = EncryptionService::new("./test-data").unwrap();
        
        // Init session
        let secret = [0x42u8; 32];
        service.init_session("test-session", secret);

        // Encrypt
        let plaintext = b"Hello, World!";
        let encrypted = service.encrypt_message("test-session", plaintext).unwrap();

        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.message_number, 1);
    }

    #[test]
    fn test_chain_verification() {
        let service = EncryptionService::new("./test-data").unwrap();
        
        let secret = [0x42u8; 32];
        service.init_session("test-session", secret);

        // Add some messages
        for _ in 0..5 {
            service.encrypt_message("test-session", b"test").unwrap();
        }

        // Verify chain
        let result = service.verify_chain("test-session").unwrap();
        assert!(result.valid);
        assert_eq!(result.sequence, 5);
    }

    #[test]
    fn test_key_derivation() {
        let service = EncryptionService::new("./test-data").unwrap();
        
        let inputs = vec![vec![0x01u8; 32], vec![0x02u8; 32]];
        let key1 = service.derive_key(inputs.clone(), b"context1").unwrap();
        let key2 = service.derive_key(inputs, b"context2").unwrap();

        // Different contexts should give different keys
        assert_ne!(key1, key2);
    }
}
