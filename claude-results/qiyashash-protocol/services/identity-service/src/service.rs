//! Identity service implementation

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use qiyashash_crypto::identity::{Identity, IdentityKeyPair, IdentityPublicKey};
use qiyashash_crypto::x3dh::PreKeyManager;

use crate::api::{
    GenerateIdentityResponse, GetPreKeysResponse, OneTimePreKeyInput, OneTimePreKeyResponse,
    PreKeyBundleResponse, RegisterPreKeysResponse, RotateIdentityResponse, RotationProofResponse,
    SignedPreKeyResponse, VerifyIdentityResponse,
};
use crate::error::ServiceError;
use crate::storage::RocksDbStorage;

/// Stored identity data
#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    user_id: String,
    identity_key_secret: String, // hex-encoded
    fingerprint: String,
    created_at: i64,
}

/// Stored device data
#[derive(Serialize, Deserialize)]
struct StoredDevice {
    device_id: String,
    device_name: String,
    registration_id: u32,
    created_at: i64,
}

/// Stored prekey
#[derive(Serialize, Deserialize)]
struct StoredPreKey {
    id: u32,
    public_key: String,
    signature: Option<String>,
}

/// Identity service implementation
pub struct IdentityServiceImpl {
    storage: RocksDbStorage,
}

impl IdentityServiceImpl {
    /// Create new service
    pub fn new(storage: RocksDbStorage) -> Self {
        Self { storage }
    }

    /// Generate a new identity
    pub async fn generate_identity(
        &self,
        device_name: &str,
    ) -> Result<GenerateIdentityResponse, ServiceError> {
        // Generate identity
        let identity = Identity::new();
        let public_key = identity.key_pair.public_key();

        // Generate user ID from fingerprint
        let user_id = hex::encode(&identity.fingerprint[..16]);
        let device_id = uuid::Uuid::new_v4().to_string();

        // Create prekey manager
        let mut prekey_manager = PreKeyManager::new(identity.key_pair.clone());
        prekey_manager.generate_one_time_prekeys(100);

        let bundle = prekey_manager.get_bundle();

        // Store identity
        let stored_identity = StoredIdentity {
            user_id: user_id.clone(),
            identity_key_secret: hex::encode(identity.key_pair.secret_bytes()),
            fingerprint: hex::encode(identity.fingerprint),
            created_at: chrono::Utc::now().timestamp(),
        };

        self.storage.store_identity(
            &user_id,
            &serde_json::to_vec(&stored_identity)?,
        )?;

        // Store device
        let stored_device = StoredDevice {
            device_id: device_id.clone(),
            device_name: device_name.to_string(),
            registration_id: rand::random(),
            created_at: chrono::Utc::now().timestamp(),
        };

        self.storage.store_device(
            &user_id,
            &device_id,
            &serde_json::to_vec(&stored_device)?,
        )?;

        // Store signed prekey
        let signed_prekey = StoredPreKey {
            id: bundle.signed_prekey.id,
            public_key: hex::encode(bundle.signed_prekey.public_key.as_bytes()),
            signature: Some(hex::encode(bundle.signed_prekey.signature)),
        };

        self.storage.store_signed_prekey(
            &user_id,
            &device_id,
            &serde_json::to_vec(&signed_prekey)?,
        )?;

        // Store one-time prekeys
        if let Some(ref otpk) = bundle.one_time_prekey {
            let stored_otpk = StoredPreKey {
                id: otpk.id,
                public_key: hex::encode(otpk.public_key.as_bytes()),
                signature: None,
            };

            self.storage.store_one_time_prekey(
                &user_id,
                &device_id,
                otpk.id,
                &serde_json::to_vec(&stored_otpk)?,
            )?;
        }

        info!("Generated new identity for user: {}", user_id);

        Ok(GenerateIdentityResponse {
            user_id,
            device_id,
            identity_key: hex::encode(public_key.signing_key_bytes()),
            fingerprint: hex::encode(identity.fingerprint),
            signed_prekey: SignedPreKeyResponse {
                id: bundle.signed_prekey.id,
                public_key: hex::encode(bundle.signed_prekey.public_key.as_bytes()),
                signature: hex::encode(bundle.signed_prekey.signature),
            },
            one_time_prekeys: bundle
                .one_time_prekey
                .map(|otpk| {
                    vec![OneTimePreKeyResponse {
                        id: otpk.id,
                        public_key: hex::encode(otpk.public_key.as_bytes()),
                    }]
                })
                .unwrap_or_default(),
        })
    }

