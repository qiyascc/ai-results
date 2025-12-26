//! Local message storage for DHT Peer

use crate::error::DhtError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info};

/// Stored message record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRecord {
    /// Record key (hash-based)
    pub key: Vec<u8>,
    /// Encrypted message data
    pub value: Vec<u8>,
    /// When the record was stored
    pub stored_at: DateTime<Utc>,
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    /// Publisher peer ID
    pub publisher: Option<String>,
}

/// Message store for local DHT records
pub struct MessageStore {
    db: Db,
    record_count: AtomicUsize,
}

impl MessageStore {
    /// Create a new message store
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DhtError> {
        let full_path = path.as_ref().join("messages");
        std::fs::create_dir_all(&full_path)
            .map_err(|e| DhtError::StorageError(format!("Failed to create directory: {}", e)))?;

        let db = sled::open(&full_path)
            .map_err(|e| DhtError::StorageError(format!("Failed to open database: {}", e)))?;

        let count = db.len();
        info!("Message store opened with {} records", count);

        Ok(Self {
            db,
            record_count: AtomicUsize::new(count),
        })
    }

    /// Store a record
    pub fn put(&self, key: &[u8], value: &[u8], ttl_seconds: u64, publisher: Option<String>) -> Result<(), DhtError> {
        let record = StoredRecord {
            key: key.to_vec(),
            value: value.to_vec(),
            stored_at: Utc::now(),
            ttl_seconds,
            publisher,
        };

        let serialized = bincode::serialize(&record)
            .map_err(|e| DhtError::SerializationError(e.to_string()))?;

        let is_new = !self.db.contains_key(key)
            .map_err(|e| DhtError::StorageError(e.to_string()))?;

        self.db.insert(key, serialized)
            .map_err(|e| DhtError::StorageError(e.to_string()))?;

        if is_new {
            self.record_count.fetch_add(1, Ordering::Relaxed);
        }

        debug!("Stored record with key: {}", hex::encode(key));
        Ok(())
    }

    /// Get a record
    pub fn get(&self, key: &[u8]) -> Result<Option<StoredRecord>, DhtError> {
        match self.db.get(key) {
            Ok(Some(data)) => {
                let record: StoredRecord = bincode::deserialize(&data)
                    .map_err(|e| DhtError::SerializationError(e.to_string()))?;

                // Check TTL
                let age = Utc::now()
                    .signed_duration_since(record.stored_at)
                    .num_seconds() as u64;

                if age > record.ttl_seconds {
                    // Record expired, remove it
                    let _ = self.remove(key);
                    return Ok(None);
                }

                Ok(Some(record))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(DhtError::StorageError(e.to_string())),
        }
    }

    /// Remove a record
    pub fn remove(&self, key: &[u8]) -> Result<bool, DhtError> {
        let existed = self.db.remove(key)
            .map_err(|e| DhtError::StorageError(e.to_string()))?
            .is_some();

        if existed {
            self.record_count.fetch_sub(1, Ordering::Relaxed);
        }

        Ok(existed)
    }

    /// Get record count
    pub fn record_count(&self) -> usize {
        self.record_count.load(Ordering::Relaxed)
    }

    /// Clean expired records
    pub fn cleanup_expired(&self) -> Result<usize, DhtError> {
        let mut removed = 0;
        let now = Utc::now();

        for result in self.db.iter() {
            let (key, value) = result
                .map_err(|e| DhtError::StorageError(e.to_string()))?;

            if let Ok(record) = bincode::deserialize::<StoredRecord>(&value) {
                let age = now
                    .signed_duration_since(record.stored_at)
                    .num_seconds() as u64;

                if age > record.ttl_seconds {
                    self.db.remove(&key)
                        .map_err(|e| DhtError::StorageError(e.to_string()))?;
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            self.record_count.fetch_sub(removed, Ordering::Relaxed);
            info!("Cleaned up {} expired records", removed);
        }

        Ok(removed)
    }

    /// List all keys
    pub fn list_keys(&self, limit: usize) -> Result<Vec<Vec<u8>>, DhtError> {
        let mut keys = Vec::new();

        for result in self.db.iter().take(limit) {
            let (key, _) = result
                .map_err(|e| DhtError::StorageError(e.to_string()))?;
            keys.push(key.to_vec());
        }

        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve() {
        let temp = TempDir::new().unwrap();
        let store = MessageStore::new(temp.path()).unwrap();

        let key = b"test-key";
        let value = b"test-value";

        store.put(key, value, 3600, None).unwrap();
        
        let record = store.get(key).unwrap().unwrap();
        assert_eq!(record.value, value);
        assert_eq!(store.record_count(), 1);
    }

    #[test]
    fn test_remove() {
        let temp = TempDir::new().unwrap();
        let store = MessageStore::new(temp.path()).unwrap();

        let key = b"test-key";
        store.put(key, b"value", 3600, None).unwrap();
        assert_eq!(store.record_count(), 1);

        store.remove(key).unwrap();
        assert_eq!(store.record_count(), 0);
        assert!(store.get(key).unwrap().is_none());
    }
}
