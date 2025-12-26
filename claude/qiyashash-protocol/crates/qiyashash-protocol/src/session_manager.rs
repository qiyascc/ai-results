//! Session management for encrypted communications
//!
//! Manages the lifecycle of encrypted sessions including establishment,
//! key ratcheting, and cleanup.

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};

use qiyashash_core::session::{Session, SessionId, SessionRecord, SessionState};
use qiyashash_core::storage::{SessionStore, IdentityStore, PreKeyStore};
use qiyashash_core::types::{DeviceId, Fingerprint, UserId};
use qiyashash_crypto::identity::{Identity, IdentityKeyPair, IdentityPublicKey};
use qiyashash_crypto::ratchet::DoubleRatchet;
use qiyashash_crypto::x3dh::{PreKeyManager, X3DHKeyAgreement};
use qiyashash_crypto::keys::PreKeyBundle;
use qiyashash_crypto::chain::ChainState;

use crate::config::ClientConfig;
use crate::error::{ProtocolError, Result};
use crate::protocol::DevicePreKeyBundle;

/// Active session with ratchet state
struct ActiveSession {
    /// Session metadata
    session: Session,
    /// Double ratchet for encryption
    ratchet: DoubleRatchet,
    /// Chain state for ordering
    chain: ChainState,
}

/// Session manager
pub struct SessionManager {
    /// Configuration
    config: ClientConfig,
    /// Our identity
    identity: Identity,
    /// Our device ID
    device_id: DeviceId,
    /// Pre-key manager
    prekey_manager: PreKeyManager,
    /// Active sessions (in memory)
    active_sessions: RwLock<HashMap<SessionId, ActiveSession>>,
    /// Storage backend
    storage: Arc<dyn SessionStore + Send + Sync>,
    /// Identity storage
    identity_storage: Arc<dyn IdentityStore + Send + Sync>,
    /// Prekey storage
    prekey_storage: Arc<dyn PreKeyStore + Send + Sync>,
}

impl SessionManager {
    /// Create a new session manager
    pub async fn new(
        config: ClientConfig,
        identity: Identity,
        device_id: DeviceId,
        storage: Arc<dyn SessionStore + Send + Sync>,
        identity_storage: Arc<dyn IdentityStore + Send + Sync>,
        prekey_storage: Arc<dyn PreKeyStore + Send + Sync>,
    ) -> Result<Self> {
        let prekey_manager = PreKeyManager::new(identity.key_pair.clone());

        let manager = Self {
            config,
            identity,
            device_id,
            prekey_manager,
            active_sessions: RwLock::new(HashMap::new()),
            storage,
            identity_storage,
            prekey_storage,
        };

        // Load active sessions from storage
        manager.load_active_sessions().await?;

        Ok(manager)
    }

    /// Load active sessions from storage
    async fn load_active_sessions(&self) -> Result<()> {
        let records = self.storage.get_active_sessions().await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        let mut sessions = self.active_sessions.write();

        for record in records {
            match self.restore_session(&record) {
                Ok((ratchet, chain)) => {
                    sessions.insert(record.session.id.clone(), ActiveSession {
                        session: record.session,
                        ratchet,
                        chain,
                    });
                }
                Err(e) => {
                    warn!("Failed to restore session {}: {}", record.session.id, e);
                }
            }
        }

        info!("Loaded {} active sessions", sessions.len());
        Ok(())
    }

    /// Restore ratchet and chain state from serialized data
    fn restore_session(&self, record: &SessionRecord) -> Result<(DoubleRatchet, ChainState)> {
        // In production, deserialize the actual ratchet state
        // For now, this is a placeholder
        Err(ProtocolError::Internal("Session restoration not implemented".to_string()))
    }

    /// Get our identity public key
    pub fn identity_public_key(&self) -> IdentityPublicKey {
        self.identity.key_pair.public_key()
    }

