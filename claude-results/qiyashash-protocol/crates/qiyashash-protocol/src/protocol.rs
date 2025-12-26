//! Protocol message types
//!
//! Defines all message types used in the QiyasHash protocol.

use serde::{Deserialize, Serialize};

use qiyashash_core::message::{MessageEnvelope, MessageReceipt, TypingIndicator, MessageDeletion};
use qiyashash_core::types::{DeviceId, Timestamp, UserId};

/// Protocol message wrapper
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Protocol version
    pub version: u32,
    /// Message type
    pub message_type: ProtocolMessageType,
    /// Sender user ID
    pub sender_id: UserId,
    /// Sender device ID
    pub sender_device_id: DeviceId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Message ID for deduplication
    pub message_id: String,
}

impl ProtocolMessage {
    /// Create a new protocol message
    pub fn new(
        message_type: ProtocolMessageType,
        sender_id: UserId,
        sender_device_id: DeviceId,
    ) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            message_type,
            sender_id,
            sender_device_id,
            timestamp: Timestamp::now(),
            message_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Protocol message type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProtocolMessageType {
    /// Pre-key bundle request
    PreKeyBundleRequest(PreKeyBundleRequest),
    /// Pre-key bundle response
    PreKeyBundleResponse(PreKeyBundleResponse),
    /// Encrypted content message
    EncryptedMessage(MessageEnvelope),
    /// Delivery receipt
    DeliveryReceipt(MessageReceipt),
    /// Read receipt
    ReadReceipt(MessageReceipt),
    /// Typing indicator
    Typing(TypingIndicator),
    /// Message deletion
    Deletion(MessageDeletion),
    /// Session reset request
    SessionReset(SessionResetRequest),
    /// Identity key update notification
    IdentityKeyUpdate(IdentityKeyUpdate),
    /// Device list update
    DeviceListUpdate(DeviceListUpdate),
    /// Prekey replenishment
    PrekeyReplenish(PrekeyReplenish),
    /// Sync message (for multi-device)
    SyncMessage(SyncMessage),
    /// Group message (future)
    GroupMessage(GroupMessage),
    /// Presence update
    Presence(PresenceUpdate),
    /// Error response
    Error(ProtocolErrorMessage),
}

/// Pre-key bundle request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreKeyBundleRequest {
    /// Target user ID
    pub target_user_id: UserId,
    /// Target device ID (optional, request for all devices if None)
    pub target_device_id: Option<DeviceId>,
}

/// Pre-key bundle response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreKeyBundleResponse {
    /// User ID
    pub user_id: UserId,
    /// Bundles for each device
    pub bundles: Vec<DevicePreKeyBundle>,
}

/// Pre-key bundle for a device
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevicePreKeyBundle {
    /// Device ID
    pub device_id: DeviceId,
    /// Registration ID
    pub registration_id: u32,
    /// Identity public key
    #[serde(with = "hex::serde")]
    pub identity_key: [u8; 32],
    /// Signed pre-key ID
    pub signed_prekey_id: u32,
    /// Signed pre-key public
    #[serde(with = "hex::serde")]
    pub signed_prekey: [u8; 32],
    /// Signed pre-key signature
    #[serde(with = "hex::serde")]
    pub signed_prekey_signature: [u8; 64],
    /// One-time pre-key ID (optional)
    pub one_time_prekey_id: Option<u32>,
    /// One-time pre-key public (optional)
    #[serde(with = "option_hex")]
    pub one_time_prekey: Option<[u8; 32]>,
}

/// Session reset request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionResetRequest {
    /// Target user ID
    pub target_user_id: UserId,
    /// Target device ID
    pub target_device_id: DeviceId,
    /// Reason for reset
    pub reason: SessionResetReason,
}

/// Reason for session reset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionResetReason {
    /// User requested reset
    UserRequested,
    /// Decryption failure
    DecryptionFailure,
    /// Identity key changed
    IdentityKeyChanged,
    /// Session expired
    SessionExpired,
    /// Protocol upgrade
    ProtocolUpgrade,
}

/// Identity key update notification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityKeyUpdate {
    /// New identity public key
    #[serde(with = "hex::serde")]
    pub new_identity_key: [u8; 32],
    /// Signature by old key
    #[serde(with = "hex::serde")]
    pub old_key_signature: [u8; 64],
    /// Signature by new key
    #[serde(with = "hex::serde")]
    pub new_key_signature: [u8; 64],
    /// Reason for update
    pub reason: IdentityUpdateReason,
}

