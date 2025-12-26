//! Protocol client - main entry point for messaging
//!
//! The ProtocolClient provides a high-level interface for sending and
//! receiving encrypted messages.

use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn, error, instrument};

use qiyashash_core::message::{Message, MessageEnvelope, MessageId, RatchetHeaderWire};
use qiyashash_core::session::SessionId;
use qiyashash_core::storage::{Storage, MessageStore, SessionStore, IdentityStore, PreKeyStore};
use qiyashash_core::types::{DeviceId, Fingerprint, Timestamp, UserId};
use qiyashash_core::user::User;
use qiyashash_crypto::identity::Identity;
use qiyashash_crypto::chain::compute_message_hash;
use qiyashash_crypto::kdf::derive_chain_proof;

use crate::config::ClientConfig;
use crate::error::{ProtocolError, Result};
use crate::protocol::{
    DevicePreKeyBundle, PreKeyBundleRequest, PreKeyBundleResponse,
    ProtocolMessage, ProtocolMessageType,
};
use crate::session_manager::SessionManager;

/// Protocol client state
enum ClientState {
    /// Not initialized
    Uninitialized,
    /// Initializing
    Initializing,
    /// Ready for operations
    Ready,
    /// Shutting down
    ShuttingDown,
}

/// Protocol client for encrypted messaging
pub struct ProtocolClient<S: Storage> {
    /// Configuration
    config: ClientConfig,
    /// Our user ID
    user_id: UserId,
    /// Our device ID
    device_id: DeviceId,
    /// Session manager
    session_manager: RwLock<Option<SessionManager>>,
    /// Storage backend
    storage: Arc<S>,
    /// Client state
    state: RwLock<ClientState>,
}

