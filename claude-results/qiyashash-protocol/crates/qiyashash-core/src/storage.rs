//! Storage traits for QiyasHash
//!
//! Defines abstract storage interfaces that can be implemented
//! for different backends (RocksDB, SQLite, memory, etc.)

use async_trait::async_trait;

use crate::error::Result;
use crate::message::{Message, MessageId};
use crate::session::{SessionId, SessionRecord};
use crate::types::{DeviceId, UserId};
use crate::user::{Contact, User};

/// Storage for user data
#[async_trait]
pub trait UserStore: Send + Sync {
    /// Get user by ID
    async fn get_user(&self, user_id: &UserId) -> Result<Option<User>>;

    /// Save user
    async fn save_user(&self, user: &User) -> Result<()>;

    /// Delete user
    async fn delete_user(&self, user_id: &UserId) -> Result<()>;

    /// Get all users
    async fn get_all_users(&self) -> Result<Vec<User>>;

    /// Search users by name
    async fn search_users(&self, query: &str) -> Result<Vec<User>>;

    /// Get contact
    async fn get_contact(&self, user_id: &UserId) -> Result<Option<Contact>>;

    /// Save contact
    async fn save_contact(&self, contact: &Contact) -> Result<()>;

    /// Delete contact
    async fn delete_contact(&self, user_id: &UserId) -> Result<()>;

    /// Get all contacts
    async fn get_all_contacts(&self) -> Result<Vec<Contact>>;

    /// Get blocked contacts
    async fn get_blocked_contacts(&self) -> Result<Vec<Contact>>;
}

/// Storage for session data
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Get session by ID
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionRecord>>;

    /// Get session by user and device
    async fn get_session_by_user_device(
        &self,
        their_user_id: &UserId,
        their_device_id: &DeviceId,
    ) -> Result<Option<SessionRecord>>;

    /// Save session
    async fn save_session(&self, session: &SessionRecord) -> Result<()>;

    /// Delete session
    async fn delete_session(&self, session_id: &SessionId) -> Result<()>;

    /// Get all sessions for a user
    async fn get_sessions_for_user(&self, their_user_id: &UserId) -> Result<Vec<SessionRecord>>;

    /// Get all active sessions
    async fn get_active_sessions(&self) -> Result<Vec<SessionRecord>>;

    /// Get sessions needing re-key
    async fn get_sessions_needing_rekey(&self) -> Result<Vec<SessionRecord>>;

    /// Update ratchet state
    async fn update_ratchet_state(
        &self,
        session_id: &SessionId,
        ratchet_state: Vec<u8>,
        chain_state: Vec<u8>,
    ) -> Result<()>;
}

/// Storage for messages
#[async_trait]
pub trait MessageStore: Send + Sync {
    /// Get message by ID
    async fn get_message(&self, message_id: &MessageId) -> Result<Option<Message>>;

    /// Save message
    async fn save_message(&self, message: &Message) -> Result<()>;

    /// Delete message
    async fn delete_message(&self, message_id: &MessageId) -> Result<()>;

    /// Get messages for conversation
    async fn get_messages_for_conversation(
        &self,
        other_user_id: &UserId,
        limit: usize,
        before: Option<&MessageId>,
    ) -> Result<Vec<Message>>;

    /// Get unread message count
    async fn get_unread_count(&self, other_user_id: &UserId) -> Result<usize>;

    /// Mark messages as read
    async fn mark_as_read(&self, other_user_id: &UserId, until: &MessageId) -> Result<()>;

    /// Search messages
    async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<Message>>;

    /// Get messages pending send
    async fn get_pending_messages(&self) -> Result<Vec<Message>>;

    /// Get expired messages (for cleanup)
    async fn get_expired_messages(&self) -> Result<Vec<MessageId>>;

    /// Delete all messages for conversation
    async fn delete_conversation(&self, other_user_id: &UserId) -> Result<()>;
}

/// Storage for identity keys
#[async_trait]
pub trait IdentityStore: Send + Sync {
    /// Get our identity key pair (encrypted)
    async fn get_identity_key(&self) -> Result<Option<Vec<u8>>>;

    /// Save our identity key pair (encrypted)
    async fn save_identity_key(&self, encrypted_key: Vec<u8>) -> Result<()>;

    /// Get their identity key
    async fn get_remote_identity(&self, user_id: &UserId) -> Result<Option<[u8; 32]>>;

    /// Save their identity key
    async fn save_remote_identity(&self, user_id: &UserId, identity_key: [u8; 32]) -> Result<()>;

