//! RocksDB storage for Identity Service

use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::Path;
use std::sync::Arc;

use crate::error::ServiceError;

/// Column family names
const CF_IDENTITIES: &str = "identities";
const CF_PREKEYS: &str = "prekeys";
const CF_ONE_TIME_PREKEYS: &str = "one_time_prekeys";
const CF_DEVICES: &str = "devices";
const CF_ROTATION_HISTORY: &str = "rotation_history";

/// RocksDB-based storage
pub struct RocksDbStorage {
    db: Arc<DB>,
}

impl RocksDbStorage {
    /// Open storage at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ServiceError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_open_files(256);
        opts.set_keep_log_file_num(5);

        let cf_opts = Options::default();
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_IDENTITIES, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_PREKEYS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_ONE_TIME_PREKEYS, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_DEVICES, cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_ROTATION_HISTORY, cf_opts),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Store identity
    pub fn store_identity(&self, user_id: &str, data: &[u8]) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_IDENTITIES)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        self.db.put_cf(cf, user_id.as_bytes(), data)?;
        Ok(())
    }

    /// Get identity
    pub fn get_identity(&self, user_id: &str) -> Result<Option<Vec<u8>>, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_IDENTITIES)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        Ok(self.db.get_cf(cf, user_id.as_bytes())?)
    }

    /// Delete identity
    pub fn delete_identity(&self, user_id: &str) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_IDENTITIES)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        self.db.delete_cf(cf, user_id.as_bytes())?;
        Ok(())
    }

    /// Store signed prekey
    pub fn store_signed_prekey(
        &self,
        user_id: &str,
        device_id: &str,
        data: &[u8],
    ) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_PREKEYS)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        let key = format!("{}:{}", user_id, device_id);
        self.db.put_cf(cf, key.as_bytes(), data)?;
        Ok(())
    }

    /// Get signed prekey
    pub fn get_signed_prekey(
        &self,
        user_id: &str,
        device_id: &str,
    ) -> Result<Option<Vec<u8>>, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_PREKEYS)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        let key = format!("{}:{}", user_id, device_id);
        Ok(self.db.get_cf(cf, key.as_bytes())?)
    }

    /// Store one-time prekey
    pub fn store_one_time_prekey(
        &self,
        user_id: &str,
        device_id: &str,
        key_id: u32,
        data: &[u8],
    ) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_ONE_TIME_PREKEYS)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        let key = format!("{}:{}:{}", user_id, device_id, key_id);
        self.db.put_cf(cf, key.as_bytes(), data)?;
        Ok(())
    }

    /// Get and consume one-time prekey
    pub fn consume_one_time_prekey(
        &self,
        user_id: &str,
        device_id: &str,
    ) -> Result<Option<(u32, Vec<u8>)>, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_ONE_TIME_PREKEYS)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;

        let prefix = format!("{}:{}:", user_id, device_id);

        // Find first available prekey
        let iter = self.db.prefix_iterator_cf(cf, prefix.as_bytes());
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with(&prefix) {
                // Parse key ID
                let parts: Vec<&str> = key_str.split(':').collect();
                if parts.len() == 3 {
                    if let Ok(key_id) = parts[2].parse::<u32>() {
                        // Delete the key
                        self.db.delete_cf(cf, &key)?;
                        return Ok(Some((key_id, value.to_vec())));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get one-time prekey count
    pub fn get_one_time_prekey_count(
        &self,
        user_id: &str,
        device_id: &str,
    ) -> Result<usize, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_ONE_TIME_PREKEYS)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;

        let prefix = format!("{}:{}:", user_id, device_id);
        let iter = self.db.prefix_iterator_cf(cf, prefix.as_bytes());

        let mut count = 0;
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(&prefix) {
                count += 1;
            } else {
                break;
            }
        }

        Ok(count)
    }

    /// Store device
    pub fn store_device(
        &self,
        user_id: &str,
        device_id: &str,
        data: &[u8],
    ) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_DEVICES)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        let key = format!("{}:{}", user_id, device_id);
        self.db.put_cf(cf, key.as_bytes(), data)?;
        Ok(())
    }

    /// Get devices for user
    pub fn get_devices(&self, user_id: &str) -> Result<Vec<Vec<u8>>, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_DEVICES)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;

        let prefix = format!("{}:", user_id);
        let iter = self.db.prefix_iterator_cf(cf, prefix.as_bytes());

        let mut devices = Vec::new();
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(&prefix) {
                devices.push(value.to_vec());
            } else {
                break;
            }
        }

        Ok(devices)
    }

    /// Store rotation history
    pub fn store_rotation(
        &self,
        user_id: &str,
        timestamp: i64,
        data: &[u8],
    ) -> Result<(), ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_ROTATION_HISTORY)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;
        let key = format!("{}:{:016x}", user_id, timestamp);
        self.db.put_cf(cf, key.as_bytes(), data)?;
        Ok(())
    }

    /// Get rotation history
    pub fn get_rotation_history(&self, user_id: &str) -> Result<Vec<Vec<u8>>, ServiceError> {
        let cf = self
            .db
            .cf_handle(CF_ROTATION_HISTORY)
            .ok_or_else(|| ServiceError::Storage("CF not found".to_string()))?;

        let prefix = format!("{}:", user_id);
        let iter = self.db.prefix_iterator_cf(cf, prefix.as_bytes());

        let mut history = Vec::new();
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(&prefix) {
                history.push(value.to_vec());
            } else {
                break;
            }
        }

        Ok(history)
    }

    /// Flush to disk
    pub fn flush(&self) -> Result<(), ServiceError> {
        self.db.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_operations() {
        let dir = tempdir().unwrap();
        let storage = RocksDbStorage::open(dir.path()).unwrap();

        // Test identity storage
        storage.store_identity("user1", b"identity_data").unwrap();
        let data = storage.get_identity("user1").unwrap();
        assert_eq!(data, Some(b"identity_data".to_vec()));

        storage.delete_identity("user1").unwrap();
        assert!(storage.get_identity("user1").unwrap().is_none());
    }

    #[test]
    fn test_prekey_storage() {
        let dir = tempdir().unwrap();
        let storage = RocksDbStorage::open(dir.path()).unwrap();

        storage
            .store_signed_prekey("user1", "device1", b"prekey_data")
            .unwrap();
        let data = storage.get_signed_prekey("user1", "device1").unwrap();
        assert_eq!(data, Some(b"prekey_data".to_vec()));
    }

    #[test]
    fn test_one_time_prekey() {
        let dir = tempdir().unwrap();
        let storage = RocksDbStorage::open(dir.path()).unwrap();

        storage
            .store_one_time_prekey("user1", "device1", 1, b"otpk1")
            .unwrap();
        storage
            .store_one_time_prekey("user1", "device1", 2, b"otpk2")
            .unwrap();

        assert_eq!(storage.get_one_time_prekey_count("user1", "device1").unwrap(), 2);

        let (key_id, data) = storage
            .consume_one_time_prekey("user1", "device1")
            .unwrap()
            .unwrap();
        assert_eq!(key_id, 1);
        assert_eq!(data, b"otpk1");

        assert_eq!(storage.get_one_time_prekey_count("user1", "device1").unwrap(), 1);
    }
}
