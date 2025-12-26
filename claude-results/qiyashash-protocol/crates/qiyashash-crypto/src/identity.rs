//! Identity key management for QiyasHash protocol
//!
//! Manages long-term identity keys, identity rotation, and verification.
//! Uses Ed25519 for signing and X25519 for key exchange.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};
use crate::keys::{PublicKeyBytes, SharedSecret};
use crate::kdf::{domain, KeyDerivationContext};

/// Identity key pair (Ed25519 for signing)
#[derive(ZeroizeOnDrop)]
pub struct IdentityKeyPair {
    /// The signing key
    #[zeroize(skip)]
    signing_key: SigningKey,
    /// The DH key for X3DH (derived from signing key)
    dh_secret: X25519StaticSecret,
}

impl IdentityKeyPair {
    /// Generate a new random identity key pair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        
        // Derive X25519 key from Ed25519 key for key exchange
        // This is a common pattern (e.g., used in libsodium's crypto_sign_ed25519_sk_to_curve25519)
        let dh_secret = Self::derive_x25519_from_ed25519(&signing_key);
        
        Self {
            signing_key,
            dh_secret,
        }
    }

    /// Create from existing secret bytes (32 bytes)
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let dh_secret = Self::derive_x25519_from_ed25519(&signing_key);
        
        Self {
            signing_key,
            dh_secret,
        }
    }

    /// Derive X25519 secret from Ed25519 signing key
    fn derive_x25519_from_ed25519(signing_key: &SigningKey) -> X25519StaticSecret {
        use sha2::{Sha512, Digest};
        
        // Hash the seed to get the expanded secret key
        let mut hasher = Sha512::new();
        hasher.update(signing_key.to_bytes());
        let hash = hasher.finalize();
        
        // Use first 32 bytes, clamped for X25519
        let mut x25519_bytes = [0u8; 32];
        x25519_bytes.copy_from_slice(&hash[..32]);
        
        // Clamp (X25519 requirement)
        x25519_bytes[0] &= 248;
        x25519_bytes[31] &= 127;
        x25519_bytes[31] |= 64;
        
        X25519StaticSecret::from(x25519_bytes)
    }

    /// Get the public identity key
    pub fn public_key(&self) -> IdentityPublicKey {
        IdentityPublicKey {
            signing_key: self.signing_key.verifying_key(),
            dh_key: X25519PublicKey::from(&self.dh_secret),
        }
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &X25519PublicKey) -> SharedSecret {
        let shared = self.dh_secret.diffie_hellman(their_public);
        SharedSecret(*shared.as_bytes())
    }

    /// Get the secret key bytes (for storage)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the DH public key
    pub fn dh_public_key(&self) -> X25519PublicKey {
        X25519PublicKey::from(&self.dh_secret)
    }
}

impl Clone for IdentityKeyPair {
    fn clone(&self) -> Self {
        Self::from_secret_bytes(&self.signing_key.to_bytes())
    }
}

/// Public identity key
#[derive(Clone, Debug)]
pub struct IdentityPublicKey {
    /// Ed25519 verifying key
    pub signing_key: VerifyingKey,
    /// X25519 public key for DH
    pub dh_key: X25519PublicKey,
}

impl IdentityPublicKey {
    /// Create from Ed25519 public key bytes
    pub fn from_bytes(ed25519_bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = VerifyingKey::from_bytes(ed25519_bytes)
            .map_err(|_| CryptoError::InvalidPublicKey("Invalid Ed25519 public key".to_string()))?;
        
        // Derive X25519 public key
        // Note: This is a simplification - in production you'd include both keys
        let dh_key = Self::ed25519_pk_to_x25519_pk(&signing_key)?;
        
        Ok(Self { signing_key, dh_key })
    }