    /// Check if identity is trusted
    async fn is_trusted_identity(&self, user_id: &UserId, identity_key: &[u8; 32]) -> Result<bool>;
}

/// Storage for prekeys
#[async_trait]
pub trait PreKeyStore: Send + Sync {
    /// Get signed prekey
    async fn get_signed_prekey(&self, id: u32) -> Result<Option<Vec<u8>>>;

    /// Save signed prekey
    async fn save_signed_prekey(&self, id: u32, prekey: Vec<u8>) -> Result<()>;

    /// Delete signed prekey
    async fn delete_signed_prekey(&self, id: u32) -> Result<()>;

    /// Get one-time prekey
    async fn get_one_time_prekey(&self, id: u32) -> Result<Option<Vec<u8>>>;

    /// Save one-time prekey
    async fn save_one_time_prekey(&self, id: u32, prekey: Vec<u8>) -> Result<()>;

    /// Delete one-time prekey
    async fn delete_one_time_prekey(&self, id: u32) -> Result<()>;

    /// Get count of available one-time prekeys
    async fn get_one_time_prekey_count(&self) -> Result<usize>;

    /// Get all one-time prekey IDs
    async fn get_one_time_prekey_ids(&self) -> Result<Vec<u32>>;
}

/// Combined storage interface
#[async_trait]
pub trait Storage:
    UserStore + SessionStore + MessageStore + IdentityStore + PreKeyStore + Send + Sync
{
    /// Begin a transaction
    async fn begin_transaction(&self) -> Result<()>;

    /// Commit transaction
    async fn commit(&self) -> Result<()>;

    /// Rollback transaction
    async fn rollback(&self) -> Result<()>;

    /// Flush all pending writes
    async fn flush(&self) -> Result<()>;

    /// Get storage stats
    async fn get_stats(&self) -> Result<StorageStats>;

    /// Vacuum/compact storage
    async fn vacuum(&self) -> Result<()>;
}

/// Storage statistics
#[derive(Clone, Debug, Default)]
pub struct StorageStats {
    /// Number of users
    pub user_count: usize,
    /// Number of sessions
    pub session_count: usize,
    /// Number of messages
    pub message_count: usize,
    /// Storage size in bytes
    pub storage_size_bytes: u64,
    /// One-time prekeys remaining
    pub prekey_count: usize,
}

/// In-memory storage for testing
pub mod memory {
    use super::*;
    use parking_lot::RwLock;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// In-memory storage implementation
    pub struct MemoryStorage {
        users: RwLock<HashMap<String, User>>,
        contacts: RwLock<HashMap<String, Contact>>,
        sessions: RwLock<HashMap<String, SessionRecord>>,
        messages: RwLock<HashMap<String, Message>>,
        identity_key: RwLock<Option<Vec<u8>>>,
        remote_identities: RwLock<HashMap<String, [u8; 32]>>,
        signed_prekeys: RwLock<HashMap<u32, Vec<u8>>>,
        one_time_prekeys: RwLock<HashMap<u32, Vec<u8>>>,
    }

    impl MemoryStorage {
        /// Create new in-memory storage
        pub fn new() -> Arc<Self> {
            Arc::new(Self {
                users: RwLock::new(HashMap::new()),
                contacts: RwLock::new(HashMap::new()),
                sessions: RwLock::new(HashMap::new()),
                messages: RwLock::new(HashMap::new()),
                identity_key: RwLock::new(None),
                remote_identities: RwLock::new(HashMap::new()),
                signed_prekeys: RwLock::new(HashMap::new()),
                one_time_prekeys: RwLock::new(HashMap::new()),
            })
        }
    }

