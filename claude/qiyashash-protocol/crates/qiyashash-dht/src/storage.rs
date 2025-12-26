//! Local storage for DHT fragments
//!
//! Uses sled for persistent storage of fragments with automatic expiry.

use sled::{Db, Tree};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::error::{DhtError, Result};
use crate::fragment::{Fragment, FragmentId};

/// DHT local storage
pub struct DhtStorage {
    /// Database handle
    db: Db,
    /// Fragments tree
    fragments: Tree,
    /// Expiry index tree (expiry_timestamp -> fragment_id)
    expiry_index: Tree,
    /// Maximum storage size
    max_size: u64,
}

impl DhtStorage {
    /// Open or create storage at path
    pub fn open(path: impl AsRef<Path>, max_size: u64) -> Result<Self> {
        let db = sled::open(path)?;
        let fragments = db.open_tree("fragments")?;
        let expiry_index = db.open_tree("expiry_index")?;

        let storage = Self {
            db,
            fragments,
            expiry_index,
            max_size,
        };

        // Run initial cleanup
        storage.cleanup_expired()?;

        Ok(storage)
    }

    /// Store a fragment
    pub fn store(&self, fragment: &Fragment) -> Result<()> {
        // Check storage capacity
        if self.size()? > self.max_size {
            warn!("Storage capacity exceeded, running cleanup");
            self.cleanup_expired()?;
            self.cleanup_oldest(self.max_size / 10)?; // Remove 10%
        }

        let key = fragment.id.as_str().as_bytes();
        let value = fragment.to_bytes()?;

        self.fragments.insert(key, value)?;

        // Add to expiry index
        let expiry_key = format!("{:016x}:{}", fragment.expiry, fragment.id);
        self.expiry_index.insert(expiry_key.as_bytes(), key)?;

        debug!("Stored fragment {}", fragment.id);
        Ok(())
    }

    /// Retrieve a fragment
    pub fn get(&self, id: &FragmentId) -> Result<Option<Fragment>> {
        let key = id.as_str().as_bytes();

        match self.fragments.get(key)? {
            Some(value) => {
                let fragment = Fragment::from_bytes(&value)?;

                // Check expiry
                if fragment.is_expired() {
                    debug!("Fragment {} is expired, removing", id);
                    self.remove(id)?;
                    return Ok(None);
                }

                Ok(Some(fragment))
            }
            None => Ok(None),
        }
    }

