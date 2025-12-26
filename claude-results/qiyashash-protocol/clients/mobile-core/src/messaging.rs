//! Messaging types for mobile

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID
    pub id: String,
    /// Conversation ID
    pub conversation_id: String,
    /// Sender ID
    pub sender_id: String,
    /// Message content (plaintext after decryption)
    pub content: String,
    /// Message type
    pub message_type: MessageType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: MessageStatus,
    /// Reply to message ID (if any)
    pub reply_to: Option<String>,
}

/// Message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Image { url: String, thumbnail: Option<String> },
    File { name: String, size: u64, mime_type: String },
    Voice { duration_seconds: u32 },
    Location { latitude: f64, longitude: f64 },
    System { action: String },
}

impl ChatMessage {
    /// Create a new text message
    pub fn new_text(
        conversation_id: String,
        sender_id: String,
        content: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            sender_id,
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            status: MessageStatus::Pending,
            reply_to: None,
        }
    }
}

/// Conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Conversation ID
    pub id: String,
    /// Participant IDs
    pub participants: Vec<String>,
    /// Conversation title (for groups)
    pub title: Option<String>,
    /// Is group conversation
    pub is_group: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Last message preview
    pub last_message: Option<String>,
    /// Last message timestamp
    pub last_message_at: Option<DateTime<Utc>>,
    /// Unread count
    pub unread_count: u32,
}

impl Conversation {
    /// Create a direct conversation
    pub fn direct(participant1: String, participant2: String) -> Self {
        let mut participants = vec![participant1, participant2];
        participants.sort();
        let id = format!("dm:{}", participants.join(":"));
        
        Self {
            id,
            participants,
            title: None,
            is_group: false,
            created_at: Utc::now(),
            last_message: None,
            last_message_at: None,
            unread_count: 0,
        }
    }

    /// Create a group conversation
    pub fn group(title: String, participants: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            participants,
            title: Some(title),
            is_group: true,
            created_at: Utc::now(),
            last_message: None,
            last_message_at: None,
            unread_count: 0,
        }
    }
}
