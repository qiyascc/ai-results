//! Message types for QiyasHash protocol

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::types::{ContentType, DeviceId, Timestamp, UserId};

/// Unique message identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(String);

impl MessageId {
    /// Create a new random message ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message delivery status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    /// Message is being prepared
    Pending,
    /// Message sent to relay
    Sent,
    /// Message delivered to recipient's device
    Delivered,
    /// Message read by recipient
    Read,
    /// Message failed to send
    Failed,
    /// Message deleted
    Deleted,
}

impl Default for MessageStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A plaintext message (before encryption)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: MessageId,
    /// Sender's user ID
    pub sender_id: UserId,
    /// Sender's device ID
    pub sender_device_id: DeviceId,
    /// Recipient's user ID
    pub recipient_id: UserId,
    /// Content type
    pub content_type: ContentType,
    /// Message content (plaintext)
    pub content: Vec<u8>,
    /// Optional quoted message ID
    pub quote_id: Option<MessageId>,
    /// Attachments
    pub attachments: Vec<Attachment>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Expiration time (for disappearing messages)
    pub expires_at: Option<Timestamp>,
    /// Message status
    pub status: MessageStatus,
}

impl Message {
    /// Create a new text message
    pub fn text(
        sender_id: UserId,
        sender_device_id: DeviceId,
        recipient_id: UserId,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: MessageId::new(),
            sender_id,
            sender_device_id,
            recipient_id,
            content_type: ContentType::Text,
            content: content.into().into_bytes(),
            quote_id: None,
            attachments: Vec::new(),
            created_at: Timestamp::now(),
            expires_at: None,
            status: MessageStatus::Pending,
        }
    }

    /// Get content as string (for text messages)
    pub fn content_as_string(&self) -> Option<String> {
        if matches!(self.content_type, ContentType::Text) {
            String::from_utf8(self.content.clone()).ok()
        } else {
            None
        }
    }

    /// Set expiration for disappearing message
    pub fn with_expiration(mut self, duration_secs: i64) -> Self {
        self.expires_at = Some(Timestamp::from_millis(
            self.created_at.as_millis() + duration_secs * 1000,
        ));
        self
    }

    /// Add a quote reference
    pub fn with_quote(mut self, quote_id: MessageId) -> Self {
        self.quote_id = Some(quote_id);
        self
    }

    /// Add an attachment
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Check if message is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Timestamp::now() > expires_at
        } else {
            false
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

/// Attachment metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    /// Unique attachment ID
    pub id: String,
    /// Content type
    pub content_type: ContentType,
    /// Size in bytes
    pub size: u64,
    /// Filename (optional)
    pub filename: Option<String>,
    /// Encryption key for attachment
    #[serde(with = "hex::serde")]
    pub key: [u8; 32],
    /// HMAC digest
    #[serde(with = "hex::serde")]
    pub digest: [u8; 32],
    /// Thumbnail (optional, for images/videos)
    pub thumbnail: Option<Thumbnail>,
}

/// Thumbnail for media attachments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thumbnail {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Encrypted thumbnail data
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
}

/// Encrypted message envelope (wire format)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Protocol version
    pub version: u32,
    /// Sender's identity key (for X3DH)
    #[serde(with = "hex::serde")]
    pub sender_identity_key: [u8; 32],
    /// Ephemeral key (for X3DH initial message)
    #[serde(with = "hex::serde")]
    pub ephemeral_key: Option<[u8; 32]>,
    /// One-time prekey ID used (for X3DH initial message)
    pub one_time_prekey_id: Option<u32>,
    /// Ratchet header
    pub ratchet_header: RatchetHeaderWire,
    /// Encrypted payload
    #[serde(with = "base64_serde")]
    pub ciphertext: Vec<u8>,
    /// Chain proof
    #[serde(with = "hex::serde")]
    pub chain_proof: [u8; 32],
    /// Timestamp hash (for metadata protection)
    #[serde(with = "hex::serde")]
    pub timestamp_hash: [u8; 32],
}

/// Wire format for ratchet header
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetHeaderWire {
    /// Sender's current DH ratchet public key
    #[serde(with = "hex::serde")]
    pub dh_public: [u8; 32],
    /// Message number in sending chain
    pub message_number: u32,
    /// Previous chain length
    pub previous_chain_length: u32,
}

impl MessageEnvelope {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string(self).map_err(Into::into)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(Into::into)
    }
}

/// Receipt for message delivery/read status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageReceipt {
    /// Message ID this receipt is for
    pub message_id: MessageId,
    /// Receipt type
    pub receipt_type: ReceiptType,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Type of receipt
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptType {
    /// Message was delivered
    Delivered,
    /// Message was read
    Read,
}

/// Typing indicator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypingIndicator {
    /// Sender's user ID
    pub sender_id: UserId,
    /// Whether currently typing
    pub is_typing: bool,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Message deletion request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDeletion {
    /// Message ID to delete
    pub message_id: MessageId,
    /// Who initiated deletion
    pub deleted_by: UserId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Whether to delete for everyone
    pub delete_for_everyone: bool,
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
    fn test_message_creation() {
        let msg = Message::text(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            "Hello, World!",
        );

        assert_eq!(msg.content_as_string(), Some("Hello, World!".to_string()));
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_message_expiration() {
        let msg = Message::text(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            "Disappearing",
        )
        .with_expiration(0); // Expires immediately

        // Give it a moment
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(msg.is_expired());
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::text(
            UserId::new(),
            DeviceId::new(),
            UserId::new(),
            "Test message",
        );

        let bytes = msg.to_bytes().unwrap();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert_eq!(msg.id, restored.id);
        assert_eq!(msg.content, restored.content);
    }

    #[test]
    fn test_envelope_serialization() {
        let envelope = MessageEnvelope {
            version: 1,
            sender_identity_key: [0x42; 32],
            ephemeral_key: Some([0x43; 32]),
            one_time_prekey_id: Some(1),
            ratchet_header: RatchetHeaderWire {
                dh_public: [0x44; 32],
                message_number: 0,
                previous_chain_length: 0,
            },
            ciphertext: vec![0x01, 0x02, 0x03],
            chain_proof: [0x45; 32],
            timestamp_hash: [0x46; 32],
        };

        let json = envelope.to_json().unwrap();
        let restored = MessageEnvelope::from_json(&json).unwrap();

        assert_eq!(envelope.version, restored.version);
        assert_eq!(envelope.ciphertext, restored.ciphertext);
    }
}
