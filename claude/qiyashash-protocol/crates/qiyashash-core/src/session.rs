//! Session management for QiyasHash
//!
//! A session represents an encrypted communication channel between two users.
//! Sessions are established using X3DH and maintained using the Double Ratchet.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::types::{DeviceId, Fingerprint, SafetyNumber, Timestamp, UserId};

/// Unique session identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new random session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from user IDs (deterministic)
    pub fn from_users(user1: &UserId, user2: &UserId, device1: &DeviceId, device2: &DeviceId) -> Self {
        use sha2::{Digest, Sha256};

        // Sort to ensure consistent ID regardless of who initiates
        let mut parts = [
            user1.as_str(),
            user2.as_str(),
            device1.as_str(),
            device2.as_str(),
        ];
        parts.sort();

        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part.as_bytes());
        }
        let hash = hasher.finalize();

        Self(hex::encode(&hash[..16]))
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is being established
    Initiating,
    /// Waiting for response from other party
    AwaitingResponse,
    /// Session is active
    Active,
    /// Session is stale (no recent activity)
    Stale,
    /// Session needs re-keying
    NeedsRekey,
    /// Session is closed
    Closed,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Initiating
    }
}

/// A session between two users
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: SessionId,
    /// Our user ID
    pub our_user_id: UserId,
    /// Our device ID
    pub our_device_id: DeviceId,
    /// Their user ID
    pub their_user_id: UserId,
    /// Their device ID
    pub their_device_id: DeviceId,
    /// Session state
    pub state: SessionState,
    /// Our identity fingerprint
    pub our_fingerprint: Fingerprint,
    /// Their identity fingerprint
    pub their_fingerprint: Fingerprint,
    /// Safety number for verification
    pub safety_number: SafetyNumber,
    /// Whether safety number has been verified
    pub is_verified: bool,
    /// Session creation time
    pub created_at: Timestamp,
    /// Last activity time
    pub last_activity_at: Timestamp,
    /// Message count
    pub message_count: u64,
    /// Root key fingerprint (for debugging)
    pub root_key_fingerprint: Fingerprint,
    /// Ratchet state hash (for sync verification)
    #[serde(with = "hex::serde")]
    pub ratchet_state_hash: [u8; 32],
}

impl Session {
    /// Create a new session
    pub fn new(
        our_user_id: UserId,
        our_device_id: DeviceId,
        their_user_id: UserId,
        their_device_id: DeviceId,
        our_fingerprint: Fingerprint,
        their_fingerprint: Fingerprint,
        root_key_fingerprint: Fingerprint,
    ) -> Self {
        let id = SessionId::from_users(
            &our_user_id,
            &their_user_id,
            &our_device_id,
            &their_device_id,
        );
        let safety_number = SafetyNumber::compute(&our_fingerprint, &their_fingerprint);

        Self {
            id,
            our_user_id,
            our_device_id,
            their_user_id,
            their_device_id,
            state: SessionState::Initiating,
            our_fingerprint,
            their_fingerprint,
            safety_number,
            is_verified: false,
            created_at: Timestamp::now(),
            last_activity_at: Timestamp::now(),
            message_count: 0,
            root_key_fingerprint,
            ratchet_state_hash: [0; 32],
        }
    }

    /// Mark session as active
    pub fn activate(&mut self) {
        self.state = SessionState::Active;
        self.update_activity();
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity_at = Timestamp::now();
    }

    /// Increment message count
    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
        self.update_activity();
    }

    /// Mark as verified
    pub fn verify(&mut self) {
        self.is_verified = true;
    }

    /// Check if session is stale (no activity in 30 days)
    pub fn is_stale(&self) -> bool {
        self.last_activity_at.is_expired(30 * 24 * 3600)
    }

    /// Close session
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
    }

    /// Check if session needs re-keying
    pub fn needs_rekey(&self) -> bool {
        // Re-key after 7 days or 1000 messages
        self.state == SessionState::NeedsRekey
            || self.last_activity_at.is_expired(7 * 24 * 3600)
            || self.message_count > 1000
    }

    /// Update ratchet state hash
    pub fn update_ratchet_hash(&mut self, hash: [u8; 32]) {
        self.ratchet_state_hash = hash;
    }
}

/// Session record for storage (with serialized ratchet state)
#[derive(Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Session metadata
    pub session: Session,
    /// Serialized ratchet state (encrypted)
    #[serde(with = "base64_serde")]
    pub ratchet_state: Vec<u8>,
    /// Chain state
    #[serde(with = "base64_serde")]
    pub chain_state: Vec<u8>,
}

/// Session key bundle for sharing with new devices
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionKeyBundle {
    /// Session ID
    pub session_id: SessionId,
    /// Encrypted session keys
    #[serde(with = "base64_serde")]
    pub encrypted_keys: Vec<u8>,
    /// Timestamp
    pub created_at: Timestamp,
}

/// Information for establishing a new session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInitInfo {
    /// Their user ID
    pub their_user_id: UserId,
    /// Their device ID
    pub their_device_id: DeviceId,
    /// Their identity key
    #[serde(with = "hex::serde")]
    pub their_identity_key: [u8; 32],
    /// Their signed prekey
    #[serde(with = "hex::serde")]
    pub their_signed_prekey: [u8; 32],
    /// Their signed prekey signature
    #[serde(with = "hex::serde")]
    pub their_signed_prekey_signature: [u8; 64],
    /// Their one-time prekey (if available)
    pub their_one_time_prekey: Option<OneTimePreKeyInfo>,
}

/// One-time prekey information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OneTimePreKeyInfo {
    /// Key ID
    pub id: u32,
    /// Public key
    #[serde(with = "hex::serde")]
    pub public_key: [u8; 32],
}

// Serde helper for base64 encoding
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_deterministic() {
        let user1 = UserId::from_string("user1");
        let user2 = UserId::from_string("user2");
        let device1 = DeviceId::from_string("device1");
        let device2 = DeviceId::from_string("device2");

        let id1 = SessionId::from_users(&user1, &user2, &device1, &device2);
        let id2 = SessionId::from_users(&user2, &user1, &device2, &device1);

        // Should be same regardless of order
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new(
            UserId::from_string("alice"),
            DeviceId::new(),
            UserId::from_string("bob"),
            DeviceId::new(),
            Fingerprint::from_bytes([0x01; 32]),
            Fingerprint::from_bytes([0x02; 32]),
            Fingerprint::from_bytes([0x03; 32]),
        );

        assert_eq!(session.state, SessionState::Initiating);
        assert!(!session.is_verified);
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_session_activity() {
        let mut session = Session::new(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            DeviceId::new(),
            Fingerprint::from_bytes([0x01; 32]),
            Fingerprint::from_bytes([0x02; 32]),
            Fingerprint::from_bytes([0x03; 32]),
        );

        session.activate();
        assert_eq!(session.state, SessionState::Active);

        session.increment_message_count();
        assert_eq!(session.message_count, 1);
    }

    #[test]
    fn test_safety_number_consistency() {
        let fp1 = Fingerprint::from_bytes([0x01; 32]);
        let fp2 = Fingerprint::from_bytes([0x02; 32]);

        let session1 = Session::new(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            DeviceId::new(),
            fp1.clone(),
            fp2.clone(),
            Fingerprint::from_bytes([0x00; 32]),
        );

        let session2 = Session::new(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            DeviceId::new(),
            fp2,
            fp1,
            Fingerprint::from_bytes([0x00; 32]),
        );

        // Safety numbers should match
        assert_eq!(
            session1.safety_number.number,
            session2.safety_number.number
        );
    }
}