    /// Get our fingerprint
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::from_bytes(self.identity.fingerprint)
    }

    /// Get our prekey bundle for publishing
    pub fn get_prekey_bundle(&self) -> PreKeyBundle {
        self.prekey_manager.get_bundle()
    }

    /// Generate more one-time prekeys
    pub fn generate_prekeys(&mut self, count: usize) {
        self.prekey_manager.generate_one_time_prekeys(count);
        info!("Generated {} new one-time prekeys", count);
    }

    /// Check if we need more prekeys
    pub fn needs_prekey_replenishment(&self) -> bool {
        // This would check the prekey store
        // For now, always return false
        false
    }

    /// Establish a new session with a user
    pub async fn establish_session(
        &mut self,
        their_user_id: &UserId,
        their_device_id: &DeviceId,
        their_bundle: &DevicePreKeyBundle,
    ) -> Result<SessionId> {
        debug!("Establishing session with {} device {}", their_user_id, their_device_id);

        // Convert to crypto bundle format
        let bundle = self.convert_bundle(their_bundle)?;

        // Perform X3DH key agreement
        let (shared_secret, ephemeral_public, opk_id) = 
            X3DHKeyAgreement::initiate(&self.identity.key_pair, &bundle)
                .map_err(|e| ProtocolError::KeyExchangeFailed(e.to_string()))?;

        // Create Double Ratchet session
        let their_spk = x25519_dalek::PublicKey::from(their_bundle.signed_prekey);
        let session_id_bytes = self.compute_session_id(shared_secret.secret());
        
        let ratchet = DoubleRatchet::new_initiator(
            shared_secret.secret(),
            &their_spk,
            session_id_bytes,
        ).map_err(|e| ProtocolError::KeyExchangeFailed(e.to_string()))?;

        // Create chain state
        let chain = ChainState::from_shared_secret(shared_secret.secret());

        // Create session metadata
        let session = Session::new(
            UserId::from_fingerprint(&self.identity.fingerprint),
            self.device_id.clone(),
            their_user_id.clone(),
            their_device_id.clone(),
            self.fingerprint(),
            Fingerprint::from_bytes(their_bundle.identity_key),
            Fingerprint::from_bytes(session_id_bytes),
        );

        let session_id = session.id.clone();

        // Store in memory
        {
            let mut sessions = self.active_sessions.write();
            sessions.insert(session_id.clone(), ActiveSession {
                session: session.clone(),
                ratchet,
                chain,
            });
        }

        // Persist to storage
        let record = SessionRecord {
            session,
            ratchet_state: Vec::new(), // Would serialize ratchet
            chain_state: Vec::new(),   // Would serialize chain
        };
        self.storage.save_session(&record).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        // Save their identity key
        self.identity_storage.save_remote_identity(their_user_id, their_bundle.identity_key).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        info!("Established session {} with {} device {}", 
            session_id, their_user_id, their_device_id);

        Ok(session_id)
    }

    /// Accept an incoming session
    pub async fn accept_session(
        &mut self,
        their_user_id: &UserId,
        their_device_id: &DeviceId,
        their_identity_key: [u8; 32],
        their_ephemeral_key: [u8; 32],
        used_opk_id: Option<u32>,
    ) -> Result<SessionId> {
        debug!("Accepting session from {} device {}", their_user_id, their_device_id);

        // Verify their identity
        let is_trusted = self.identity_storage.is_trusted_identity(their_user_id, &their_identity_key).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        if !is_trusted {
            // Check if this is a new identity (TOFU)
            let existing = self.identity_storage.get_remote_identity(their_user_id).await
                .map_err(|e| ProtocolError::Storage(e.to_string()))?;

            if existing.is_some() {
                return Err(ProtocolError::UntrustedIdentity(their_user_id.to_string()));
            }
        }

        // Perform X3DH as responder
        let their_identity = IdentityPublicKey::from_bytes(&their_identity_key)
            .map_err(|e| ProtocolError::KeyExchangeFailed(e.to_string()))?;
        
        let ephemeral_key = qiyashash_crypto::keys::PublicKeyBytes::from(their_ephemeral_key);

        let shared_secret = X3DHKeyAgreement::respond(
            &mut self.prekey_manager,
            &their_identity,
            &ephemeral_key,
            used_opk_id,
        ).map_err(|e| ProtocolError::KeyExchangeFailed(e.to_string()))?;

        // Get our signed prekey for the ratchet
        let our_spk_secret = self.prekey_manager.signed_prekey_secret().clone();
        let session_id_bytes = self.compute_session_id(shared_secret.secret());

        // Create Double Ratchet session as responder
        let ratchet = DoubleRatchet::new_responder(
            shared_secret.secret(),
            our_spk_secret,
            session_id_bytes,
        );

        // Create chain state
        let chain = ChainState::from_shared_secret(shared_secret.secret());

        // Create session metadata
        let mut session = Session::new(
            UserId::from_fingerprint(&self.identity.fingerprint),
            self.device_id.clone(),
            their_user_id.clone(),
            their_device_id.clone(),
            self.fingerprint(),
            Fingerprint::from_bytes(their_identity_key),
            Fingerprint::from_bytes(session_id_bytes),
        );
        session.activate();

        let session_id = session.id.clone();

        // Store in memory
        {
            let mut sessions = self.active_sessions.write();
            sessions.insert(session_id.clone(), ActiveSession {
                session: session.clone(),
                ratchet,
                chain,
            });
        }

        // Persist to storage
        let record = SessionRecord {
            session,
            ratchet_state: Vec::new(),
            chain_state: Vec::new(),
        };
        self.storage.save_session(&record).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        // Save their identity key
        self.identity_storage.save_remote_identity(their_user_id, their_identity_key).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        info!("Accepted session {} from {} device {}", 
            session_id, their_user_id, their_device_id);

        Ok(session_id)
    }

    /// Get session by user and device
    pub fn get_session(
        &self,
        their_user_id: &UserId,
        their_device_id: &DeviceId,
    ) -> Option<SessionId> {
        self.active_sessions.read()
            .values()
            .find(|s| {
                s.session.their_user_id == *their_user_id 
                    && s.session.their_device_id == *their_device_id
            })
            .map(|s| s.session.id.clone())
    }

    /// Check if session exists
    pub fn has_session(
        &self,
        their_user_id: &UserId,
        their_device_id: &DeviceId,
    ) -> bool {
        self.get_session(their_user_id, their_device_id).is_some()
    }

    /// Encrypt message for a session
    pub fn encrypt(
        &self,
        session_id: &SessionId,
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, [u8; 32], [u8; 32])> {
        let mut sessions = self.active_sessions.write();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ProtocolError::SessionNotFound(session_id.to_string()))?;

        // Encrypt with ratchet
        let ratchet_msg = session.ratchet.encrypt(plaintext)
            .map_err(|e| ProtocolError::Crypto(e))?;

        // Update chain state
        let msg_hash = qiyashash_crypto::chain::compute_message_hash(
            &ratchet_msg.payload.ciphertext,
            &ratchet_msg.header.to_bytes(),
        );
        let chain_link = session.chain.add_message(&msg_hash);

        // Serialize ratchet message
        let ciphertext = bincode::serialize(&ratchet_msg)
            .map_err(|e| ProtocolError::Internal(e.to_string()))?;

        // Update session
        session.session.increment_message_count();
        session.session.update_ratchet_hash(session.ratchet.current_ratchet_public()
            .map(|p| *p.as_bytes())
            .unwrap_or([0; 32]));

        Ok((ciphertext, chain_link.state, msg_hash))
    }

    /// Decrypt message for a session
    pub fn decrypt(
        &self,
        session_id: &SessionId,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        let mut sessions = self.active_sessions.write();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ProtocolError::SessionNotFound(session_id.to_string()))?;

        // Deserialize ratchet message
        let ratchet_msg: qiyashash_crypto::ratchet::RatchetMessage = bincode::deserialize(ciphertext)
            .map_err(|e| ProtocolError::InvalidMessage(e.to_string()))?;

        // Decrypt with ratchet
        let plaintext = session.ratchet.decrypt(&ratchet_msg)
            .map_err(|e| ProtocolError::DecryptionFailed(e.to_string()))?;

        // Update session
        session.session.increment_message_count();
        session.session.update_ratchet_hash(session.ratchet.current_ratchet_public()
            .map(|p| *p.as_bytes())
            .unwrap_or([0; 32]));

        Ok(plaintext)
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &SessionId) -> Result<()> {
        {
            let mut sessions = self.active_sessions.write();
            if let Some(mut session) = sessions.remove(session_id) {
                session.session.close();
                // Could persist the closed state
            }
        }

        self.storage.delete_session(session_id).await
            .map_err(|e| ProtocolError::Storage(e.to_string()))?;

        info!("Closed session {}", session_id);
        Ok(())
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.active_sessions.read().len()
    }

    /// Get sessions needing rekey
    pub fn sessions_needing_rekey(&self) -> Vec<SessionId> {
        self.active_sessions.read()
            .values()
            .filter(|s| s.session.needs_rekey())
            .map(|s| s.session.id.clone())
            .collect()
    }

    // Helper functions

    fn convert_bundle(&self, bundle: &DevicePreKeyBundle) -> Result<PreKeyBundle> {
        Ok(PreKeyBundle {
            identity_key: bundle.identity_key,
            signed_prekey: qiyashash_crypto::keys::SignedPreKey {
                id: bundle.signed_prekey_id,
                public_key: qiyashash_crypto::keys::PublicKeyBytes::from(bundle.signed_prekey),
                signature: bundle.signed_prekey_signature,
                timestamp: chrono::Utc::now().timestamp(),
            },
            one_time_prekey: bundle.one_time_prekey_id.map(|id| {
                qiyashash_crypto::keys::OneTimePreKey {
                    id,
                    public_key: qiyashash_crypto::keys::PublicKeyBytes::from(
                        bundle.one_time_prekey.unwrap_or([0; 32])
                    ),
                }
            }),
        })
    }

    fn compute_session_id(&self, shared_secret: &[u8; 32]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"QiyasHash_SessionId_v1");
        hasher.update(shared_secret);
        hasher.update(&self.identity.fingerprint);
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