    /// Convert Ed25519 public key to X25519
    fn ed25519_pk_to_x25519_pk(ed_pk: &VerifyingKey) -> Result<X25519PublicKey> {
        use curve25519_dalek::edwards::CompressedEdwardsY;
        use curve25519_dalek::montgomery::MontgomeryPoint;
        
        let compressed = CompressedEdwardsY::from_slice(ed_pk.as_bytes())
            .map_err(|_| CryptoError::InvalidPublicKey("Invalid compressed point".to_string()))?;
        
        let edwards = compressed.decompress()
            .ok_or_else(|| CryptoError::InvalidPublicKey("Could not decompress point".to_string()))?;
        
        let montgomery: MontgomeryPoint = edwards.to_montgomery();
        Ok(X25519PublicKey::from(montgomery.to_bytes()))
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<()> {
        let sig = Signature::from_bytes(signature);
        self.signing_key
            .verify(message, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Get Ed25519 public key bytes
    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get X25519 public key bytes
    pub fn dh_key_bytes(&self) -> [u8; 32] {
        *self.dh_key.as_bytes()
    }

    /// Serialize to bytes (Ed25519 + X25519 = 64 bytes)
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&self.signing_key_bytes());
        bytes[32..].copy_from_slice(&self.dh_key_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_full_bytes(bytes: &[u8; 64]) -> Result<Self> {
        let signing_key = VerifyingKey::from_bytes(&bytes[..32].try_into().unwrap())
            .map_err(|_| CryptoError::InvalidPublicKey("Invalid Ed25519 public key".to_string()))?;
        
        let dh_key = X25519PublicKey::from(<[u8; 32]>::try_from(&bytes[32..]).unwrap());
        
        Ok(Self { signing_key, dh_key })
    }
}

/// Serializable version of IdentityPublicKey
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableIdentityKey {
    /// Ed25519 public key
    #[serde(with = "hex::serde")]
    pub signing_key: [u8; 32],
    /// X25519 public key
    #[serde(with = "hex::serde")]
    pub dh_key: [u8; 32],
}

impl From<&IdentityPublicKey> for SerializableIdentityKey {
    fn from(key: &IdentityPublicKey) -> Self {
        Self {
            signing_key: key.signing_key_bytes(),
            dh_key: key.dh_key_bytes(),
        }
    }
}

impl TryFrom<SerializableIdentityKey> for IdentityPublicKey {
    type Error = CryptoError;

    fn try_from(value: SerializableIdentityKey) -> Result<Self> {
        let signing_key = VerifyingKey::from_bytes(&value.signing_key)
            .map_err(|_| CryptoError::InvalidPublicKey("Invalid Ed25519 public key".to_string()))?;
        let dh_key = X25519PublicKey::from(value.dh_key);
        Ok(Self { signing_key, dh_key })
    }
}

/// Full identity with key pair and metadata
pub struct Identity {
    /// The identity key pair
    pub key_pair: IdentityKeyPair,
    /// Identity creation timestamp
    pub created_at: i64,
    /// Identity fingerprint (hash of public key)
    pub fingerprint: [u8; 32],
}

impl Identity {
    /// Create a new identity
    pub fn new() -> Self {
        let key_pair = IdentityKeyPair::generate();
        let public_key = key_pair.public_key();
        let fingerprint = Self::compute_fingerprint(&public_key);
        
        Self {
            key_pair,
            created_at: chrono::Utc::now().timestamp(),
            fingerprint,
        }
    }

    /// Create from existing key pair
    pub fn from_key_pair(key_pair: IdentityKeyPair) -> Self {
        let public_key = key_pair.public_key();
        let fingerprint = Self::compute_fingerprint(&public_key);
        
        Self {
            key_pair,
            created_at: chrono::Utc::now().timestamp(),
            fingerprint,
        }
    }

    /// Compute fingerprint from public key
    fn compute_fingerprint(public_key: &IdentityPublicKey) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_bytes());
        let result = hasher.finalize();
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&result);
        fingerprint
    }

    /// Get the public identity
    pub fn public_key(&self) -> IdentityPublicKey {
        self.key_pair.public_key()
    }

    /// Get fingerprint as hex string
    pub fn fingerprint_hex(&self) -> String {
        hex::encode(self.fingerprint)
    }

    /// Rotate identity (create new key pair with proof of ownership)
    pub fn rotate(&self) -> (Identity, IdentityRotationProof) {
        let new_identity = Identity::new();
        
        // Create proof that owner of old key also owns new key
        let proof = self.create_rotation_proof(&new_identity);
        
        (new_identity, proof)
    }

