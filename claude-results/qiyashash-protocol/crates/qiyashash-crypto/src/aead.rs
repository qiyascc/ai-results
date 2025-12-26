//! Authenticated Encryption with Associated Data (AEAD)
//!
//! Provides both ChaCha20-Poly1305 and AES-256-GCM for message encryption.
//! ChaCha20-Poly1305 is preferred for software implementations while
//! AES-256-GCM may be faster on hardware with AES-NI support.

use aes_gcm::{
    aead::{Aead as AeadTrait, KeyInit},
    Aes256Gcm,
};
use chacha20poly1305::XChaCha20Poly1305;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};
use crate::MAX_MESSAGE_SIZE;

/// Nonce size for XChaCha20-Poly1305 (192 bits)
pub const XCHACHA_NONCE_SIZE: usize = 24;

/// Nonce size for AES-256-GCM (96 bits)
pub const AES_GCM_NONCE_SIZE: usize = 12;

/// Authentication tag size (128 bits)
pub const TAG_SIZE: usize = 16;

/// Key size for both algorithms (256 bits)
pub const KEY_SIZE: usize = 32;

/// AEAD key with automatic zeroization
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct AeadKey(pub [u8; KEY_SIZE]);

impl AeadKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; KEY_SIZE]) -> Self {
        Self(bytes)
    }

    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.0
    }
}

impl AsRef<[u8]> for AeadKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Nonce (number used once) for AEAD
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Nonce {
    /// XChaCha20-Poly1305 nonce (24 bytes)
    XChaCha([u8; XCHACHA_NONCE_SIZE]),
    /// AES-GCM nonce (12 bytes)
    AesGcm([u8; AES_GCM_NONCE_SIZE]),
}

impl Nonce {
    /// Generate a random XChaCha20 nonce
    pub fn random_xchacha() -> Self {
        let mut nonce = [0u8; XCHACHA_NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce);
        Self::XChaCha(nonce)
    }

    /// Generate a random AES-GCM nonce
    pub fn random_aes_gcm() -> Self {
        let mut nonce = [0u8; AES_GCM_NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce);
        Self::AesGcm(nonce)
    }

    /// Get nonce bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Nonce::XChaCha(n) => n,
            Nonce::AesGcm(n) => n,
        }
    }
}

/// AEAD algorithm selection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AeadAlgorithm {
    /// XChaCha20-Poly1305 (default, better for software)
    XChaCha20Poly1305,
    /// AES-256-GCM (faster with hardware support)
    Aes256Gcm,
}

impl Default for AeadAlgorithm {
    fn default() -> Self {
        Self::XChaCha20Poly1305
    }
}

/// Encrypted payload with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// The algorithm used
    pub algorithm: AeadAlgorithm,
    /// The nonce
    pub nonce: Nonce,
    /// The ciphertext with authentication tag
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}

impl EncryptedPayload {
    /// Get the plaintext length (without tag)
    pub fn plaintext_len(&self) -> usize {
        self.ciphertext.len().saturating_sub(TAG_SIZE)
    }
}

/// AEAD cipher for message encryption
pub struct Aead {
    algorithm: AeadAlgorithm,
}

impl Aead {
    /// Create a new AEAD cipher with the default algorithm (XChaCha20-Poly1305)
    pub fn new() -> Self {
        Self {
            algorithm: AeadAlgorithm::default(),
        }
    }