    /// Rotate identity
    pub async fn rotate_identity(
        &self,
        user_id: &str,
        device_id: &str,
    ) -> Result<RotateIdentityResponse, ServiceError> {
        // Get current identity
        let identity_data = self
            .storage
            .get_identity(user_id)?
            .ok_or_else(|| ServiceError::NotFound(format!("User {} not found", user_id)))?;

        let stored: StoredIdentity = serde_json::from_slice(&identity_data)?;

        // Restore old identity
        let old_secret = hex::decode(&stored.identity_key_secret)?;
        let old_secret_arr: [u8; 32] = old_secret
            .try_into()
            .map_err(|_| ServiceError::Crypto("Invalid key length".to_string()))?;

        let old_keypair = IdentityKeyPair::from_secret_bytes(&old_secret_arr);
        let old_identity = Identity::from_key_pair(old_keypair);

        // Rotate
        let (new_identity, proof) = old_identity.rotate();

        // Update storage
        let new_stored = StoredIdentity {
            user_id: user_id.to_string(),
            identity_key_secret: hex::encode(new_identity.key_pair.secret_bytes()),
            fingerprint: hex::encode(new_identity.fingerprint),
            created_at: chrono::Utc::now().timestamp(),
        };

        self.storage
            .store_identity(user_id, &serde_json::to_vec(&new_stored)?)?;

        // Store rotation history
        self.storage.store_rotation(
            user_id,
            chrono::Utc::now().timestamp(),
            &serde_json::to_vec(&proof)?,
        )?;

        info!("Rotated identity for user: {}", user_id);

        Ok(RotateIdentityResponse {
            new_identity_key: hex::encode(new_identity.key_pair.public_key().signing_key_bytes()),
            new_fingerprint: hex::encode(new_identity.fingerprint),
            rotation_proof: RotationProofResponse {
                old_public_key: hex::encode(proof.old_public_key.signing_key),
                new_public_key: hex::encode(proof.new_public_key.signing_key),
                old_signature: hex::encode(proof.old_signature),
                new_signature: hex::encode(proof.new_signature),
                timestamp: proof.timestamp,
                commitment: hex::encode(proof.commitment),
            },
        })
    }

    /// Verify identity
    pub async fn verify_identity(
        &self,
        user_id: &str,
        identity_key: &str,
        signature: &str,
        message: &str,
    ) -> Result<VerifyIdentityResponse, ServiceError> {
        // Get stored identity
        let identity_data = self.storage.get_identity(user_id)?;

        let trusted = if let Some(data) = &identity_data {
            let stored: StoredIdentity = serde_json::from_slice(data)?;
            // Check if provided key matches stored key
            let stored_pub = hex::decode(&stored.identity_key_secret)?;
            let stored_secret: [u8; 32] = stored_pub
                .try_into()
                .map_err(|_| ServiceError::Crypto("Invalid key length".to_string()))?;
            let stored_keypair = IdentityKeyPair::from_secret_bytes(&stored_secret);
            let stored_public = stored_keypair.public_key();

            hex::encode(stored_public.signing_key_bytes()) == identity_key
        } else {
            false
        };

        // Verify signature
        let identity_key_bytes = hex::decode(identity_key)?;
        let identity_key_arr: [u8; 32] = identity_key_bytes
            .try_into()
            .map_err(|_| ServiceError::BadRequest("Invalid identity key length".to_string()))?;

        let public_key = IdentityPublicKey::from_bytes(&identity_key_arr)?;

        let signature_bytes = hex::decode(signature)?;
        let signature_arr: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| ServiceError::BadRequest("Invalid signature length".to_string()))?;

        let valid = public_key.verify(message.as_bytes(), &signature_arr).is_ok();