    impl Default for MemoryStorage {
        fn default() -> Self {
            Self {
                users: RwLock::new(HashMap::new()),
                contacts: RwLock::new(HashMap::new()),
                sessions: RwLock::new(HashMap::new()),
                messages: RwLock::new(HashMap::new()),
                identity_key: RwLock::new(None),
                remote_identities: RwLock::new(HashMap::new()),
                signed_prekeys: RwLock::new(HashMap::new()),
                one_time_prekeys: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl UserStore for MemoryStorage {
        async fn get_user(&self, user_id: &UserId) -> Result<Option<User>> {
            Ok(self.users.read().get(user_id.as_str()).cloned())
        }

        async fn save_user(&self, user: &User) -> Result<()> {
            self.users
                .write()
                .insert(user.id.as_str().to_string(), user.clone());
            Ok(())
        }

        async fn delete_user(&self, user_id: &UserId) -> Result<()> {
            self.users.write().remove(user_id.as_str());
            Ok(())
        }

        async fn get_all_users(&self) -> Result<Vec<User>> {
            Ok(self.users.read().values().cloned().collect())
        }

        async fn search_users(&self, query: &str) -> Result<Vec<User>> {
            let query = query.to_lowercase();
            Ok(self
                .users
                .read()
                .values()
                .filter(|u| {
                    u.profile
                        .display_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query))
                        .unwrap_or(false)
                })
                .cloned()
                .collect())
        }

        async fn get_contact(&self, user_id: &UserId) -> Result<Option<Contact>> {
            Ok(self.contacts.read().get(user_id.as_str()).cloned())
        }

        async fn save_contact(&self, contact: &Contact) -> Result<()> {
            self.contacts
                .write()
                .insert(contact.user_id.as_str().to_string(), contact.clone());
            Ok(())
        }

        async fn delete_contact(&self, user_id: &UserId) -> Result<()> {
            self.contacts.write().remove(user_id.as_str());
            Ok(())
        }

        async fn get_all_contacts(&self) -> Result<Vec<Contact>> {
            Ok(self.contacts.read().values().cloned().collect())
        }

        async fn get_blocked_contacts(&self) -> Result<Vec<Contact>> {
            Ok(self
                .contacts
                .read()
                .values()
                .filter(|c| c.is_blocked)
                .cloned()
                .collect())
        }
    }

    #[async_trait]
    impl SessionStore for MemoryStorage {
        async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionRecord>> {
            Ok(self.sessions.read().get(session_id.as_str()).cloned())
        }

        async fn get_session_by_user_device(
            &self,
            their_user_id: &UserId,
            their_device_id: &DeviceId,
        ) -> Result<Option<SessionRecord>> {
            Ok(self
                .sessions
                .read()
                .values()
                .find(|s| {
                    s.session.their_user_id == *their_user_id
                        && s.session.their_device_id == *their_device_id
                })
                .cloned())
        }

        async fn save_session(&self, session: &SessionRecord) -> Result<()> {
            self.sessions
                .write()
                .insert(session.session.id.as_str().to_string(), session.clone());
            Ok(())
        }

        async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
            self.sessions.write().remove(session_id.as_str());
            Ok(())
        }

        async fn get_sessions_for_user(&self, their_user_id: &UserId) -> Result<Vec<SessionRecord>> {
            Ok(self
                .sessions
                .read()
                .values()
                .filter(|s| s.session.their_user_id == *their_user_id)
                .cloned()
                .collect())
        }

        async fn get_active_sessions(&self) -> Result<Vec<SessionRecord>> {
            use crate::session::SessionState;
            Ok(self
                .sessions
                .read()
                .values()
                .filter(|s| s.session.state == SessionState::Active)
                .cloned()
                .collect())
        }

        async fn get_sessions_needing_rekey(&self) -> Result<Vec<SessionRecord>> {
            Ok(self
                .sessions
                .read()
                .values()
                .filter(|s| s.session.needs_rekey())
                .cloned()
                .collect())
        }

        async fn update_ratchet_state(
            &self,
            session_id: &SessionId,
            ratchet_state: Vec<u8>,
            chain_state: Vec<u8>,
        ) -> Result<()> {
            if let Some(session) = self.sessions.write().get_mut(session_id.as_str()) {
                session.ratchet_state = ratchet_state;
                session.chain_state = chain_state;
            }
            Ok(())
        }
    }

    #[async_trait]
    impl MessageStore for MemoryStorage {
        async fn get_message(&self, message_id: &MessageId) -> Result<Option<Message>> {
            Ok(self.messages.read().get(message_id.as_str()).cloned())
        }

        async fn save_message(&self, message: &Message) -> Result<()> {
            self.messages
                .write()
                .insert(message.id.as_str().to_string(), message.clone());
            Ok(())
        }

        async fn delete_message(&self, message_id: &MessageId) -> Result<()> {
            self.messages.write().remove(message_id.as_str());
            Ok(())
        }

        async fn get_messages_for_conversation(
            &self,
            other_user_id: &UserId,
            limit: usize,
            _before: Option<&MessageId>,
        ) -> Result<Vec<Message>> {
            let mut msgs: Vec<_> = self
                .messages
                .read()
                .values()
                .filter(|m| {
                    m.sender_id == *other_user_id || m.recipient_id == *other_user_id
                })
                .cloned()
                .collect();
            msgs.sort_by(|a, b| b.created_at.as_millis().cmp(&a.created_at.as_millis()));
            msgs.truncate(limit);
            Ok(msgs)
        }

        async fn get_unread_count(&self, _other_user_id: &UserId) -> Result<usize> {
            Ok(0) // Simplified
        }

