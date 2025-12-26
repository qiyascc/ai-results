//! Secure storage for mobile

use crate::identity::UserIdentity;
use sled::Db;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Secure local storage
pub struct SecureStorage {
    db: Db,
}

impl SecureStorage {
    /// Open or create storage at path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let db = sled::open(path)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(Self { db })
    }

    /// Save user identity
    pub fn save_identity(&self, identity: &UserIdentity) -> Result<(), StorageError> {
        let data = serde_json::to_vec(identity)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.insert("identity", data)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        self.db.flush()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        
        Ok(())
    }

    /// Load user identity
    pub fn load_identity(&self) -> Result<Option<UserIdentity>, StorageError> {
        match self.db.get("identity") {
            Ok(Some(data)) => {
                let identity = serde_json::from_slice(&data)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(identity))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(e.to_string())),
        }
    }

    /// Save a key-value pair
    pub fn set(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        self.db.insert(key, value)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.db.get(key)
            .map(|opt| opt.map(|v| v.to_vec()))
            .map_err(|e| StorageError::Database(e.to_string()))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<bool, StorageError> {
        self.db.remove(key)
            .map(|opt| opt.is_some())
            .map_err(|e| StorageError::Database(e.to_string()))
    }

    /// Wipe all data
    pub fn wipe_all(&self) -> Result<(), StorageError> {
        self.db.clear()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        self.db.flush()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get storage size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.db.size_on_disk().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_storage_operations() {
        let temp = TempDir::new().unwrap();
        let storage = SecureStorage::new(temp.path()).unwrap();
        
        storage.set("key1", b"value1").unwrap();
        let value = storage.get("key1").unwrap().unwrap();
        assert_eq!(value, b"value1");
        
        storage.delete("key1").unwrap();
        assert!(storage.get("key1").unwrap().is_none());
    }

    #[test]
    fn test_identity_storage() {
        let temp = TempDir::new().unwrap();
        let storage = SecureStorage::new(temp.path()).unwrap();
        
        let identity = UserIdentity::generate("Test".to_string()).unwrap();
        storage.save_identity(&identity).unwrap();
        
        let loaded = storage.load_identity().unwrap().unwrap();
        assert_eq!(loaded.id, identity.id);
    }
}
