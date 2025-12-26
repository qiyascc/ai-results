//! Chain State Manager implementation

use crate::error::ChainStateError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sled::Db;
use std::path::Path;
use tracing::{debug, info, warn};

/// A single entry in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    /// Unique entry ID
    pub entry_id: String,
    /// Sequence number in the chain
    pub sequence: u64,
    /// Hash of the previous entry
    pub previous_hash: String,
    /// Hash of this entry's content
    pub content_hash: String,
    /// Hash of the entire entry (including previous_hash)
    pub entry_hash: String,
    /// Timestamp when created
    pub timestamp: DateTime<Utc>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Chain state for a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    /// Chain identifier (typically conversation ID)
    pub chain_id: String,
    /// Latest sequence number
    pub head_sequence: u64,
    /// Hash of the latest entry
    pub head_hash: String,
    /// When the chain was created
    pub created_at: DateTime<Utc>,
    /// When the chain was last updated
    pub updated_at: DateTime<Utc>,
    /// Total number of entries
    pub entry_count: u64,
}

/// Request to append a new entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendRequest {
    /// Chain ID to append to
    pub chain_id: String,
    /// Content hash of the new entry
    pub content_hash: String,
    /// Expected previous hash (for verification)
    pub expected_previous_hash: Option<String>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Chain State Manager
pub struct ChainStateManager {
    /// Database for chain states
    chains_db: Db,
    /// Database for chain entries
    entries_db: Db,
}

