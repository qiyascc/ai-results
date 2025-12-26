//! Local storage for CLI client

use sled::Db;
use std::path::Path;

use qiyashash_crypto::identity::{Identity, IdentityKeyPair, IdentityRotationProof};

/// Local storage for CLI
pub struct LocalStorage {
    db: Db,
}

impl LocalStorage {
    /// Open storage at path
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&path)?;
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Check if identity exists
    pub fn has_identity(&self) -> anyhow::Result<bool> {
        Ok(self.db.contains_key("identity")?)
    }

    /// Get identity
    pub fn get_identity(&self) -> anyhow::Result<Option<Identity>> {
        match self.db.get("identity")? {
            Some(data) => {
                let stored: StoredIdentity = bincode::deserialize(&data)?;
                let keypair = IdentityKeyPair::from_secret_bytes(&stored.secret_key);
                Ok(Some(Identity {
                    key_pair: keypair,
                    created_at: stored.created_at,
                    fingerprint: stored.fingerprint,
                }))
            }
            None => Ok(None),
        }
    }

    /// Save identity
    pub fn save_identity(&self, identity: &Identity, device_name: &str) -> anyhow::Result<()> {
        let stored = StoredIdentity {
            secret_key: identity.key_pair.secret_bytes(),
            fingerprint: identity.fingerprint,
            created_at: identity.created_at,
            device_name: device_name.to_string(),
        };

        self.db.insert("identity", bincode::serialize(&stored)?)?;
        self.db.insert("device_name", device_name.as_bytes())?;
        self.db.flush()?;

        Ok(())
    }

    /// Save rotated identity
    pub fn save_rotated_identity(
        &self,
        identity: &Identity,
        proof: &IdentityRotationProof,
    ) -> anyhow::Result<()> {
        // Save new identity
        let device_name = self
            .db
            .get("device_name")?
            .map(|v| String::from_utf8_lossy(&v).to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        self.save_identity(identity, &device_name)?;

        // Save rotation proof to history
        let proof_key = format!("rotation:{}", chrono::Utc::now().timestamp());
        self.db.insert(
            proof_key.as_bytes(),
            serde_json::to_vec(proof)?,
        )?;

        Ok(())
    }

    /// Get device name
    pub fn get_device_name(&self) -> anyhow::Result<Option<String>> {
        Ok(self
            .db
            .get("device_name")?
            .map(|v| String::from_utf8_lossy(&v).to_string()))
    }
}

/// Stored identity format
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredIdentity {
    secret_key: [u8; 32],
    fingerprint: [u8; 32],
    created_at: i64,
    device_name: String,
}