    /// Create with a specific algorithm
    pub fn with_algorithm(algorithm: AeadAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Encrypt plaintext with associated data
    ///
    /// # Arguments
    /// * `key` - The encryption key
    /// * `plaintext` - The message to encrypt
    /// * `aad` - Associated data (authenticated but not encrypted)
    ///
    /// # Returns
    /// Encrypted payload containing algorithm, nonce, and ciphertext
    pub fn encrypt(&self, key: &AeadKey, plaintext: &[u8], aad: &[u8]) -> Result<EncryptedPayload> {
        if plaintext.len() > MAX_MESSAGE_SIZE {
            return Err(CryptoError::MessageTooLarge {
                size: plaintext.len(),
                max: MAX_MESSAGE_SIZE,
            });
        }

        match self.algorithm {
            AeadAlgorithm::XChaCha20Poly1305 => self.encrypt_xchacha(key, plaintext, aad),
            AeadAlgorithm::Aes256Gcm => self.encrypt_aes_gcm(key, plaintext, aad),
        }
    }

    /// Decrypt ciphertext with associated data
    ///
    /// # Arguments
    /// * `key` - The decryption key
    /// * `payload` - The encrypted payload
    /// * `aad` - Associated data (must match what was used during encryption)
    ///
    /// # Returns
    /// Decrypted plaintext
    pub fn decrypt(
        &self,
        key: &AeadKey,
        payload: &EncryptedPayload,
        aad: &[u8],
    ) -> Result<Vec<u8>> {
        match payload.algorithm {
            AeadAlgorithm::XChaCha20Poly1305 => self.decrypt_xchacha(key, payload, aad),
            AeadAlgorithm::Aes256Gcm => self.decrypt_aes_gcm(key, payload, aad),
        }
    }

    fn encrypt_xchacha(
        &self,
        key: &AeadKey,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<EncryptedPayload> {
        use chacha20poly1305::aead::Payload;

        let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
        let nonce = Nonce::random_xchacha();

        let nonce_bytes = match &nonce {
            Nonce::XChaCha(n) => n,
            _ => unreachable!(),
        };

        let ciphertext = cipher
            .encrypt(
                nonce_bytes.into(),
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed("XChaCha20-Poly1305 failed".to_string()))?;

        Ok(EncryptedPayload {
            algorithm: AeadAlgorithm::XChaCha20Poly1305,
            nonce,
            ciphertext,
        })
    }

    fn decrypt_xchacha(
        &self,
        key: &AeadKey,
        payload: &EncryptedPayload,
        aad: &[u8],
    ) -> Result<Vec<u8>> {
        use chacha20poly1305::aead::Payload;

        let cipher = XChaCha20Poly1305::new(key.as_bytes().into());

        let nonce_bytes = match &payload.nonce {
            Nonce::XChaCha(n) => n,
            _ => return Err(CryptoError::DecryptionFailed("Wrong nonce type".to_string())),
        };

        cipher
            .decrypt(
                nonce_bytes.into(),
                Payload {
                    msg: &payload.ciphertext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::AuthenticationFailed)
    }

    fn encrypt_aes_gcm(
        &self,
        key: &AeadKey,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<EncryptedPayload> {
        use aes_gcm::aead::Payload;

        let cipher = Aes256Gcm::new(key.as_bytes().into());
        let nonce = Nonce::random_aes_gcm();

        let nonce_bytes = match &nonce {
            Nonce::AesGcm(n) => n,
            _ => unreachable!(),
        };

        let ciphertext = cipher
            .encrypt(
                nonce_bytes.into(),
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed("AES-256-GCM failed".to_string()))?;

        Ok(EncryptedPayload {
            algorithm: AeadAlgorithm::Aes256Gcm,
            nonce,
            ciphertext,
        })
    }

    fn decrypt_aes_gcm(
        &self,
        key: &AeadKey,
        payload: &EncryptedPayload,
        aad: &[u8],
    ) -> Result<Vec<u8>> {
        use aes_gcm::aead::Payload;

        let cipher = Aes256Gcm::new(key.as_bytes().into());

        let nonce_bytes = match &payload.nonce {
            Nonce::AesGcm(n) => n,
            _ => return Err(CryptoError::DecryptionFailed("Wrong nonce type".to_string())),
        };

        cipher
            .decrypt(
                nonce_bytes.into(),
                Payload {
                    msg: &payload.ciphertext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::AuthenticationFailed)
    }
}

impl Default for Aead {
    fn default() -> Self {
        Self::new()
    }
}

/// Encrypt-then-MAC construction for header encryption
///
/// Used when we need deterministic encryption for headers
pub struct HeaderCipher {
    cipher: Aead,
}

impl HeaderCipher {
    /// Create a new header cipher
    pub fn new() -> Self {
        Self {
            cipher: Aead::new(),
        }
    }

    /// Encrypt a header
    pub fn encrypt(&self, key: &AeadKey, header: &[u8]) -> Result<EncryptedPayload> {
        // Use empty AAD for headers since the header itself is the data
        self.cipher.encrypt(key, header, &[])
    }

    /// Decrypt a header
    pub fn decrypt(&self, key: &AeadKey, payload: &EncryptedPayload) -> Result<Vec<u8>> {
        self.cipher.decrypt(key, payload, &[])
    }
}

impl Default for HeaderCipher {
    fn default() -> Self {
        Self::new()
    }
}

// Serde helper for Vec<u8>
mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: &[u8] = Deserialize::deserialize(deserializer)?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xchacha20_roundtrip() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = b"Hello, QiyasHash!";
        let aad = b"associated data";

        let encrypted = cipher.encrypt(&key, plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_aes_gcm_roundtrip() {
        let cipher = Aead::with_algorithm(AeadAlgorithm::Aes256Gcm);
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = b"Hello, QiyasHash!";
        let aad = b"associated data";

        let encrypted = cipher.encrypt(&key, plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails() {
        let cipher = Aead::new();
        let key1 = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let key2 = AeadKey::from_bytes([0x43; KEY_SIZE]);
        let plaintext = b"Secret message";
        let aad = b"aad";

        let encrypted = cipher.encrypt(&key1, plaintext, aad).unwrap();
        let result = cipher.decrypt(&key2, &encrypted, aad);

        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_aad_fails() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = b"Secret message";

        let encrypted = cipher.encrypt(&key, plaintext, b"aad1").unwrap();
        let result = cipher.decrypt(&key, &encrypted, b"aad2");

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = b"Secret message";
        let aad = b"aad";

        let mut encrypted = cipher.encrypt(&key, plaintext, aad).unwrap();
        encrypted.ciphertext[0] ^= 0xFF; // Tamper with ciphertext

        let result = cipher.decrypt(&key, &encrypted, aad);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = b"";
        let aad = b"aad";

        let encrypted = cipher.encrypt(&key, plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_large_message() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = vec![0x42u8; 10000];
        let aad = b"aad";

        let encrypted = cipher.encrypt(&key, &plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_message_too_large() {
        let cipher = Aead::new();
        let key = AeadKey::from_bytes([0x42; KEY_SIZE]);
        let plaintext = vec![0x42u8; MAX_MESSAGE_SIZE + 1];

        let result = cipher.encrypt(&key, &plaintext, b"");
        assert!(matches!(result, Err(CryptoError::MessageTooLarge { .. })));
    }
}