    /// Create proof of identity rotation
    fn create_rotation_proof(&self, new_identity: &Identity) -> IdentityRotationProof {
        use sha2::{Sha256, Digest};
        
        let old_public = self.key_pair.public_key();
        let new_public = new_identity.key_pair.public_key();
        
        // Create message to sign: old_pub || new_pub || timestamp
        let timestamp = chrono::Utc::now().timestamp();
        let mut message = Vec::with_capacity(128 + 8);
        message.extend_from_slice(&old_public.to_bytes());
        message.extend_from_slice(&new_public.to_bytes());
        message.extend_from_slice(&timestamp.to_be_bytes());
        
        // Sign with old key
        let old_signature = self.key_pair.sign(&message);
        
        // Sign with new key
        let new_signature = new_identity.key_pair.sign(&message);
        
        // Create commitment
        let mut hasher = Sha256::new();
        hasher.update(&message);
        hasher.update(&old_signature);
        hasher.update(&new_signature);
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&hasher.finalize());
        
        IdentityRotationProof {
            old_public_key: SerializableIdentityKey::from(&old_public),
            new_public_key: SerializableIdentityKey::from(&new_public),
            old_signature,
            new_signature,
            timestamp,
            commitment,
        }
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self::new()
    }
}

/// Proof of identity rotation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityRotationProof {
    /// Old public key
    pub old_public_key: SerializableIdentityKey,
    /// New public key
    pub new_public_key: SerializableIdentityKey,
    /// Signature by old key
    #[serde(with = "hex::serde")]
    pub old_signature: [u8; 64],
    /// Signature by new key
    #[serde(with = "hex::serde")]
    pub new_signature: [u8; 64],
    /// Timestamp
    pub timestamp: i64,
    /// Commitment hash
    #[serde(with = "hex::serde")]
    pub commitment: [u8; 32],
}

impl IdentityRotationProof {
    /// Verify the rotation proof
    pub fn verify(&self) -> Result<()> {
        let old_public: IdentityPublicKey = self.old_public_key.clone().try_into()?;
        let new_public: IdentityPublicKey = self.new_public_key.clone().try_into()?;
        
        // Reconstruct message
        let mut message = Vec::with_capacity(128 + 8);
        message.extend_from_slice(&old_public.to_bytes());
        message.extend_from_slice(&new_public.to_bytes());
        message.extend_from_slice(&self.timestamp.to_be_bytes());
        
        // Verify old signature
        old_public.verify(&message, &self.old_signature)?;
        
        // Verify new signature
        new_public.verify(&message, &self.new_signature)?;
        
        // Verify commitment
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&message);
        hasher.update(&self.old_signature);
        hasher.update(&self.new_signature);
        let computed_commitment: [u8; 32] = hasher.finalize().into();
        
        if computed_commitment != self.commitment {
            return Err(CryptoError::IdentityVerificationFailed(
                "Commitment mismatch".to_string(),
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::new();
        assert!(!identity.fingerprint_hex().is_empty());
    }

    #[test]
    fn test_sign_verify() {
        let identity = Identity::new();
        let message = b"Hello, QiyasHash!";
        
        let signature = identity.key_pair.sign(message);
        let public_key = identity.key_pair.public_key();
        
        assert!(public_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let identity = Identity::new();
        let message = b"Hello, QiyasHash!";
        
        let mut signature = identity.key_pair.sign(message);
        signature[0] ^= 0xFF; // Tamper
        
        let public_key = identity.key_pair.public_key();
        assert!(public_key.verify(message, &signature).is_err());
    }

    #[test]
    fn test_identity_rotation() {
        let identity = Identity::new();
        let (new_identity, proof) = identity.rotate();
        
        // Proof should be valid
        assert!(proof.verify().is_ok());
        
        // New identity should have different fingerprint
        assert_ne!(identity.fingerprint, new_identity.fingerprint);
    }

    #[test]
    fn test_diffie_hellman() {
        let alice = Identity::new();
        let bob = Identity::new();
        
        let alice_shared = alice.key_pair.diffie_hellman(&bob.key_pair.dh_public_key());
        let bob_shared = bob.key_pair.diffie_hellman(&alice.key_pair.dh_public_key());
        
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_serialization() {
        let identity = Identity::new();
        let public_key = identity.key_pair.public_key();
        
        let serializable = SerializableIdentityKey::from(&public_key);
        let json = serde_json::to_string(&serializable).unwrap();
        
        let deserialized: SerializableIdentityKey = serde_json::from_str(&json).unwrap();
        let restored: IdentityPublicKey = deserialized.try_into().unwrap();
        
        assert_eq!(public_key.signing_key_bytes(), restored.signing_key_bytes());
    }
}