        async fn mark_as_read(&self, _other_user_id: &UserId, _until: &MessageId) -> Result<()> {
            Ok(()) // Simplified
        }

        async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<Message>> {
            let query = query.to_lowercase();
            let mut msgs: Vec<_> = self
                .messages
                .read()
                .values()
                .filter(|m| {
                    String::from_utf8(m.content.clone())
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            msgs.truncate(limit);
            Ok(msgs)
        }

        async fn get_pending_messages(&self) -> Result<Vec<Message>> {
            use crate::message::MessageStatus;
            Ok(self
                .messages
                .read()
                .values()
                .filter(|m| m.status == MessageStatus::Pending)
                .cloned()
                .collect())
        }

        async fn get_expired_messages(&self) -> Result<Vec<MessageId>> {
            Ok(self
                .messages
                .read()
                .values()
                .filter(|m| m.is_expired())
                .map(|m| m.id.clone())
                .collect())
        }

        async fn delete_conversation(&self, other_user_id: &UserId) -> Result<()> {
            self.messages.write().retain(|_, m| {
                m.sender_id != *other_user_id && m.recipient_id != *other_user_id
            });
            Ok(())
        }
    }

    #[async_trait]
    impl IdentityStore for MemoryStorage {
        async fn get_identity_key(&self) -> Result<Option<Vec<u8>>> {
            Ok(self.identity_key.read().clone())
        }

        async fn save_identity_key(&self, encrypted_key: Vec<u8>) -> Result<()> {
            *self.identity_key.write() = Some(encrypted_key);
            Ok(())
        }

        async fn get_remote_identity(&self, user_id: &UserId) -> Result<Option<[u8; 32]>> {
            Ok(self.remote_identities.read().get(user_id.as_str()).copied())
        }

        async fn save_remote_identity(
            &self,
            user_id: &UserId,
            identity_key: [u8; 32],
        ) -> Result<()> {
            self.remote_identities
                .write()
                .insert(user_id.as_str().to_string(), identity_key);
            Ok(())
        }

        async fn is_trusted_identity(
            &self,
            user_id: &UserId,
            identity_key: &[u8; 32],
        ) -> Result<bool> {
            Ok(self
                .remote_identities
                .read()
                .get(user_id.as_str())
                .map(|k| k == identity_key)
                .unwrap_or(true)) // Trust on first use
        }
    }

    #[async_trait]
    impl PreKeyStore for MemoryStorage {
        async fn get_signed_prekey(&self, id: u32) -> Result<Option<Vec<u8>>> {
            Ok(self.signed_prekeys.read().get(&id).cloned())
        }

        async fn save_signed_prekey(&self, id: u32, prekey: Vec<u8>) -> Result<()> {
            self.signed_prekeys.write().insert(id, prekey);
            Ok(())
        }

        async fn delete_signed_prekey(&self, id: u32) -> Result<()> {
            self.signed_prekeys.write().remove(&id);
            Ok(())
        }

        async fn get_one_time_prekey(&self, id: u32) -> Result<Option<Vec<u8>>> {
            Ok(self.one_time_prekeys.read().get(&id).cloned())
        }

        async fn save_one_time_prekey(&self, id: u32, prekey: Vec<u8>) -> Result<()> {
            self.one_time_prekeys.write().insert(id, prekey);
            Ok(())
        }

        async fn delete_one_time_prekey(&self, id: u32) -> Result<()> {
            self.one_time_prekeys.write().remove(&id);
            Ok(())
        }

        async fn get_one_time_prekey_count(&self) -> Result<usize> {
            Ok(self.one_time_prekeys.read().len())
        }

        async fn get_one_time_prekey_ids(&self) -> Result<Vec<u32>> {
            Ok(self.one_time_prekeys.read().keys().copied().collect())
        }
    }

    #[async_trait]
    impl Storage for MemoryStorage {
        async fn begin_transaction(&self) -> Result<()> {
            Ok(())
        }

        async fn commit(&self) -> Result<()> {
            Ok(())
        }

        async fn rollback(&self) -> Result<()> {
            Ok(())
        }

        async fn flush(&self) -> Result<()> {
            Ok(())
        }

        async fn get_stats(&self) -> Result<StorageStats> {
            Ok(StorageStats {
                user_count: self.users.read().len(),
                session_count: self.sessions.read().len(),
                message_count: self.messages.read().len(),
                storage_size_bytes: 0,
                prekey_count: self.one_time_prekeys.read().len(),
            })
        }

        async fn vacuum(&self) -> Result<()> {
            Ok(())
        }
    }
}
