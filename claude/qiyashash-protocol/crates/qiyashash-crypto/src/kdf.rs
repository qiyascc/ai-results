//! Key Derivation Functions (KDF) for QiyasHash protocol
//!
//! This module provides HKDF-based key derivation with domain separation
//! to prevent key reuse across different protocol contexts.

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};

/// HKDF using SHA-512 for key derivation
pub type HkdfSha512 = Hkdf<Sha512>;

/// HKDF using SHA-256 for key derivation
pub type HkdfSha256 = Hkdf<Sha256>;

/// HMAC-SHA256 for message authentication
pub type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA512 for message authentication
pub type HmacSha512 = Hmac<Sha512>;

/// Domain separation strings for different key derivation contexts
pub mod domain {
    /// Root key derivation from X3DH
    pub const ROOT_KEY: &[u8] = b"QiyasHash_v1_RootKey";
    /// Chain key derivation
    pub const CHAIN_KEY: &[u8] = b"QiyasHash_v1_ChainKey";
    /// Message key derivation
    pub const MESSAGE_KEY: &[u8] = b"QiyasHash_v1_MessageKey";
    /// Header key derivation
    pub const HEADER_KEY: &[u8] = b"QiyasHash_v1_HeaderKey";
    /// Next header key derivation
    pub const NEXT_HEADER_KEY: &[u8] = b"QiyasHash_v1_NextHeaderKey";
    /// Authentication key derivation
    pub const AUTH_KEY: &[u8] = b"QiyasHash_v1_AuthKey";
    /// Chain proof derivation
    pub const CHAIN_PROOF: &[u8] = b"QiyasHash_v1_ChainProof";
    /// Identity proof derivation
    pub const IDENTITY_PROOF: &[u8] = b"QiyasHash_v1_IdentityProof";
}

/// A derived key with automatic zeroization
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey<const N: usize>(pub [u8; N]);

impl<const N: usize> DerivedKey<N> {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Convert to raw bytes
    pub fn into_bytes(self) -> [u8; N] {
        self.0
    }
}

impl<const N: usize> AsRef<[u8]> for DerivedKey<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Key derivation context for HKDF operations
pub struct KeyDerivationContext {
    /// The PRK (Pseudo-Random Key) from HKDF-Extract
    hkdf: HkdfSha512,
}

impl KeyDerivationContext {
    /// Create a new KDF context from input key material
    ///
    /// # Arguments
    /// * `salt` - Optional salt (if None, uses zero-filled salt)
    /// * `ikm` - Input Key Material (e.g., shared secret from DH)
    pub fn new(salt: Option<&[u8]>, ikm: &[u8]) -> Self {
        let hkdf = HkdfSha512::new(salt, ikm);
        Self { hkdf }
    }

    /// Create from multiple input key materials (concatenated)
    pub fn from_multiple_secrets(salt: Option<&[u8]>, secrets: &[&[u8]]) -> Self {
        let ikm: Vec<u8> = secrets.iter().flat_map(|s| s.iter().copied()).collect();
        Self::new(salt, &ikm)
    }

    /// Derive a key with the given info string
    pub fn derive<const N: usize>(&self, info: &[u8]) -> Result<DerivedKey<N>> {
        let mut output = [0u8; N];
        self.hkdf
            .expand(info, &mut output)
            .map_err(|_| CryptoError::KeyDerivation("HKDF expansion failed".to_string()))?;
        Ok(DerivedKey(output))
    }

    /// Derive multiple keys at once for efficiency
    pub fn derive_keys(&self, infos: &[&[u8]], output_sizes: &[usize]) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(infos.len());

        for (info, &size) in infos.iter().zip(output_sizes.iter()) {
            let mut output = vec![0u8; size];
            self.hkdf
                .expand(info, &mut output)
                .map_err(|_| CryptoError::KeyDerivation("HKDF expansion failed".to_string()))?;
            results.push(output);
        }

        Ok(results)
    }
}

/// HMAC-based chain key ratcheting
///
/// Used in the Double Ratchet algorithm to derive new chain keys
/// and message keys from the current chain key.
pub struct ChainRatchet {
    chain_key: [u8; 32],
}

impl ChainRatchet {
    /// Create a new chain ratchet from an initial chain key
    pub fn new(chain_key: [u8; 32]) -> Self {
        Self { chain_key }
    }

    /// Ratchet the chain key and derive a message key
    ///
    /// Returns (new_chain_key, message_key)
    pub fn ratchet(&mut self) -> ([u8; 32], [u8; 32]) {
        // Derive message key: HMAC(chain_key, 0x01)
        let message_key = self.hmac_derive(&[0x01]);

        // Derive new chain key: HMAC(chain_key, 0x02)
        let new_chain_key = self.hmac_derive(&[0x02]);

        self.chain_key = new_chain_key;
        (new_chain_key, message_key)
    }