/// Reason for identity update
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityUpdateReason {
    /// Regular rotation
    Rotation,
    /// Key compromise suspected
    Compromise,
    /// User reinstalled app
    Reinstall,
}

/// Device list update
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceListUpdate {
    /// Updated device list
    pub devices: Vec<DeviceInfo>,
}

/// Device information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID
    pub device_id: DeviceId,
    /// Device name
    pub name: String,
    /// Registration ID
    pub registration_id: u32,
    /// Is primary device
    pub is_primary: bool,
}

/// Prekey replenishment notification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrekeyReplenish {
    /// New one-time prekeys
    pub new_prekeys: Vec<OneTimePreKeyInfo>,
}

/// One-time prekey info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OneTimePreKeyInfo {
    /// Key ID
    pub id: u32,
    /// Public key
    #[serde(with = "hex::serde")]
    pub public_key: [u8; 32],
}

/// Sync message for multi-device
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncMessage {
    /// Sync type
    pub sync_type: SyncType,
    /// Encrypted sync data
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
}

/// Sync type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncType {
    /// Sent message sync
    SentMessage,
    /// Read receipt sync
    ReadReceipt,
    /// Contact sync
    Contacts,
    /// Group sync
    Groups,
    /// Settings sync
    Settings,
    /// Blocked users sync
    Blocked,
}

/// Group message (placeholder for future)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMessage {
    /// Group ID
    pub group_id: String,
    /// Encrypted content
    #[serde(with = "base64_serde")]
    pub content: Vec<u8>,
}

/// Presence update
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresenceUpdate {
    /// Online status
    pub is_online: bool,
    /// Last seen timestamp (if sharing)
    pub last_seen: Option<Timestamp>,
    /// Custom status message
    pub status_message: Option<String>,
}

/// Protocol error message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolErrorMessage {
    /// Error code
    pub code: u32,
    /// Error message
    pub message: String,
    /// Related message ID
    pub related_message_id: Option<String>,
}

impl ProtocolErrorMessage {
    /// Session not found
    pub const SESSION_NOT_FOUND: u32 = 1001;
    /// Invalid message
    pub const INVALID_MESSAGE: u32 = 1002;
    /// Decryption failed
    pub const DECRYPTION_FAILED: u32 = 1003;
    /// Rate limited
    pub const RATE_LIMITED: u32 = 1004;
    /// Internal error
    pub const INTERNAL_ERROR: u32 = 5000;

    /// Create error message
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            related_message_id: None,
        }
    }

    /// With related message ID
    pub fn with_related_message(mut self, id: impl Into<String>) -> Self {
        self.related_message_id = Some(id.into());
        self
    }
}

// Serde helper for Option<[u8; 32]>
mod option_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<[u8; 32]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => serializer.serialize_some(&hex::encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 32]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => {
                let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
                if bytes.len() != 32 {
                    return Err(serde::de::Error::custom("Invalid key length"));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

// Serde helper for base64
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
    fn test_protocol_message_creation() {
        let msg = ProtocolMessage::new(
            ProtocolMessageType::Presence(PresenceUpdate {
                is_online: true,
                last_seen: None,
                status_message: None,
            }),
            UserId::from_string("test-user"),
            DeviceId::new(),
        );

        assert_eq!(msg.version, crate::PROTOCOL_VERSION);
    }

    #[test]
    fn test_error_message() {
        let err = ProtocolErrorMessage::new(
            ProtocolErrorMessage::SESSION_NOT_FOUND,
            "Session not found",
        )
        .with_related_message("msg-123");

        assert_eq!(err.code, 1001);
        assert_eq!(err.related_message_id, Some("msg-123".to_string()));
    }

    #[test]
    fn test_serialization() {
        let bundle = DevicePreKeyBundle {
            device_id: DeviceId::new(),
            registration_id: 12345,
            identity_key: [0x01; 32],
            signed_prekey_id: 1,
            signed_prekey: [0x02; 32],
            signed_prekey_signature: [0x03; 64],
            one_time_prekey_id: Some(1),
            one_time_prekey: Some([0x04; 32]),
        };

        let json = serde_json::to_string(&bundle).unwrap();
        let restored: DevicePreKeyBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(bundle.registration_id, restored.registration_id);
        assert_eq!(bundle.identity_key, restored.identity_key);
    }
}
