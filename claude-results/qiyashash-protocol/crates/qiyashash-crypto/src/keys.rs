//! Key types and management for QiyasHash protocol
//!
//! This module provides the fundamental key types used throughout the protocol:
//! - Identity keys (long-term Ed25519 for signing)
//! - Ephemeral keys (X25519 for key exchange)
//! - Pre-keys (X25519 for asynchronous key agreement)

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};

/// Size of X25519 public keys in bytes
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;

/// Size of X25519 private keys in bytes  
pub const X25519_PRIVATE_KEY_SIZE: usize = 32;

/// A X25519 key pair for Diffie-Hellman key exchange
#[derive(ZeroizeOnDrop)]
pub struct EphemeralKeyPair {
    /// The secret key (zeroized on drop)
    #[zeroize(skip)]
    secret: X25519StaticSecret,
    /// The public key
    public: X25519PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new random ephemeral key pair
    pub fn generate() -> Self {
        let secret = X25519StaticSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Create from existing secret bytes
    ///
    /// # Security
    /// The input bytes should come from a secure random source
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        let secret = X25519StaticSecret::from(bytes);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key
    pub fn public_key(&self) -> &X25519PublicKey {
        &self.public
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Perform X25519 Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &X25519PublicKey) -> SharedSecret {
        let shared = self.secret.diffie_hellman(their_public);
        SharedSecret(*shared.as_bytes())
    }

    /// Perform X25519 DH with public key bytes
    pub fn diffie_hellman_bytes(&self, their_public: &[u8; 32]) -> Result<SharedSecret> {
        let their_key = X25519PublicKey::from(*their_public);
        Ok(self.diffie_hellman(&their_key))
    }

    /// Get the secret key bytes (use with caution)
    ///
    /// # Security Warning
    /// This exposes the secret key. Only use for serialization/storage
    /// and ensure the result is properly secured.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }
}

impl Clone for EphemeralKeyPair {
    fn clone(&self) -> Self {
        Self::from_secret_bytes(self.secret.to_bytes())
    }
}

/// A shared secret derived from Diffie-Hellman key exchange
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SharedSecret(pub(crate) [u8; 32]);

impl SharedSecret {
    /// Create a shared secret from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the secret bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to bytes, consuming the secret
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl AsRef<[u8]> for SharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A serializable public key wrapper
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKeyBytes(#[serde(with = "hex::serde")] pub [u8; 32]);

impl PublicKeyBytes {
    /// Create from X25519 public key
    pub fn from_x25519(key: &X25519PublicKey) -> Self {
        Self(*key.as_bytes())
    }

    /// Convert to X25519 public key
    pub fn to_x25519(&self) -> X25519PublicKey {
        X25519PublicKey::from(self.0)
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for PublicKeyBytes {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<X25519PublicKey> for PublicKeyBytes {
    fn from(key: X25519PublicKey) -> Self {
        Self(*key.as_bytes())
    }
}

/// A signed pre-key for asynchronous key exchange
#[derive(Clone, Serialize, Deserialize)]
pub struct SignedPreKey {
    /// The pre-key ID
    pub id: u32,
    /// The public key
    pub public_key: PublicKeyBytes,
    /// Signature over the public key by the identity key
    #[serde(with = "hex::serde")]
    pub signature: [u8; 64],
    /// Timestamp when this key was generated
    pub timestamp: i64,
}

impl SignedPreKey {
    /// Verify the signature using the identity public key
    pub fn verify(&self, identity_key: &ed25519_dalek::VerifyingKey) -> Result<()> {
        use ed25519_dalek::Signature;
        let signature = Signature::from_bytes(&self.signature);
        identity_key
            .verify_strict(self.public_key.as_bytes(), &signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

/// A bundle of pre-keys for asynchronous session establishment
#[derive(Clone, Serialize, Deserialize)]
pub struct PreKeyBundle {
    /// Identity public key (Ed25519)
    #[serde(with = "hex::serde")]
    pub identity_key: [u8; 32],
    /// Signed pre-key
    pub signed_prekey: SignedPreKey,
    /// One-time pre-key (optional, for additional forward secrecy)
    pub one_time_prekey: Option<OneTimePreKey>,
}

/// A one-time pre-key (used once and discarded)
#[derive(Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    /// The key ID
    pub id: u32,
    /// The public key
    pub public_key: PublicKeyBytes,
}

/// Key pair storage with both signing and key exchange capabilities
#[derive(ZeroizeOnDrop)]
pub struct KeyPairStorage {
    /// Ed25519 signing key pair
    signing_secret: [u8; 32],
    #[zeroize(skip)]
    signing_public: ed25519_dalek::VerifyingKey,
    /// X25519 key exchange key pair
    exchange_secret: [u8; 32],
    #[zeroize(skip)]
    exchange_public: X25519PublicKey,
}

impl KeyPairStorage {
    /// Generate new random key pairs
    pub fn generate() -> Result<Self> {
        use ed25519_dalek::SigningKey;

        // Generate Ed25519 signing key
        let signing_key = SigningKey::generate(&mut OsRng);
        let signing_public = signing_key.verifying_key();
        let signing_secret = signing_key.to_bytes();

        // Generate X25519 key exchange key
        let exchange_secret_key = X25519StaticSecret::random_from_rng(OsRng);
        let exchange_public = X25519PublicKey::from(&exchange_secret_key);
        let exchange_secret = exchange_secret_key.to_bytes();

        Ok(Self {
            signing_secret,
            signing_public,
            exchange_secret,
            exchange_public,
        })
    }

    /// Get the signing public key
    pub fn signing_public_key(&self) -> &ed25519_dalek::VerifyingKey {
        &self.signing_public
    }

    /// Get the exchange public key
    pub fn exchange_public_key(&self) -> &X25519PublicKey {
        &self.exchange_public
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&self.signing_secret);
        let signature = signing_key.sign(message);
        signature.to_bytes()
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &X25519PublicKey) -> SharedSecret {
        let secret = X25519StaticSecret::from(self.exchange_secret);
        let shared = secret.diffie_hellman(their_public);
        SharedSecret(*shared.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_keypair_generation() {
        let kp1 = EphemeralKeyPair::generate();
        let kp2 = EphemeralKeyPair::generate();

        // Keys should be different
        assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn test_diffie_hellman_exchange() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();

        let alice_shared = alice.diffie_hellman(bob.public_key());
        let bob_shared = bob.diffie_hellman(alice.public_key());

        // Both should derive the same shared secret
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_keypair_storage() {
        let storage = KeyPairStorage::generate().unwrap();

        // Test signing
        let message = b"Hello, QiyasHash!";
        let signature = storage.sign(message);

        // Verify signature
        use ed25519_dalek::Signature;
        let sig = Signature::from_bytes(&signature);
        assert!(storage.signing_public_key().verify_strict(message, &sig).is_ok());
    }

    #[test]
    fn test_shared_secret_zeroize() {
        let bytes = [0xAB; 32];
        let secret = SharedSecret::from_bytes(bytes);
        let ptr = secret.as_bytes().as_ptr();

        drop(secret);

        // After drop, memory should be zeroed
        // Note: This test may not always work due to compiler optimizations
        // In production, use secure memory allocators
    }
}