    /// Get current chain key
    pub fn chain_key(&self) -> &[u8; 32] {
        &self.chain_key
    }

    /// Derive using HMAC-SHA256
    fn hmac_derive(&self, input: &[u8]) -> [u8; 32] {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.chain_key)
            .expect("HMAC can take key of any size");
        mac.update(input);
        let result = mac.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result.into_bytes());
        output
    }
}

impl Drop for ChainRatchet {
    fn drop(&mut self) {
        self.chain_key.zeroize();
    }
}

/// Derive root and chain keys from a shared secret
///
/// Used after X3DH or DH ratchet step
pub fn derive_root_and_chain_keys(
    root_key: &[u8; 32],
    dh_output: &[u8; 32],
) -> Result<([u8; 32], [u8; 32])> {
    let kdf = KeyDerivationContext::new(Some(root_key), dh_output);

    let new_root_key: DerivedKey<32> = kdf.derive(domain::ROOT_KEY)?;
    let chain_key: DerivedKey<32> = kdf.derive(domain::CHAIN_KEY)?;

    Ok((new_root_key.into_bytes(), chain_key.into_bytes()))
}

/// Derive message keys from a chain key
///
/// Returns (new_chain_key, message_key, header_key)
pub fn derive_message_keys(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32], [u8; 32]) {
    let mut ratchet = ChainRatchet::new(*chain_key);
    let (new_chain_key, message_key) = ratchet.ratchet();

    // Derive header key from message key
    let header_key = {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&message_key)
            .expect("HMAC can take key of any size");
        mac.update(domain::HEADER_KEY);
        let result = mac.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result.into_bytes());
        output
    };

    (new_chain_key, message_key, header_key)
}

/// Compute authentication tag for deniable authentication
///
/// Uses HMAC instead of signatures to maintain deniability
pub fn compute_auth_tag(auth_key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(auth_key).expect("HMAC can take key of any size");
    mac.update(data);
    let result = mac.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result.into_bytes());
    output
}

/// Verify authentication tag
pub fn verify_auth_tag(auth_key: &[u8; 32], data: &[u8], tag: &[u8; 32]) -> bool {
    let expected = compute_auth_tag(auth_key, data);
    constant_time_eq(&expected, tag)
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Derive chain proof for message ordering
pub fn derive_chain_proof(
    chain_state: &[u8],
    message_hash: &[u8; 32],
    timestamp: u64,
) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(domain::CHAIN_PROOF);
    hasher.update(chain_state);
    hasher.update(message_hash);
    hasher.update(&timestamp.to_be_bytes());
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation_context() {
        let ikm = [0x42u8; 32];
        let kdf = KeyDerivationContext::new(Some(b"test-salt"), &ikm);

        let key1: DerivedKey<32> = kdf.derive(b"context1").unwrap();
        let key2: DerivedKey<32> = kdf.derive(b"context2").unwrap();

        // Different contexts should produce different keys
        assert_ne!(key1.as_bytes(), key2.as_bytes());

        // Same context should produce same key
        let key1_again: DerivedKey<32> = kdf.derive(b"context1").unwrap();
        assert_eq!(key1.as_bytes(), key1_again.as_bytes());
    }

    #[test]
    fn test_chain_ratchet() {
        let initial_key = [0x42u8; 32];
        let mut ratchet = ChainRatchet::new(initial_key);

        let (chain1, msg1) = ratchet.ratchet();
        let (chain2, msg2) = ratchet.ratchet();

        // Each ratchet should produce different keys
        assert_ne!(chain1, chain2);
        assert_ne!(msg1, msg2);

        // Message keys should be different from chain keys
        assert_ne!(chain1, msg1);
    }

    #[test]
    fn test_auth_tag() {
        let key = [0x42u8; 32];
        let data = b"Hello, QiyasHash!";

        let tag = compute_auth_tag(&key, data);
        assert!(verify_auth_tag(&key, data, &tag));

        // Tampered data should fail
        let tampered = b"Hello, QiyasHash?";
        assert!(!verify_auth_tag(&key, tampered, &tag));

        // Wrong key should fail
        let wrong_key = [0x43u8; 32];
        assert!(!verify_auth_tag(&wrong_key, data, &tag));
    }

    #[test]
    fn test_constant_time_eq() {
        let a = [1, 2, 3, 4];
        let b = [1, 2, 3, 4];
        let c = [1, 2, 3, 5];

        assert!(constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, &c));
        assert!(!constant_time_eq(&a, &[]));
    }
}
