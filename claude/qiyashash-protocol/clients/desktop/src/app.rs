//! Desktop application core

use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug};

use qiyashash_core::message::Message;
use qiyashash_core::session::SessionId;
use qiyashash_core::types::{DeviceId, UserId};
use qiyashash_crypto::identity::Identity;

use crate::state::AppState;
use crate::storage::DesktopStorage;

/// Desktop application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Not initialized
    #[error("App not initialized")]
    NotInitialized,

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, AppError>;

/// Desktop application
pub struct App {
    /// Application state
    state: Arc<RwLock<AppState>>,
    /// Storage
    storage: Arc<DesktopStorage>,
    /// Our identity
    identity: Option<Identity>,
    /// Our device ID
    device_id: DeviceId,
}

impl App {
    /// Create a new application instance
    pub fn new(data_dir: &str) -> Result<Self> {
        info!("Initializing QiyasHash Desktop");

        let storage = DesktopStorage::open(data_dir)
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(Self {
            state: Arc::new(RwLock::new(AppState::new())),
            storage: Arc::new(storage),
            identity: None,
            device_id: DeviceId::new(),
        })
    }

    /// Initialize with existing or new identity
    pub async fn initialize(&mut self) -> Result<()> {
        // Try to load existing identity
        if let Some(identity_data) = self.storage.load_identity()? {
            let key_pair = qiyashash_crypto::identity::IdentityKeyPair::from_secret_bytes(
                &identity_data
            );
            self.identity = Some(Identity::from_key_pair(key_pair));
            info!("Loaded existing identity");
        } else {
            // Create new identity
            let identity = Identity::new();
            self.storage.save_identity(&identity.key_pair.secret_bytes())?;
            self.identity = Some(identity);
            info!("Created new identity");
        }

        // Update state
        if let Some(ref identity) = self.identity {
            let mut state = self.state.write();
            state.user_id = Some(UserId::from_fingerprint(&identity.fingerprint));
            state.device_id = Some(self.device_id.clone());
            state.initialized = true;
        }

        info!("Desktop app initialized");
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.state.read().initialized
    }

    /// Get our user ID
    pub fn user_id(&self) -> Option<UserId> {
        self.state.read().user_id.clone()
    }

    /// Get our identity fingerprint
    pub fn fingerprint(&self) -> Option<String> {
        self.identity.as_ref().map(|i| i.fingerprint_hex())
    }

    /// Send a message
    pub async fn send_message(
        &self,
        recipient_id: &UserId,
        content: &str,
    ) -> Result<Message> {
        if !self.is_initialized() {
            return Err(AppError::NotInitialized);
        }

        let user_id = self.user_id().ok_or(AppError::NotInitialized)?;
        
        let message = Message::text(
            user_id,
            self.device_id.clone(),
            recipient_id.clone(),
            content,
        );

        // In full implementation:
        // 1. Get or establish session
        // 2. Encrypt message
        // 3. Send via relay/DHT
        // 4. Store locally

        self.storage.save_message(&message)?;

        debug!("Sent message {} to {}", message.id, recipient_id);
        Ok(message)
    }

    /// Get conversation messages
    pub fn get_conversation(
        &self,
        with_user: &UserId,
        limit: usize,
    ) -> Result<Vec<Message>> {
        self.storage.get_messages(with_user, limit)
    }

    /// Get all conversations
    pub fn get_conversations(&self) -> Result<Vec<ConversationInfo>> {
        self.storage.get_conversations()
    }

    /// Mark messages as read
    pub fn mark_as_read(&self, with_user: &UserId) -> Result<()> {
        // In full implementation, also send read receipts
        Ok(())
    }

    /// Get unread count for a conversation
    pub fn unread_count(&self, with_user: &UserId) -> Result<usize> {
        self.storage.get_unread_count(with_user)
    }

    /// Delete a message
    pub fn delete_message(&self, message_id: &str, for_everyone: bool) -> Result<()> {
        // In full implementation:
        // 1. Update chain state
        // 2. Send deletion notice if for_everyone
        // 3. Delete locally

        self.storage.delete_message(message_id)?;
        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> AppState {
        self.state.read().clone()
    }

    /// Shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down desktop app");
        
        let mut state = self.state.write();
        state.initialized = false;
        
        Ok(())
    }
}

/// Conversation summary
#[derive(Clone, Debug, serde::Serialize)]
pub struct ConversationInfo {
    /// Other user's ID
    pub user_id: UserId,
    /// Other user's name (if known)
    pub display_name: Option<String>,
    /// Last message preview
    pub last_message: Option<String>,
    /// Last message timestamp
    pub last_message_at: Option<i64>,
    /// Unread count
    pub unread_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_app_initialization() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path().to_str().unwrap()).unwrap();

        assert!(!app.is_initialized());
        
        app.initialize().await.unwrap();
        
        assert!(app.is_initialized());
        assert!(app.fingerprint().is_some());
    }
}
