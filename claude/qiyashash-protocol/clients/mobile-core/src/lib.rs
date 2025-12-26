//! QiyasHash Mobile Core Library
//! 
//! Cross-platform mobile library for iOS and Android using UniFFI bindings.
//! Provides a simple, safe interface to the QiyasHash E2E encryption protocol.

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

mod crypto;
mod identity;
mod messaging;
mod storage;

pub use crypto::*;
pub use identity::*;
pub use messaging::*;
pub use storage::*;

/// Mobile-specific error types
#[derive(Error, Debug)]
pub enum MobileError {
    #[error("Initialization error: {0}")]
    InitError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type for mobile operations
pub type MobileResult<T> = Result<T, MobileError>;

/// QiyasHash Mobile Client
/// 
/// Main entry point for mobile applications.
/// Thread-safe and designed for FFI.
pub struct QiyasHashClient {
    inner: Arc<RwLock<ClientInner>>,
}

struct ClientInner {
    identity: Option<UserIdentity>,
    storage: Option<SecureStorage>,
    initialized: bool,
}

impl QiyasHashClient {
    /// Create a new QiyasHash client
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ClientInner {
                identity: None,
                storage: None,
                initialized: false,
            })),
        }
    }

    /// Initialize the client with a storage path
    pub async fn initialize(&self, storage_path: String) -> MobileResult<()> {
        let mut inner = self.inner.write().await;
        
        let storage = SecureStorage::new(&storage_path)
            .map_err(|e| MobileError::StorageError(e.to_string()))?;
        
        inner.storage = Some(storage);
        inner.initialized = true;
        
        Ok(())
    }

    /// Check if client is initialized
    pub async fn is_initialized(&self) -> bool {
        self.inner.read().await.initialized
    }

    /// Create or load identity
    pub async fn create_identity(&self, display_name: String) -> MobileResult<String> {
        let mut inner = self.inner.write().await;
        
        if !inner.initialized {
            return Err(MobileError::NotInitialized);
        }

        let identity = UserIdentity::generate(display_name)
            .map_err(|e| MobileError::CryptoError(e.to_string()))?;
        
        let identity_id = identity.id.clone();
        
        // Save to storage
        if let Some(ref storage) = inner.storage {
            storage.save_identity(&identity)
                .map_err(|e| MobileError::StorageError(e.to_string()))?;
        }
        
        inner.identity = Some(identity);
        
        Ok(identity_id)
    }

    /// Load existing identity
    pub async fn load_identity(&self) -> MobileResult<Option<String>> {
        let mut inner = self.inner.write().await;
        
        if !inner.initialized {
            return Err(MobileError::NotInitialized);
        }

        if let Some(ref storage) = inner.storage {
            if let Some(identity) = storage.load_identity()
                .map_err(|e| MobileError::StorageError(e.to_string()))? 
            {
                let id = identity.id.clone();
                inner.identity = Some(identity);
                return Ok(Some(id));
            }
        }
        
        Ok(None)
    }

    /// Get current identity ID
    pub async fn get_identity_id(&self) -> MobileResult<Option<String>> {
        let inner = self.inner.read().await;
        Ok(inner.identity.as_ref().map(|i| i.id.clone()))
    }

    /// Get identity public key (for sharing)
    pub async fn get_public_key(&self) -> MobileResult<String> {
        let inner = self.inner.read().await;
        
        let identity = inner.identity.as_ref()
            .ok_or(MobileError::NotInitialized)?;
        
        Ok(identity.public_key_base64())
    }

    /// Encrypt a message for a recipient
    pub async fn encrypt_message(
        &self,
        recipient_public_key: String,
        plaintext: String,
    ) -> MobileResult<String> {
        let inner = self.inner.read().await;
        
        let identity = inner.identity.as_ref()
            .ok_or(MobileError::NotInitialized)?;
        
        let ciphertext = identity.encrypt_for(&recipient_public_key, plaintext.as_bytes())
            .map_err(|e| MobileError::CryptoError(e.to_string()))?;
        
        Ok(base64::encode(&ciphertext))
    }

    /// Decrypt a received message
    pub async fn decrypt_message(
        &self,
        sender_public_key: String,
        ciphertext: String,
    ) -> MobileResult<String> {
        let inner = self.inner.read().await;
        
        let identity = inner.identity.as_ref()
            .ok_or(MobileError::NotInitialized)?;
        
        let ciphertext_bytes = base64::decode(&ciphertext)
            .map_err(|e| MobileError::InvalidInput(e.to_string()))?;
        
        let plaintext = identity.decrypt_from(&sender_public_key, &ciphertext_bytes)
            .map_err(|e| MobileError::CryptoError(e.to_string()))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| MobileError::InvalidInput(e.to_string()))
    }

    /// Generate a random session key
    pub fn generate_session_key(&self) -> MobileResult<String> {
        let key = MobileCrypto::generate_session_key()
            .map_err(|e| MobileError::CryptoError(e.to_string()))?;
        Ok(base64::encode(&key))
    }

    /// Delete all local data
    pub async fn wipe_data(&self) -> MobileResult<()> {
        let mut inner = self.inner.write().await;
        
        if let Some(ref storage) = inner.storage {
            storage.wipe_all()
                .map_err(|e| MobileError::StorageError(e.to_string()))?;
        }
        
        inner.identity = None;
        
        Ok(())
    }
}

impl Default for QiyasHashClient {
    fn default() -> Self {
        Self::new()
    }
}

// UniFFI bindings
uniffi::include_scaffolding!("qiyashash_mobile");

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_initialization() {
        let client = QiyasHashClient::new();
        assert!(!client.is_initialized().await);
        
        let temp_dir = tempfile::TempDir::new().unwrap();
        client.initialize(temp_dir.path().to_string_lossy().to_string()).await.unwrap();
        
        assert!(client.is_initialized().await);
    }
}
