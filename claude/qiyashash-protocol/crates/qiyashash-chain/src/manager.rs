//! Chain manager for session chain states

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

use qiyashash_core::session::SessionId;
use qiyashash_crypto::chain::{ChainState, ChainLink, ChainProof, ChainVerifier};

use crate::storage::ChainStorage;

/// Error type for chain operations
#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    /// Chain not found
    #[error("Chain not found: {0}")]
    NotFound(String),

    /// Chain verification failed
    #[error("Chain verification failed: {0}")]
    VerificationFailed(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, ChainError>;

/// Manager for session chain states
pub struct ChainManager {
    /// Active chains in memory
    chains: RwLock<HashMap<SessionId, ChainState>>,
    /// Persistent storage
    storage: Option<Arc<dyn ChainStorage>>,
}

impl ChainManager {
    /// Create a new chain manager (in-memory only)
    pub fn new() -> Self {
        Self {
            chains: RwLock::new(HashMap::new()),
            storage: None,
        }
    }

    /// Create with persistent storage
    pub fn with_storage(storage: Arc<dyn ChainStorage>) -> Self {
        Self {
            chains: RwLock::new(HashMap::new()),
            storage: Some(storage),
        }
    }

    /// Create a new chain for a session
    pub fn create_chain(&self, session_id: &SessionId, shared_secret: &[u8; 32]) -> ChainState {
        let chain = ChainState::from_shared_secret(shared_secret);
        
        self.chains.write().insert(session_id.clone(), chain.clone());
        
        debug!("Created chain for session {}", session_id);
        chain
    }

    /// Get chain for session
    pub fn get_chain(&self, session_id: &SessionId) -> Option<ChainState> {
        self.chains.read().get(session_id).cloned()
    }

    /// Check if chain exists
    pub fn has_chain(&self, session_id: &SessionId) -> bool {
        self.chains.read().contains_key(session_id)
    }

    /// Add message to chain
    pub fn add_message(
        &self,
        session_id: &SessionId,
        message_hash: &[u8; 32],
    ) -> Result<ChainLink> {
        let mut chains = self.chains.write();
        
        let chain = chains.get_mut(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        let link = chain.add_message(message_hash);
        
        debug!(
            "Added message to chain {} (seq {})",
            session_id, link.sequence
        );

        // Persist if storage available
        if let Some(ref storage) = self.storage {
            if let Err(e) = storage.save_link(session_id, &link) {
                warn!("Failed to persist chain link: {}", e);
            }
        }

        Ok(link)
    }

    /// Add deletion to chain
    pub fn add_deletion(
        &self,
        session_id: &SessionId,
        message_hash: &[u8; 32],
    ) -> Result<ChainLink> {
        let mut chains = self.chains.write();
        
        let chain = chains.get_mut(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        let link = chain.add_deletion(message_hash);
        
        debug!("Added deletion to chain {} (seq {})", session_id, link.sequence);

        Ok(link)
    }

    /// Add identity rotation to chain
    pub fn add_identity_rotation(
        &self,
        session_id: &SessionId,
        proof_hash: &[u8; 32],
    ) -> Result<ChainLink> {
        let mut chains = self.chains.write();
        
        let chain = chains.get_mut(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        let link = chain.add_identity_rotation(proof_hash);
        
        info!("Added identity rotation to chain {} (seq {})", session_id, link.sequence);

        Ok(link)
    }

    /// Verify chain integrity
    pub fn verify_chain(&self, session_id: &SessionId) -> Result<()> {
        let chains = self.chains.read();
        
        let chain = chains.get(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        chain.verify_integrity()
            .map_err(|e| ChainError::VerificationFailed(e.to_string()))
    }

    /// Generate chain proof
    pub fn generate_proof(&self, session_id: &SessionId) -> Result<ChainProof> {
        let chains = self.chains.read();
        
        let chain = chains.get(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        Ok(chain.generate_proof())
    }

    /// Get current state hash
    pub fn current_state(&self, session_id: &SessionId) -> Result<[u8; 32]> {
        let chains = self.chains.read();
        
        let chain = chains.get(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        Ok(*chain.current_state())
    }

    /// Get sequence number
    pub fn sequence(&self, session_id: &SessionId) -> Result<u64> {
        let chains = self.chains.read();
        
        let chain = chains.get(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        Ok(chain.sequence())
    }

    /// Get chain history
    pub fn history(&self, session_id: &SessionId) -> Result<Vec<ChainLink>> {
        let chains = self.chains.read();
        
        let chain = chains.get(session_id)
            .ok_or_else(|| ChainError::NotFound(session_id.to_string()))?;

        Ok(chain.history().to_vec())
    }

    /// Remove chain
    pub fn remove_chain(&self, session_id: &SessionId) -> bool {
        let removed = self.chains.write().remove(session_id).is_some();
        
        if removed {
            debug!("Removed chain for session {}", session_id);
            
            if let Some(ref storage) = self.storage {
                if let Err(e) = storage.delete_chain(session_id) {
                    warn!("Failed to delete persisted chain: {}", e);
                }
            }
        }

        removed
    }

    /// Get chain count
    pub fn chain_count(&self) -> usize {
        self.chains.read().len()
    }

    /// Load chain from storage
    pub async fn load_chain(&self, session_id: &SessionId) -> Result<Option<ChainState>> {
        if let Some(ref storage) = self.storage {
            match storage.load_chain(session_id) {
                Ok(Some(chain)) => {
                    self.chains.write().insert(session_id.clone(), chain.clone());
                    Ok(Some(chain))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(ChainError::Storage(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// Persist all chains
    pub fn persist_all(&self) -> Result<()> {
        if let Some(ref storage) = self.storage {
            let chains = self.chains.read();
            
            for (session_id, chain) in chains.iter() {
                if let Err(e) = storage.save_chain(session_id, chain) {
                    warn!("Failed to persist chain {}: {}", session_id, e);
                }
            }
        }
        Ok(())
    }
}

impl Default for ChainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_chain() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let secret = [0x42u8; 32];

        let chain = manager.create_chain(&session_id, &secret);
        assert_eq!(chain.sequence(), 0);
        assert!(manager.has_chain(&session_id));
    }

    #[test]
    fn test_add_messages() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let secret = [0x42u8; 32];

        manager.create_chain(&session_id, &secret);

        let hash1 = [0x01u8; 32];
        let link1 = manager.add_message(&session_id, &hash1).unwrap();
        assert_eq!(link1.sequence, 1);

        let hash2 = [0x02u8; 32];
        let link2 = manager.add_message(&session_id, &hash2).unwrap();
        assert_eq!(link2.sequence, 2);

        assert_eq!(manager.sequence(&session_id).unwrap(), 2);
    }

    #[test]
    fn test_verify_chain() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let secret = [0x42u8; 32];

        manager.create_chain(&session_id, &secret);
        
        for i in 0..10 {
            let hash = [i as u8; 32];
            manager.add_message(&session_id, &hash).unwrap();
        }

        assert!(manager.verify_chain(&session_id).is_ok());
    }

    #[test]
    fn test_chain_not_found() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let hash = [0x01u8; 32];

        let result = manager.add_message(&session_id, &hash);
        assert!(matches!(result, Err(ChainError::NotFound(_))));
    }

    #[test]
    fn test_generate_proof() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let secret = [0x42u8; 32];

        manager.create_chain(&session_id, &secret);
        
        let hash = [0x01u8; 32];
        manager.add_message(&session_id, &hash).unwrap();

        let proof = manager.generate_proof(&session_id).unwrap();
        assert_eq!(proof.sequence, 1);
    }

    #[test]
    fn test_remove_chain() {
        let manager = ChainManager::new();
        let session_id = SessionId::new();
        let secret = [0x42u8; 32];

        manager.create_chain(&session_id, &secret);
        assert!(manager.has_chain(&session_id));

        assert!(manager.remove_chain(&session_id));
        assert!(!manager.has_chain(&session_id));
    }
}
