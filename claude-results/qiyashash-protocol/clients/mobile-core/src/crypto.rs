//! Mobile crypto utilities

use crate::MobileResult;
use rand::RngCore;

/// Mobile crypto utilities
pub struct MobileCrypto;

impl MobileCrypto {
    /// Generate a random session key (32 bytes)
    pub fn generate_session_key() -> MobileResult<Vec<u8>> {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    /// Hash data with SHA-256
    pub fn sha256(data: &[u8]) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}
