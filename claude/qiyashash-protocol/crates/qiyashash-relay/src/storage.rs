//! Relay storage for blob management

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{RelayError, Result};

/// Stored blob metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// Blob ID
    pub id: String,
    /// Size in bytes
    pub size: usize,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Expiry timestamp (Unix seconds)
    pub expires_at: u64,
    /// Hash of the blob data
    #[serde(with = "hex::serde")]
    pub hash: [u8; 32],
}

impl BlobMetadata {
    /// Check if blob is expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

/// Stored blob with data
#[derive(Clone, Debug)]
pub struct StoredBlob {
    /// Metadata
    pub metadata: BlobMetadata,
    /// Encrypted data
    pub data: Vec<u8>,
}

/// Relay storage trait
pub trait RelayStorage: Send + Sync {
    /// Store a blob
    fn store(&self, id: &str, data: Vec<u8>, expiry_secs: u64) -> Result<BlobMetadata>;

    /// Retrieve a blob
    fn retrieve(&self, id: &str) -> Result<Option<StoredBlob>>;

    /// Delete a blob
    fn delete(&self, id: &str) -> Result<bool>;

    /// Check if blob exists
    fn exists(&self, id: &str) -> Result<bool>;

    /// Get blob metadata
    fn get_metadata(&self, id: &str) -> Result<Option<BlobMetadata>>;

    /// List all blob IDs
    fn list_ids(&self) -> Result<Vec<String>>;

    /// Cleanup expired blobs
    fn cleanup_expired(&self) -> Result<usize>;