impl ChainStateManager {
    /// Create a new chain state manager
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Result<Self, ChainStateError> {
        let path = storage_path.as_ref();
        std::fs::create_dir_all(path).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to create storage directory: {}", e))
        })?;

        let chains_db = sled::open(path.join("chains")).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to open chains database: {}", e))
        })?;

        let entries_db = sled::open(path.join("entries")).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to open entries database: {}", e))
        })?;

        info!("Chain state manager initialized at {:?}", path);
        Ok(Self { chains_db, entries_db })
    }

    /// Create a new chain
    pub fn create_chain(&self, chain_id: &str) -> Result<ChainState, ChainStateError> {
        // Check if chain already exists
        if self.chains_db.contains_key(chain_id).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to check chain existence: {}", e))
        })? {
            return Err(ChainStateError::ValidationError(format!(
                "Chain {} already exists",
                chain_id
            )));
        }

        let now = Utc::now();
        let genesis_hash = self.compute_genesis_hash(chain_id);

        let state = ChainState {
            chain_id: chain_id.to_string(),
            head_sequence: 0,
            head_hash: genesis_hash,
            created_at: now,
            updated_at: now,
            entry_count: 0,
        };

        let serialized = serde_json::to_vec(&state).map_err(|e| {
            ChainStateError::SerializationError(format!("Failed to serialize chain state: {}", e))
        })?;

        self.chains_db.insert(chain_id, serialized).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to store chain state: {}", e))
        })?;

        info!("Created new chain: {}", chain_id);
        Ok(state)
    }

    /// Get chain state
    pub fn get_chain(&self, chain_id: &str) -> Result<ChainState, ChainStateError> {
        let data = self
            .chains_db
            .get(chain_id)
            .map_err(|e| ChainStateError::StorageError(format!("Failed to get chain: {}", e)))?
            .ok_or_else(|| ChainStateError::ChainNotFound(chain_id.to_string()))?;

        serde_json::from_slice(&data).map_err(|e| {
            ChainStateError::SerializationError(format!("Failed to deserialize chain state: {}", e))
        })
    }

    /// Append an entry to a chain
    pub fn append_entry(&self, request: AppendRequest) -> Result<ChainEntry, ChainStateError> {
        let mut state = self.get_chain(&request.chain_id)?;

        // Verify expected previous hash if provided
        if let Some(expected) = &request.expected_previous_hash {
            if expected != &state.head_hash {
                return Err(ChainStateError::HashMismatch {
                    expected: expected.clone(),
                    actual: state.head_hash.clone(),
                });
            }
        }

        let now = Utc::now();
        let new_sequence = state.head_sequence + 1;

        // Compute entry hash
        let entry_hash = self.compute_entry_hash(
            &request.chain_id,
            new_sequence,
            &state.head_hash,
            &request.content_hash,
            &now,
        );

        let entry = ChainEntry {
            entry_id: format!("{}:{}", request.chain_id, new_sequence),
            sequence: new_sequence,
            previous_hash: state.head_hash.clone(),
            content_hash: request.content_hash,
            entry_hash: entry_hash.clone(),
            timestamp: now,
            metadata: request.metadata,
        };

        // Store entry
        let entry_key = format!("{}:{}", request.chain_id, new_sequence);
        let entry_data = serde_json::to_vec(&entry).map_err(|e| {
            ChainStateError::SerializationError(format!("Failed to serialize entry: {}", e))
        })?;

        self.entries_db.insert(&entry_key, entry_data).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to store entry: {}", e))
        })?;

        // Update chain state
        state.head_sequence = new_sequence;
        state.head_hash = entry_hash;
        state.updated_at = now;
        state.entry_count += 1;

        let state_data = serde_json::to_vec(&state).map_err(|e| {
            ChainStateError::SerializationError(format!("Failed to serialize state: {}", e))
        })?;

        self.chains_db.insert(&request.chain_id, state_data).map_err(|e| {
            ChainStateError::StorageError(format!("Failed to update chain state: {}", e))
        })?;

        debug!(
            "Appended entry {} to chain {}",
            new_sequence, request.chain_id
        );
        Ok(entry)
    }

    /// Get an entry by chain ID and sequence number
    pub fn get_entry(&self, chain_id: &str, sequence: u64) -> Result<ChainEntry, ChainStateError> {
        let key = format!("{}:{}", chain_id, sequence);
        let data = self
            .entries_db
            .get(&key)
            .map_err(|e| ChainStateError::StorageError(format!("Failed to get entry: {}", e)))?
            .ok_or_else(|| {
                ChainStateError::ChainNotFound(format!("Entry {} not found", key))
            })?;

        serde_json::from_slice(&data).map_err(|e| {
            ChainStateError::SerializationError(format!("Failed to deserialize entry: {}", e))
        })
    }

    /// Get entries in a range
    pub fn get_entries(
        &self,
        chain_id: &str,
        from_sequence: u64,
        to_sequence: u64,
    ) -> Result<Vec<ChainEntry>, ChainStateError> {
        let mut entries = Vec::new();

        for seq in from_sequence..=to_sequence {
            match self.get_entry(chain_id, seq) {
                Ok(entry) => entries.push(entry),
                Err(ChainStateError::ChainNotFound(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(entries)
    }

    /// Verify chain integrity
    pub fn verify_chain(&self, chain_id: &str) -> Result<bool, ChainStateError> {
        let state = self.get_chain(chain_id)?;

        if state.entry_count == 0 {
            return Ok(true);
        }

        let mut previous_hash = self.compute_genesis_hash(chain_id);

        for seq in 1..=state.head_sequence {
            let entry = self.get_entry(chain_id, seq)?;

            // Verify previous hash linkage
            if entry.previous_hash != previous_hash {
                warn!(
                    "Chain {} integrity failure at sequence {}: previous hash mismatch",
                    chain_id, seq
                );
                return Ok(false);
            }

            // Verify entry hash
            let computed_hash = self.compute_entry_hash(
                chain_id,
                entry.sequence,
                &entry.previous_hash,
                &entry.content_hash,
                &entry.timestamp,
            );

            if entry.entry_hash != computed_hash {
                warn!(
                    "Chain {} integrity failure at sequence {}: entry hash mismatch",
                    chain_id, seq
                );
                return Ok(false);
            }

            previous_hash = entry.entry_hash;
        }

        info!("Chain {} integrity verified", chain_id);
        Ok(true)
    }

    /// List all chains
    pub fn list_chains(&self, limit: usize, offset: usize) -> Result<Vec<ChainState>, ChainStateError> {
        let mut chains = Vec::new();

        for result in self.chains_db.iter().skip(offset).take(limit) {
            let (_, value) = result.map_err(|e| {
                ChainStateError::StorageError(format!("Failed to iterate chains: {}", e))
            })?;

            let state: ChainState = serde_json::from_slice(&value).map_err(|e| {
                ChainStateError::SerializationError(format!("Failed to deserialize chain: {}", e))
            })?;

            chains.push(state);
        }

        Ok(chains)
    }

    /// Compute genesis hash for a chain
    fn compute_genesis_hash(&self, chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"QIYASHASH_GENESIS:");
        hasher.update(chain_id.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Compute entry hash
    fn compute_entry_hash(
        &self,
        chain_id: &str,
        sequence: u64,
        previous_hash: &str,
        content_hash: &str,
        timestamp: &DateTime<Utc>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"QIYASHASH_ENTRY:");
        hasher.update(chain_id.as_bytes());
        hasher.update(b":");
        hasher.update(sequence.to_le_bytes());
        hasher.update(b":");
        hasher.update(previous_hash.as_bytes());
        hasher.update(b":");
        hasher.update(content_hash.as_bytes());
        hasher.update(b":");
        hasher.update(timestamp.timestamp().to_le_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (ChainStateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChainStateManager::new(temp_dir.path()).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_create_chain() {
        let (manager, _temp) = create_test_manager();
        let state = manager.create_chain("test-chain").unwrap();
        
        assert_eq!(state.chain_id, "test-chain");
        assert_eq!(state.head_sequence, 0);
        assert_eq!(state.entry_count, 0);
    }

    #[test]
    fn test_append_entry() {
        let (manager, _temp) = create_test_manager();
        manager.create_chain("test-chain").unwrap();

        let request = AppendRequest {
            chain_id: "test-chain".to_string(),
            content_hash: "abc123".to_string(),
            expected_previous_hash: None,
            metadata: None,
        };

        let entry = manager.append_entry(request).unwrap();
        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.content_hash, "abc123");
    }

    #[test]
    fn test_verify_chain() {
        let (manager, _temp) = create_test_manager();
        manager.create_chain("test-chain").unwrap();

        for i in 0..5 {
            let request = AppendRequest {
                chain_id: "test-chain".to_string(),
                content_hash: format!("content_{}", i),
                expected_previous_hash: None,
                metadata: None,
            };
            manager.append_entry(request).unwrap();
        }

        assert!(manager.verify_chain("test-chain").unwrap());
    }
}
