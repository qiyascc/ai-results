//! Desktop storage using sled

use std::path::Path;
use tracing::{debug, info};

use qiyashash_core::message::Message;
use qiyashash_core::types::UserId;

use crate::app::{AppError, ConversationInfo, Result};

/// Desktop storage
pub struct DesktopStorage {
    db: sled::Db,
}

impl DesktopStorage {
    /// Open storage at path
    pub fn open(path: &str) -> Result<Self> {
        let db_path = Path::new(path).join("qiyashash.db");
        
        let db = sled::open(&db_path)
            .map_err(|e| AppError::Storage(e.to_string()))?;

        info!("Opened storage at {:?}", db_path);

        Ok(Self { db })
    }

    /// Save identity
    pub fn save_identity(&self, secret: &[u8; 32]) -> Result<()> {
        self.db.insert("identity", secret.as_slice())
            .map_err(|e| AppError::Storage(e.to_string()))?;
        self.db.flush()
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Load identity
    pub fn load_identity(&self) -> Result<Option<[u8; 32]>> {
        match self.db.get("identity")
            .map_err(|e| AppError::Storage(e.to_string()))? {
            Some(bytes) => {
                if bytes.len() != 32 {
                    return Err(AppError::Storage("Invalid identity data".to_string()));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }

    /// Save message
    pub fn save_message(&self, message: &Message) -> Result<()> {
        let key = format!("msg:{}", message.id);
        let data = bincode::serialize(message)
            .map_err(|e| AppError::Storage(e.to_string()))?;

        self.db.insert(key.as_bytes(), data)
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Update conversation index
        let conv_key = self.conversation_key(&message.sender_id, &message.recipient_id);
        let msg_key = format!("{}:{}", message.created_at.as_millis(), message.id);
        
        let conv_tree = self.db.open_tree(&conv_key)
            .map_err(|e| AppError::Storage(e.to_string()))?;
        
        conv_tree.insert(msg_key.as_bytes(), message.id.as_str().as_bytes())
            .map_err(|e| AppError::Storage(e.to_string()))?;

        self.db.flush()
            .map_err(|e| AppError::Storage(e.to_string()))?;

        debug!("Saved message {}", message.id);
        Ok(())
    }

    /// Get message by ID
    pub fn get_message(&self, message_id: &str) -> Result<Option<Message>> {
        let key = format!("msg:{}", message_id);
        
        match self.db.get(key.as_bytes())
            .map_err(|e| AppError::Storage(e.to_string()))? {
            Some(data) => {
                let message: Message = bincode::deserialize(&data)
                    .map_err(|e| AppError::Storage(e.to_string()))?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    /// Get messages for conversation
    pub fn get_messages(&self, with_user: &UserId, limit: usize) -> Result<Vec<Message>> {
        // For simplicity, scan all messages
        // In production, use conversation index
        let mut messages = Vec::new();
        let prefix = b"msg:";

        for result in self.db.scan_prefix(prefix) {
            let (_, value) = result
                .map_err(|e| AppError::Storage(e.to_string()))?;
            
            let message: Message = bincode::deserialize(&value)
                .map_err(|e| AppError::Storage(e.to_string()))?;

            if message.sender_id == *with_user || message.recipient_id == *with_user {
                messages.push(message);
            }

            if messages.len() >= limit {
                break;
            }
        }

        // Sort by timestamp (newest first)
        messages.sort_by(|a, b| b.created_at.as_millis().cmp(&a.created_at.as_millis()));
        messages.truncate(limit);

        Ok(messages)
    }

    /// Delete message
    pub fn delete_message(&self, message_id: &str) -> Result<()> {
        let key = format!("msg:{}", message_id);
        self.db.remove(key.as_bytes())
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get conversations list
    pub fn get_conversations(&self) -> Result<Vec<ConversationInfo>> {
        // In production, maintain a conversations index
        // For now, return empty
        Ok(Vec::new())
    }

    /// Get unread count
    pub fn get_unread_count(&self, _with_user: &UserId) -> Result<usize> {
        // In production, maintain read state
        Ok(0)
    }

    /// Save settings
    pub fn save_settings(&self, settings: &crate::state::AppSettings) -> Result<()> {
        let data = serde_json::to_vec(settings)
            .map_err(|e| AppError::Storage(e.to_string()))?;
        
        self.db.insert("settings", data)
            .map_err(|e| AppError::Storage(e.to_string()))?;
        
        Ok(())
    }

    /// Load settings
    pub fn load_settings(&self) -> Result<Option<crate::state::AppSettings>> {
        match self.db.get("settings")
            .map_err(|e| AppError::Storage(e.to_string()))? {
            Some(data) => {
                let settings = serde_json::from_slice(&data)
                    .map_err(|e| AppError::Storage(e.to_string()))?;
                Ok(Some(settings))
            }
            None => Ok(None),
        }
    }

    // Helper to create consistent conversation key
    fn conversation_key(&self, user1: &UserId, user2: &UserId) -> String {
        let mut users = [user1.as_str(), user2.as_str()];
        users.sort();
        format!("conv:{}:{}", users[0], users[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use qiyashash_core::types::DeviceId;

    #[test]
    fn test_identity_storage() {
        let dir = tempdir().unwrap();
        let storage = DesktopStorage::open(dir.path().to_str().unwrap()).unwrap();

        let secret = [0x42u8; 32];
        storage.save_identity(&secret).unwrap();

        let loaded = storage.load_identity().unwrap().unwrap();
        assert_eq!(secret, loaded);
    }

    #[test]
    fn test_message_storage() {
        let dir = tempdir().unwrap();
        let storage = DesktopStorage::open(dir.path().to_str().unwrap()).unwrap();

        let message = Message::text(
            UserId::from_string("alice"),
            DeviceId::new(),
            UserId::from_string("bob"),
            "Hello!",
        );

        storage.save_message(&message).unwrap();

        let loaded = storage.get_message(message.id.as_str()).unwrap().unwrap();
        assert_eq!(message.id, loaded.id);
    }
}