impl<S: Storage + 'static> ProtocolClient<S> {
    /// Create a new protocol client
    pub fn new(config: ClientConfig, storage: Arc<S>) -> Self {
        Self {
            config,
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            session_manager: RwLock::new(None),
            storage,
            state: RwLock::new(ClientState::Uninitialized),
        }
    }

    /// Initialize the client with a new or existing identity
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        {
            let mut state = self.state.write();
            match *state {
                ClientState::Ready => return Err(ProtocolError::AlreadyInitialized),
                ClientState::Initializing => return Err(ProtocolError::AlreadyInitialized),
                _ => *state = ClientState::Initializing,
            }
        }

        info!("Initializing protocol client");

        // Try to load existing identity
        let identity = match self.load_identity().await? {
            Some(identity) => {
                info!("Loaded existing identity");
                identity
            }
            None => {
                info!("Creating new identity");
                let identity = Identity::new();
                self.save_identity(&identity).await?;
                identity
            }
        };

        // Create session manager
        let session_manager = SessionManager::new(
            self.config.clone(),
            identity,
            self.device_id.clone(),
            self.storage.clone(),
            self.storage.clone(),
            self.storage.clone(),
        ).await?;

        *self.session_manager.write() = Some(session_manager);
        *self.state.write() = ClientState::Ready;

        info!("Protocol client initialized");
        Ok(())
    }

    /// Check if client is ready
    pub fn is_ready(&self) -> bool {
        matches!(*self.state.read(), ClientState::Ready)
    }

    /// Get our user ID
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    /// Get our device ID
    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    /// Get our identity fingerprint
    pub fn fingerprint(&self) -> Result<Fingerprint> {
        self.with_session_manager(|sm| Ok(sm.fingerprint()))
    }

    /// Get our prekey bundle for publishing
    pub fn get_prekey_bundle(&self) -> Result<qiyashash_crypto::keys::PreKeyBundle> {
        self.with_session_manager(|sm| Ok(sm.get_prekey_bundle()))
    }

    /// Send a text message to a user
    #[instrument(skip(self, content))]
    pub async fn send_message(
        &self,
        recipient_id: &UserId,
        recipient_device_id: &DeviceId,
        content: &str,
    ) -> Result<MessageEnvelope> {
        self.ensure_ready()?;

        // Create message
        let message = Message::text(
            self.user_id.clone(),
            self.device_id.clone(),
            recipient_id.clone(),
            content,
        );

        // Encrypt and send
        self.encrypt_message(recipient_id, recipient_device_id, &message).await
    }

    /// Encrypt a message for a recipient
    #[instrument(skip(self, message))]
    pub async fn encrypt_message(
        &self,
        recipient_id: &UserId,
        recipient_device_id: &DeviceId,
        message: &Message,
    ) -> Result<MessageEnvelope> {
        self.ensure_ready()?;

        // Check for existing session
        let session_id = self.with_session_manager(|sm| {
            Ok(sm.get_session(recipient_id, recipient_device_id))
        })?;

        let session_id = match session_id {
            Some(id) => id,
            None => {
                // Need to establish session first
                return Err(ProtocolError::SessionNotEstablished(recipient_id.to_string()));
            }
        };

        // Serialize message
        let plaintext = message.to_bytes()
            .map_err(|e| ProtocolError::Internal(e.to_string()))?;

        // Encrypt
        let (ciphertext, chain_state, msg_hash) = self.with_session_manager(|sm| {
            sm.encrypt(&session_id, &plaintext)
        })?;

        // Create timestamp hash
        let timestamp = Timestamp::now();
        let timestamp_hash = self.compute_timestamp_hash(timestamp);

        // Create chain proof
        let chain_proof = derive_chain_proof(&chain_state, &msg_hash, timestamp.as_millis() as u64);

        // Get our identity key
        let identity_key = self.with_session_manager(|sm| {
            Ok(sm.identity_public_key().signing_key_bytes())
        })?;

        // Get ratchet public key
        let ratchet_public = self.with_session_manager(|sm| {
            sm.encrypt(&session_id, &[]) // Dummy call to get current key
                .map(|(_, _, _)| [0u8; 32]) // Placeholder
                .or_else(|_| Ok([0u8; 32]))
        })?;

        // Create envelope
        let envelope = MessageEnvelope {
            version: crate::PROTOCOL_VERSION,
            sender_identity_key: identity_key,
            ephemeral_key: None, // Only for initial message
            one_time_prekey_id: None,
            ratchet_header: RatchetHeaderWire {
                dh_public: ratchet_public,
                message_number: 0, // Would come from ratchet
                previous_chain_length: 0,
            },
            ciphertext,
            chain_proof,
            timestamp_hash,
        };

        // Save message to storage
        self.storage.save_message(message).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        debug!("Encrypted message {} for {}", message.id, recipient_id);
        Ok(envelope)
    }

    /// Decrypt a received message
    #[instrument(skip(self, envelope))]
    pub async fn decrypt_message(
        &self,
        sender_id: &UserId,
        sender_device_id: &DeviceId,
        envelope: &MessageEnvelope,
    ) -> Result<Message> {
        self.ensure_ready()?;

        // Verify protocol version
        if envelope.version != crate::PROTOCOL_VERSION {
            return Err(ProtocolError::VersionMismatch {
                expected: crate::PROTOCOL_VERSION,
                actual: envelope.version,
            });
        }

        // Check for session
        let session_id = self.with_session_manager(|sm| {
            Ok(sm.get_session(sender_id, sender_device_id))
        })?;

        let session_id = match session_id {
            Some(id) => id,
            None => {
                // Check if this is an initial message (has ephemeral key)
                if let Some(ephemeral_key) = envelope.ephemeral_key {
                    // Accept new session
                    self.with_session_manager_mut(|sm| {
                        // Need async here - this is a simplification
                        Err(ProtocolError::SessionNotEstablished(sender_id.to_string()))
                    })?
                } else {
                    return Err(ProtocolError::SessionNotFound(sender_id.to_string()));
                }
            }
        };

        // Decrypt
        let plaintext = self.with_session_manager(|sm| {
            sm.decrypt(&session_id, &envelope.ciphertext)
        })?;

        // Deserialize message
        let message = Message::from_bytes(&plaintext)
            .map_err(|e| ProtocolError::InvalidMessage(e.to_string()))?;

        // Save to storage
        self.storage.save_message(&message).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        debug!("Decrypted message {} from {}", message.id, sender_id);
        Ok(message)
    }

    /// Establish a session with a user using their prekey bundle
    #[instrument(skip(self, bundle))]
    pub async fn establish_session(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        bundle: &DevicePreKeyBundle,
    ) -> Result<SessionId> {
        self.ensure_ready()?;

        self.with_session_manager_mut(|sm| {
            // This is async but we're in sync context - simplified
            Err(ProtocolError::Internal("Need async context".to_string()))
        })
    }

    /// Process an incoming protocol message
    #[instrument(skip(self, message))]
    pub async fn process_message(&self, message: ProtocolMessage) -> Result<Option<ProtocolMessage>> {
        self.ensure_ready()?;

        match message.message_type {
            ProtocolMessageType::EncryptedMessage(envelope) => {
                let decrypted = self.decrypt_message(
                    &message.sender_id,
                    &message.sender_device_id,
                    &envelope,
                ).await?;
                
                // Return delivery receipt
                // ...
                Ok(None)
            }
            ProtocolMessageType::PreKeyBundleRequest(request) => {
                // Handle prekey request
                let bundle = self.get_prekey_bundle()?;
                // Convert and return response
                Ok(None)
            }
            ProtocolMessageType::DeliveryReceipt(receipt) => {
                // Update message status
                Ok(None)
            }
            ProtocolMessageType::ReadReceipt(receipt) => {
                // Update message status
                Ok(None)
            }
            ProtocolMessageType::SessionReset(reset) => {
                // Handle session reset
                Ok(None)
            }
            _ => {
                debug!("Unhandled message type");
                Ok(None)
            }
        }
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<()> {
        *self.state.write() = ClientState::ShuttingDown;
        
        // Flush storage
        self.storage.flush().await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        info!("Protocol client shutdown");
        Ok(())
    }

    // Helper methods

    fn ensure_ready(&self) -> Result<()> {
        if !self.is_ready() {
            return Err(ProtocolError::NotInitialized);
        }
        Ok(())
    }

    fn with_session_manager<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&SessionManager) -> Result<T>,
    {
        let guard = self.session_manager.read();
        let sm = guard.as_ref().ok_or(ProtocolError::NotInitialized)?;
        f(sm)
    }

    fn with_session_manager_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut SessionManager) -> Result<T>,
    {
        let mut guard = self.session_manager.write();
        let sm = guard.as_mut().ok_or(ProtocolError::NotInitialized)?;
        f(sm)
    }

    async fn load_identity(&self) -> Result<Option<Identity>> {
        let encrypted = self.storage.get_identity_key().await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        match encrypted {
            Some(data) => {
                // In production, decrypt with user password
                // For now, just deserialize
                let key_bytes: [u8; 32] = bincode::deserialize(&data)
                    .map_err(|e| ProtocolError::Internal(e.to_string()))?;
                
                let key_pair = qiyashash_crypto::identity::IdentityKeyPair::from_secret_bytes(&key_bytes);
                Ok(Some(Identity::from_key_pair(key_pair)))
            }
            None => Ok(None),
        }
    }

    async fn save_identity(&self, identity: &Identity) -> Result<()> {
        let key_bytes = identity.key_pair.secret_bytes();
        let encrypted = bincode::serialize(&key_bytes)
            .map_err(|e| ProtocolError::Internal(e.to_string()))?;

        self.storage.save_identity_key(encrypted).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        Ok(())
    }

    fn compute_timestamp_hash(&self, timestamp: Timestamp) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"QiyasHash_Timestamp_v1");
        hasher.update(&timestamp.as_millis().to_be_bytes());
        // Add random noise for metadata protection
        let noise: [u8; 16] = rand::random();
        hasher.update(&noise);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiyashash_core::storage::memory::MemoryStorage;

    #[tokio::test]
    async fn test_client_initialization() {
        let storage = MemoryStorage::new();
        let config = ClientConfig::default();
        let client = ProtocolClient::new(config, storage);

        assert!(!client.is_ready());
        
        client.initialize().await.unwrap();
        
        assert!(client.is_ready());
    }

    #[tokio::test]
    async fn test_double_initialization() {
        let storage = MemoryStorage::new();
        let config = ClientConfig::default();
        let client = ProtocolClient::new(config, storage);

        client.initialize().await.unwrap();
        
        let result = client.initialize().await;
        assert!(matches!(result, Err(ProtocolError::AlreadyInitialized)));
    }
}
