//! Chain storage interface

use qiyashash_core::session::SessionId;
use qiyashash_crypto::chain::{ChainState, ChainLink};

/// Error type for storage operations
pub type StorageError = Box<dyn std::error::Error + Send + Sync>;

/// Storage trait for chain persistence
pub trait ChainStorage: Send + Sync {
    /// Save entire chain state
    fn save_chain(&self, session_id: &SessionId, chain: &ChainState) -> Result<(), StorageError>;

    /// Load chain state
    fn load_chain(&self, session_id: &SessionId) -> Result<Option<ChainState>, StorageError>;

    /// Delete chain
    fn delete_chain(&self, session_id: &SessionId) -> Result<(), StorageError>;

    /// Save individual link
    fn save_link(&self, session_id: &SessionId, link: &ChainLink) -> Result<(), StorageError>;

    /// Get link by sequence
    fn get_link(&self, session_id: &SessionId, sequence: u64) -> Result<Option<ChainLink>, StorageError>;

    /// List all session IDs with chains
    fn list_sessions(&self) -> Result<Vec<SessionId>, StorageError>;
}

/// In-memory chain storage for testing
pub struct MemoryChainStorage {
    chains: parking_lot::RwLock<std::collections::HashMap<String, Vec<u8>>>,
    links: parking_lot::RwLock<std::collections::HashMap<String, ChainLink>>,
}

impl MemoryChainStorage {
    /// Create new in-memory storage
    pub fn new() -> Self {
        Self {
            chains: parking_lot::RwLock::new(std::collections::HashMap::new()),
            links: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    fn chain_key(session_id: &SessionId) -> String {
        format!("chain:{}", session_id)
    }

    fn link_key(session_id: &SessionId, sequence: u64) -> String {
        format!("link:{}:{}", session_id, sequence)
    }
}

impl Default for MemoryChainStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainStorage for MemoryChainStorage {
    fn save_chain(&self, session_id: &SessionId, chain: &ChainState) -> Result<(), StorageError> {
        // Serialize chain state
        let data = bincode::serialize(chain.history())?;
        self.chains.write().insert(Self::chain_key(session_id), data);
        Ok(())
    }

    fn load_chain(&self, session_id: &SessionId) -> Result<Option<ChainState>, StorageError> {
        match self.chains.read().get(&Self::chain_key(session_id)) {
            Some(data) => {
                // Deserialize and reconstruct chain
                // Note: This is simplified - full implementation would reconstruct ChainState
                Ok(None)
            }
            None => Ok(None),
        }
    }

    fn delete_chain(&self, session_id: &SessionId) -> Result<(), StorageError> {
        self.chains.write().remove(&Self::chain_key(session_id));
        
        // Remove all links for this session
        let prefix = format!("link:{}:", session_id);
        self.links.write().retain(|k, _| !k.starts_with(&prefix));
        
        Ok(())
    }

    fn save_link(&self, session_id: &SessionId, link: &ChainLink) -> Result<(), StorageError> {
        self.links.write().insert(
            Self::link_key(session_id, link.sequence),
            link.clone(),
        );
        Ok(())
    }

    fn get_link(&self, session_id: &SessionId, sequence: u64) -> Result<Option<ChainLink>, StorageError> {
        Ok(self.links.read().get(&Self::link_key(session_id, sequence)).cloned())
    }

    fn list_sessions(&self) -> Result<Vec<SessionId>, StorageError> {
        let chains = self.chains.read();
        let session_ids: Vec<SessionId> = chains
            .keys()
            .filter_map(|k| k.strip_prefix("chain:"))
            .map(|s| SessionId::from_string(s))
            .collect();
        Ok(session_ids)
    }
}

// Helper for SessionId
trait FromString {
    fn from_string(s: &str) -> Self;
}

impl FromString for SessionId {
    fn from_string(s: &str) -> Self {
        // This would need to be implemented in the core crate
        // For now, create a new one
        SessionId::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryChainStorage::new();
        let session_id = SessionId::new();
        
        // Test link storage
        let link = ChainLink {
            link_type: qiyashash_crypto::chain::ChainLinkType::Message,
            state: [0x42; 32],
            message_hash: [0x01; 32],
            timestamp: 12345,
            sequence: 1,
        };

        storage.save_link(&session_id, &link).unwrap();
        let loaded = storage.get_link(&session_id, 1).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().sequence, 1);
    }
}