    /// Get storage stats
    fn stats(&self) -> Result<StorageStats>;
}

/// Storage statistics
#[derive(Clone, Debug, Default)]
pub struct StorageStats {
    /// Total blobs stored
    pub blob_count: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Expired blobs
    pub expired_count: usize,
}

/// In-memory relay storage (for testing)
pub struct MemoryRelayStorage {
    blobs: RwLock<HashMap<String, StoredBlob>>,
    max_size: u64,
}

impl MemoryRelayStorage {
    /// Create new in-memory storage
    pub fn new(max_size: u64) -> Self {
        Self {
            blobs: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    fn current_size(&self) -> u64 {
        self.blobs
            .read()
            .values()
            .map(|b| b.data.len() as u64)
            .sum()
    }

    fn compute_hash(data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl RelayStorage for MemoryRelayStorage {
    fn store(&self, id: &str, data: Vec<u8>, expiry_secs: u64) -> Result<BlobMetadata> {
        let size = data.len();
        
        // Check size limit
        if self.current_size() + size as u64 > self.max_size {
            return Err(RelayError::Storage("Storage full".to_string()));
        }

        let now = Self::current_timestamp();
        let metadata = BlobMetadata {
            id: id.to_string(),
            size,
            created_at: now,
            expires_at: now + expiry_secs,
            hash: Self::compute_hash(&data),
        };

        let blob = StoredBlob {
            metadata: metadata.clone(),
            data,
        };

        self.blobs.write().insert(id.to_string(), blob);
        debug!("Stored blob {}: {} bytes", id, size);

        Ok(metadata)
    }

    fn retrieve(&self, id: &str) -> Result<Option<StoredBlob>> {
        let blobs = self.blobs.read();
        
        match blobs.get(id) {
            Some(blob) => {
                if blob.metadata.is_expired() {
                    debug!("Blob {} is expired", id);
                    Ok(None)
                } else {
                    Ok(Some(blob.clone()))
                }
            }
            None => Ok(None),
        }
    }

    fn delete(&self, id: &str) -> Result<bool> {
        let removed = self.blobs.write().remove(id).is_some();
        if removed {
            debug!("Deleted blob {}", id);
        }
        Ok(removed)
    }

    fn exists(&self, id: &str) -> Result<bool> {
        let blobs = self.blobs.read();
        match blobs.get(id) {
            Some(blob) => Ok(!blob.metadata.is_expired()),
            None => Ok(false),
        }
    }

    fn get_metadata(&self, id: &str) -> Result<Option<BlobMetadata>> {
        Ok(self.blobs.read().get(id).map(|b| b.metadata.clone()))
    }

    fn list_ids(&self) -> Result<Vec<String>> {
        Ok(self.blobs.read().keys().cloned().collect())
    }

    fn cleanup_expired(&self) -> Result<usize> {
        let mut blobs = self.blobs.write();
        let before = blobs.len();
        blobs.retain(|_, blob| !blob.metadata.is_expired());
        let removed = before - blobs.len();
        
        if removed > 0 {
            info!("Cleaned up {} expired blobs", removed);
        }
        
        Ok(removed)
    }

    fn stats(&self) -> Result<StorageStats> {
        let blobs = self.blobs.read();
        let now = Self::current_timestamp();
        
        Ok(StorageStats {
            blob_count: blobs.len(),
            total_size: blobs.values().map(|b| b.data.len() as u64).sum(),
            expired_count: blobs.values().filter(|b| b.metadata.expires_at <= now).count(),
        })
    }
}

/// Sled-based persistent storage
pub struct SledRelayStorage {
    db: sled::Db,
    max_size: u64,
}

impl SledRelayStorage {
    /// Open or create storage at path
    pub fn open(path: &str, max_size: u64) -> Result<Self> {
        let db = sled::open(path)
            .map_err(|e| RelayError::Storage(e.to_string()))?;
        
        Ok(Self { db, max_size })
    }

    fn blob_key(id: &str) -> Vec<u8> {
        format!("blob:{}", id).into_bytes()
    }

    fn meta_key(id: &str) -> Vec<u8> {
        format!("meta:{}", id).into_bytes()
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn compute_hash(data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

impl RelayStorage for SledRelayStorage {
    fn store(&self, id: &str, data: Vec<u8>, expiry_secs: u64) -> Result<BlobMetadata> {
        let size = data.len();
        let now = Self::current_timestamp();
        
        let metadata = BlobMetadata {
            id: id.to_string(),
            size,
            created_at: now,
            expires_at: now + expiry_secs,
            hash: Self::compute_hash(&data),
        };

        let meta_bytes = bincode::serialize(&metadata)
            .map_err(|e| RelayError::Storage(e.to_string()))?;

        self.db.insert(Self::blob_key(id), data)
            .map_err(|e| RelayError::Storage(e.to_string()))?;
        self.db.insert(Self::meta_key(id), meta_bytes)
            .map_err(|e| RelayError::Storage(e.to_string()))?;
        self.db.flush()
            .map_err(|e| RelayError::Storage(e.to_string()))?;

        debug!("Stored blob {}: {} bytes", id, size);
        Ok(metadata)
    }

    fn retrieve(&self, id: &str) -> Result<Option<StoredBlob>> {
        let meta_bytes = match self.db.get(Self::meta_key(id))
            .map_err(|e| RelayError::Storage(e.to_string()))? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let metadata: BlobMetadata = bincode::deserialize(&meta_bytes)
            .map_err(|e| RelayError::Storage(e.to_string()))?;

        if metadata.is_expired() {
            return Ok(None);
        }

        let data = match self.db.get(Self::blob_key(id))
            .map_err(|e| RelayError::Storage(e.to_string()))? {
            Some(bytes) => bytes.to_vec(),
            None => return Ok(None),
        };

        Ok(Some(StoredBlob { metadata, data }))
    }

    fn delete(&self, id: &str) -> Result<bool> {
        let removed = self.db.remove(Self::blob_key(id))
            .map_err(|e| RelayError::Storage(e.to_string()))?
            .is_some();
        self.db.remove(Self::meta_key(id))
            .map_err(|e| RelayError::Storage(e.to_string()))?;
        self.db.flush()
            .map_err(|e| RelayError::Storage(e.to_string()))?;
        Ok(removed)
    }

    fn exists(&self, id: &str) -> Result<bool> {
        match self.get_metadata(id)? {
            Some(meta) => Ok(!meta.is_expired()),
            None => Ok(false),
        }
    }

    fn get_metadata(&self, id: &str) -> Result<Option<BlobMetadata>> {
        match self.db.get(Self::meta_key(id))
            .map_err(|e| RelayError::Storage(e.to_string()))? {
            Some(bytes) => {
                let metadata: BlobMetadata = bincode::deserialize(&bytes)
                    .map_err(|e| RelayError::Storage(e.to_string()))?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    fn list_ids(&self) -> Result<Vec<String>> {
        let prefix = b"meta:";
        let mut ids = Vec::new();
        
        for result in self.db.scan_prefix(prefix) {
            let (key, _) = result.map_err(|e| RelayError::Storage(e.to_string()))?;
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Some(id) = key_str.strip_prefix("meta:") {
                    ids.push(id.to_string());
                }
            }
        }
        
        Ok(ids)
    }

    fn cleanup_expired(&self) -> Result<usize> {
        let ids = self.list_ids()?;
        let mut removed = 0;
        
        for id in ids {
            if let Some(meta) = self.get_metadata(&id)? {
                if meta.is_expired() {
                    self.delete(&id)?;
                    removed += 1;
                }
            }
        }
        
        if removed > 0 {
            info!("Cleaned up {} expired blobs", removed);
        }
        
        Ok(removed)
    }

    fn stats(&self) -> Result<StorageStats> {
        let ids = self.list_ids()?;
        let now = Self::current_timestamp();
        
        let mut total_size = 0u64;
        let mut expired_count = 0;
        
        for id in &ids {
            if let Some(meta) = self.get_metadata(id)? {
                total_size += meta.size as u64;
                if meta.expires_at <= now {
                    expired_count += 1;
                }
            }
        }
        
        Ok(StorageStats {
            blob_count: ids.len(),
            total_size,
            expired_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryRelayStorage::new(1024 * 1024);
        
        // Store
        let meta = storage.store("test-1", vec![0x42; 100], 3600).unwrap();
        assert_eq!(meta.size, 100);
        
        // Retrieve
        let blob = storage.retrieve("test-1").unwrap().unwrap();
        assert_eq!(blob.data.len(), 100);
        
        // Exists
        assert!(storage.exists("test-1").unwrap());
        assert!(!storage.exists("nonexistent").unwrap());
        
        // Delete
        assert!(storage.delete("test-1").unwrap());
        assert!(!storage.exists("test-1").unwrap());
    }

    #[test]
    fn test_stats() {
        let storage = MemoryRelayStorage::new(1024 * 1024);
        
        storage.store("blob-1", vec![0x42; 100], 3600).unwrap();
        storage.store("blob-2", vec![0x42; 200], 3600).unwrap();
        
        let stats = storage.stats().unwrap();
        assert_eq!(stats.blob_count, 2);
        assert_eq!(stats.total_size, 300);
    }
}