        // Compute fingerprint
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&identity_key_arr);
        let fingerprint = hex::encode(hasher.finalize());

        debug!("Verified identity for user {}: valid={}", user_id, valid);

        Ok(VerifyIdentityResponse {
            valid,
            fingerprint,
            trusted,
        })
    }

    /// Get prekey status
    pub async fn get_prekey_status(
        &self,
        user_id: &str,
    ) -> Result<GetPreKeysResponse, ServiceError> {
        // Get devices
        let devices = self.storage.get_devices(user_id)?;

        if devices.is_empty() {
            return Err(ServiceError::NotFound(format!("User {} not found", user_id)));
        }

        // Get first device
        let device: StoredDevice = serde_json::from_slice(&devices[0])?;

        // Get prekey count
        let count = self
            .storage
            .get_one_time_prekey_count(user_id, &device.device_id)?;

        // Get signed prekey
        let signed_prekey = self
            .storage
            .get_signed_prekey(user_id, &device.device_id)?
            .map(|data| serde_json::from_slice::<StoredPreKey>(&data).ok())
            .flatten();

        Ok(GetPreKeysResponse {
            count,
            needs_replenishment: count < 20,
            signed_prekey_id: signed_prekey.map(|p| p.id).unwrap_or(0),
        })
    }

    /// Register new prekeys
    pub async fn register_prekeys(
        &self,
        user_id: &str,
        device_id: &str,
        prekeys: &[OneTimePreKeyInput],
    ) -> Result<RegisterPreKeysResponse, ServiceError> {
        let mut registered = 0;

        for prekey in prekeys {
            let stored = StoredPreKey {
                id: prekey.id,
                public_key: prekey.public_key.clone(),
                signature: None,
            };

            self.storage.store_one_time_prekey(
                user_id,
                device_id,
                prekey.id,
                &serde_json::to_vec(&stored)?,
            )?;

            registered += 1;
        }

        let total = self
            .storage
            .get_one_time_prekey_count(user_id, device_id)?;

        info!(
            "Registered {} prekeys for user {}, total: {}",
            registered, user_id, total
        );

        Ok(RegisterPreKeysResponse {
            registered,
            total_count: total,
        })
    }

    /// Get prekey bundle
    pub async fn get_prekey_bundle(
        &self,
        user_id: &str,
        device_id: Option<&str>,
    ) -> Result<PreKeyBundleResponse, ServiceError> {
        // Get identity
        let identity_data = self
            .storage
            .get_identity(user_id)?
            .ok_or_else(|| ServiceError::NotFound(format!("User {} not found", user_id)))?;

        let identity: StoredIdentity = serde_json::from_slice(&identity_data)?;

        // Get device
        let devices = self.storage.get_devices(user_id)?;
        if devices.is_empty() {
            return Err(ServiceError::NotFound("No devices found".to_string()));
        }

        let device: StoredDevice = if let Some(did) = device_id {
            devices
                .iter()
                .find_map(|d| {
                    serde_json::from_slice::<StoredDevice>(d)
                        .ok()
                        .filter(|dev| dev.device_id == did)
                })
                .ok_or_else(|| ServiceError::NotFound(format!("Device {} not found", did)))?
        } else {
            serde_json::from_slice(&devices[0])?
        };

        // Get signed prekey
        let signed_prekey_data = self
            .storage
            .get_signed_prekey(user_id, &device.device_id)?
            .ok_or_else(|| ServiceError::NotFound("Signed prekey not found".to_string()))?;

        let signed_prekey: StoredPreKey = serde_json::from_slice(&signed_prekey_data)?;

        // Consume one-time prekey
        let one_time_prekey = self
            .storage
            .consume_one_time_prekey(user_id, &device.device_id)?
            .map(|(id, data)| {
                serde_json::from_slice::<StoredPreKey>(&data)
                    .ok()
                    .map(|p| OneTimePreKeyResponse {
                        id: p.id,
                        public_key: p.public_key,
                    })
            })
            .flatten();

        // Get identity public key
        let secret_bytes = hex::decode(&identity.identity_key_secret)?;
        let secret_arr: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| ServiceError::Crypto("Invalid key length".to_string()))?;
        let keypair = IdentityKeyPair::from_secret_bytes(&secret_arr);
        let public_key = keypair.public_key();

        Ok(PreKeyBundleResponse {
            user_id: user_id.to_string(),
            device_id: device.device_id,
            identity_key: hex::encode(public_key.signing_key_bytes()),
            signed_prekey: SignedPreKeyResponse {
                id: signed_prekey.id,
                public_key: signed_prekey.public_key,
                signature: signed_prekey.signature.unwrap_or_default(),
            },
            one_time_prekey,
        })
    }
}