    /// Remove a fragment
    pub fn remove(&self, id: &FragmentId) -> Result<bool> {
        let key = id.as_str().as_bytes();

        if let Some(value) = self.fragments.remove(key)? {
            // Try to remove from expiry index
            if let Ok(fragment) = Fragment::from_bytes(&value) {
                let expiry_key = format!("{:016x}:{}", fragment.expiry, fragment.id);
                let _ = self.expiry_index.remove(expiry_key.as_bytes());
            }

            debug!("Removed fragment {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if fragment exists
    pub fn contains(&self, id: &FragmentId) -> Result<bool> {
        let key = id.as_str().as_bytes();
        Ok(self.fragments.contains_key(key)?)
    }

    /// Get storage size in bytes
    pub fn size(&self) -> Result<u64> {
        Ok(self.db.size_on_disk()?)
    }

    /// Get fragment count
    pub fn count(&self) -> Result<usize> {
        Ok(self.fragments.len())
    }

    /// List all fragment IDs
    pub fn list_ids(&self) -> Result<Vec<FragmentId>> {
        let mut ids = Vec::new();

        for result in self.fragments.iter() {
            let (key, _) = result?;
            let id_str = String::from_utf8_lossy(&key);
            ids.push(FragmentId(id_str.to_string()));
        }

        Ok(ids)
    }

    /// Cleanup expired fragments
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff = format!("{:016x}", now);
        let mut removed = 0;

        // Iterate through expiry index up to current time
        for result in self.expiry_index.range(..cutoff.as_bytes()) {
            let (expiry_key, fragment_key) = result?;

            // Remove fragment
            if self.fragments.remove(&fragment_key)?.is_some() {
                removed += 1;
            }

            // Remove from index
            self.expiry_index.remove(&expiry_key)?;
        }

        if removed > 0 {
            info!("Cleaned up {} expired fragments", removed);
        }

        Ok(removed)
    }

    /// Cleanup oldest fragments to free space
    pub fn cleanup_oldest(&self, bytes_to_free: u64) -> Result<usize> {
        let mut freed: u64 = 0;
        let mut removed = 0;

        // Iterate through expiry index (oldest first)
        for result in self.expiry_index.iter() {
            if freed >= bytes_to_free {
                break;
            }

            let (expiry_key, fragment_key) = result?;

            if let Some(value) = self.fragments.remove(&fragment_key)? {
                freed += value.len() as u64;
                removed += 1;
            }

            self.expiry_index.remove(&expiry_key)?;
        }

        if removed > 0 {
            info!("Removed {} old fragments to free {} bytes", removed, freed);
        }

        Ok(removed)
    }

    /// Flush to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Get fragments for a message
    pub fn get_message_fragments(&self, message_id: &str) -> Result<Vec<Fragment>> {
        let mut fragments = Vec::new();

        for result in self.fragments.iter() {
            let (_, value) = result?;
            let fragment = Fragment::from_bytes(&value)?;

            if fragment.message_id == message_id && !fragment.is_expired() {
                fragments.push(fragment);
            }
        }

        fragments.sort_by_key(|f| f.index);
        Ok(fragments)
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats> {
        Ok(StorageStats {
            fragment_count: self.count()?,
            size_bytes: self.size()?,
            max_size_bytes: self.max_size,
        })
    }
}

/// Storage statistics
#[derive(Clone, Debug)]
pub struct StorageStats {
    /// Number of fragments stored
    pub fragment_count: usize,
    /// Current size in bytes
    pub size_bytes: u64,
    /// Maximum size in bytes
    pub max_size_bytes: u64,
}

impl StorageStats {
    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.max_size_bytes == 0 {
            0.0
        } else {
            (self.size_bytes as f64 / self.max_size_bytes as f64) * 100.0
        }
    }
}

// Manual implementation of FragmentId for the storage module
impl FragmentId {
    fn new_internal(id: String) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_fragment(id: &str, expiry_offset: i64) -> Fragment {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Fragment {
            id: FragmentId::new("msg-123", 0),
            message_id: "msg-123".to_string(),
            index: 0,
            total: 3,
            data: vec![1, 2, 3, 4],
            is_parity: false,
            shard_size: 4,
            message_size: 10,
            expiry: (now as i64 + expiry_offset) as u64,
            created_at: now,
        }
    }

    #[test]
    fn test_storage_operations() {
        let dir = tempdir().unwrap();
        let storage = DhtStorage::open(dir.path(), 1024 * 1024).unwrap();

        let fragment = create_test_fragment("frag-1", 3600);
        storage.store(&fragment).unwrap();

        assert!(storage.contains(&fragment.id).unwrap());

        let retrieved = storage.get(&fragment.id).unwrap().unwrap();
        assert_eq!(fragment.data, retrieved.data);

        storage.remove(&fragment.id).unwrap();
        assert!(!storage.contains(&fragment.id).unwrap());
    }

    #[test]
    fn test_expired_fragment() {
        let dir = tempdir().unwrap();
        let storage = DhtStorage::open(dir.path(), 1024 * 1024).unwrap();

        let fragment = create_test_fragment("frag-2", -10); // Already expired
        storage.store(&fragment).unwrap();

        // Should return None for expired fragment
        assert!(storage.get(&fragment.id).unwrap().is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let dir = tempdir().unwrap();
        let storage = DhtStorage::open(dir.path(), 1024 * 1024).unwrap();

        let expired = create_test_fragment("frag-3", -10);
        let valid = create_test_fragment("frag-4", 3600);

        storage.store(&expired).unwrap();
        storage.store(&valid).unwrap();

        let removed = storage.cleanup_expired().unwrap();
        assert_eq!(removed, 1);
    }
}
