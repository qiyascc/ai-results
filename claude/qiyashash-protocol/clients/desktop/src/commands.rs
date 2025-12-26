//! Commands for Tauri integration
//!
//! These commands can be exposed to the frontend via Tauri.

use serde::{Deserialize, Serialize};

use qiyashash_core::types::UserId;

use crate::app::{App, AppError, ConversationInfo, Result};
use crate::state::AppState;

/// Initialize the application
pub async fn initialize(app: &mut App) -> Result<InitializeResult> {
    app.initialize().await?;
    
    Ok(InitializeResult {
        user_id: app.user_id().map(|u| u.to_string()),
        fingerprint: app.fingerprint(),
    })
}

/// Get current state
pub fn get_state(app: &App) -> AppState {
    app.state()
}

/// Send a message
pub async fn send_message(
    app: &App,
    recipient: &str,
    content: &str,
) -> Result<SendMessageResult> {
    let recipient_id = UserId::from_string(recipient);
    let message = app.send_message(&recipient_id, content).await?;
    
    Ok(SendMessageResult {
        message_id: message.id.to_string(),
        timestamp: message.created_at.as_millis(),
    })
}

/// Get conversation messages
pub fn get_conversation(
    app: &App,
    with_user: &str,
    limit: usize,
) -> Result<Vec<MessageInfo>> {
    let user_id = UserId::from_string(with_user);
    let messages = app.get_conversation(&user_id, limit)?;
    
    Ok(messages.into_iter().map(|m| MessageInfo {
        id: m.id.to_string(),
        sender_id: m.sender_id.to_string(),
        recipient_id: m.recipient_id.to_string(),
        content: m.content_as_string(),
        timestamp: m.created_at.as_millis(),
        status: format!("{:?}", m.status),
    }).collect())
}

/// Get all conversations
pub fn get_conversations(app: &App) -> Result<Vec<ConversationInfo>> {
    app.get_conversations()
}

/// Delete a message
pub fn delete_message(
    app: &App,
    message_id: &str,
    for_everyone: bool,
) -> Result<()> {
    app.delete_message(message_id, for_everyone)
}

/// Mark conversation as read
pub fn mark_as_read(app: &App, with_user: &str) -> Result<()> {
    let user_id = UserId::from_string(with_user);
    app.mark_as_read(&user_id)
}

/// Get fingerprint for verification
pub fn get_fingerprint(app: &App) -> Option<String> {
    app.fingerprint()
}

/// Shutdown
pub async fn shutdown(app: &App) -> Result<()> {
    app.shutdown().await
}

// Result types

/// Result of initialization
#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub user_id: Option<String>,
    pub fingerprint: Option<String>,
}

/// Result of sending message
#[derive(Debug, Serialize)]
pub struct SendMessageResult {
    pub message_id: String,
    pub timestamp: i64,
}

/// Message information for frontend
#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub content: Option<String>,
    pub timestamp: i64,
    pub status: String,
}

/// Contact information
#[derive(Debug, Serialize)]
pub struct ContactInfo {
    pub user_id: String,
    pub display_name: Option<String>,
    pub fingerprint: String,
    pub is_verified: bool,
}

/// Verification info
#[derive(Debug, Serialize)]
pub struct VerificationInfo {
    pub safety_number: String,
    pub our_fingerprint: String,
    pub their_fingerprint: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_initialize_command() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path().to_str().unwrap()).unwrap();
        
        let result = initialize(&mut app).await.unwrap();
        
        assert!(result.user_id.is_some());
        assert!(result.fingerprint.is_some());
    }

    #[tokio::test]
    async fn test_send_message_command() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path().to_str().unwrap()).unwrap();
        initialize(&mut app).await.unwrap();
        
        let result = send_message(&app, "recipient-123", "Hello!").await.unwrap();
        
        assert!(!result.message_id.is_empty());
    }
}
