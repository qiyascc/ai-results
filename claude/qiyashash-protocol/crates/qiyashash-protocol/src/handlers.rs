//! Protocol message handlers
//!
//! Handles incoming protocol messages and generates appropriate responses.

use tracing::{debug, info, warn};

use qiyashash_core::message::{MessageReceipt, ReceiptType, TypingIndicator, MessageDeletion};
use qiyashash_core::types::{DeviceId, Timestamp, UserId};

use crate::error::{ProtocolError, Result};
use crate::protocol::{
    DevicePreKeyBundle, PreKeyBundleRequest, PreKeyBundleResponse,
    ProtocolMessage, ProtocolMessageType, SessionResetRequest, SessionResetReason,
    IdentityKeyUpdate, DeviceListUpdate, SyncMessage,
};

/// Handler for pre-key bundle requests
pub struct PreKeyBundleHandler;

impl PreKeyBundleHandler {
    /// Handle a prekey bundle request
    pub fn handle(
        request: &PreKeyBundleRequest,
        our_bundles: Vec<DevicePreKeyBundle>,
    ) -> PreKeyBundleResponse {
        debug!("Handling prekey bundle request for {:?}", request.target_device_id);

        let bundles = match &request.target_device_id {
            Some(device_id) => {
                our_bundles
                    .into_iter()
                    .filter(|b| &b.device_id == device_id)
                    .collect()
            }
            None => our_bundles,
        };

        PreKeyBundleResponse {
            user_id: request.target_user_id.clone(),
            bundles,
        }
    }
}

/// Handler for delivery receipts
pub struct ReceiptHandler;

impl ReceiptHandler {
    /// Handle a delivery receipt
    pub fn handle_delivery(
        receipt: &MessageReceipt,
        on_delivered: impl FnOnce(&str, Timestamp),
    ) {
        debug!("Handling delivery receipt for message {}", receipt.message_id);
        on_delivered(receipt.message_id.as_str(), receipt.timestamp);
    }

    /// Handle a read receipt
    pub fn handle_read(
        receipt: &MessageReceipt,
        on_read: impl FnOnce(&str, Timestamp),
    ) {
        debug!("Handling read receipt for message {}", receipt.message_id);
        on_read(receipt.message_id.as_str(), receipt.timestamp);
    }

    /// Create a delivery receipt
    pub fn create_delivery_receipt(message_id: &str) -> MessageReceipt {
        MessageReceipt {
            message_id: qiyashash_core::message::MessageId::from_string(message_id),
            receipt_type: ReceiptType::Delivered,
            timestamp: Timestamp::now(),
        }
    }

    /// Create a read receipt
    pub fn create_read_receipt(message_id: &str) -> MessageReceipt {
        MessageReceipt {
            message_id: qiyashash_core::message::MessageId::from_string(message_id),
            receipt_type: ReceiptType::Read,
            timestamp: Timestamp::now(),
        }
    }
}

/// Handler for typing indicators
pub struct TypingHandler;

impl TypingHandler {
    /// Handle typing indicator
    pub fn handle(
        indicator: &TypingIndicator,
        on_typing: impl FnOnce(&UserId, bool),
    ) {
        debug!("Handling typing indicator from {}: {}", indicator.sender_id, indicator.is_typing);
        on_typing(&indicator.sender_id, indicator.is_typing);
    }

    /// Create typing started indicator
    pub fn create_typing_started(sender_id: UserId) -> TypingIndicator {
        TypingIndicator {
            sender_id,
            is_typing: true,
            timestamp: Timestamp::now(),
        }
    }

    /// Create typing stopped indicator
    pub fn create_typing_stopped(sender_id: UserId) -> TypingIndicator {
        TypingIndicator {
            sender_id,
            is_typing: false,
            timestamp: Timestamp::now(),
        }
    }
}

/// Handler for message deletion
pub struct DeletionHandler;

impl DeletionHandler {
    /// Handle message deletion request
    pub fn handle(
        deletion: &MessageDeletion,
        can_delete: impl Fn(&str, &UserId) -> bool,
        on_delete: impl FnOnce(&str, bool),
    ) -> Result<()> {
        debug!("Handling deletion request for message {}", deletion.message_id);

        // Check if deletion is allowed
        if !can_delete(deletion.message_id.as_str(), &deletion.deleted_by) {
            warn!("Deletion not allowed for message {}", deletion.message_id);
            return Err(ProtocolError::Internal("Deletion not allowed".to_string()));
        }

        on_delete(deletion.message_id.as_str(), deletion.delete_for_everyone);
        Ok(())
    }

    /// Create deletion request
    pub fn create_deletion(
        message_id: &str,
        deleted_by: UserId,
        delete_for_everyone: bool,
    ) -> MessageDeletion {
        MessageDeletion {
            message_id: qiyashash_core::message::MessageId::from_string(message_id),
            deleted_by,
            timestamp: Timestamp::now(),
            delete_for_everyone,
        }
    }
}

/// Handler for session reset
pub struct SessionResetHandler;

impl SessionResetHandler {
    /// Handle session reset request
    pub fn handle(
        request: &SessionResetRequest,
        on_reset: impl FnOnce(&UserId, &DeviceId, SessionResetReason),
    ) {
        info!(
            "Handling session reset for {} device {}: {:?}",
            request.target_user_id, request.target_device_id, request.reason
        );
        on_reset(&request.target_user_id, &request.target_device_id, request.reason);
    }

    /// Create session reset request
    pub fn create_reset(
        target_user_id: UserId,
        target_device_id: DeviceId,
        reason: SessionResetReason,
    ) -> SessionResetRequest {
        SessionResetRequest {
            target_user_id,
            target_device_id,
            reason,
        }
    }
}

/// Handler for identity key updates
pub struct IdentityKeyHandler;

impl IdentityKeyHandler {
    /// Verify identity key update
    pub fn verify_update(
        update: &IdentityKeyUpdate,
        old_identity_key: &[u8; 32],
    ) -> Result<bool> {
        use ed25519_dalek::{Signature, VerifyingKey, Verifier};

        // Verify old key signature
        let old_verifier = VerifyingKey::from_bytes(old_identity_key)
            .map_err(|_| ProtocolError::InvalidMessage("Invalid old identity key".to_string()))?;

        let message_for_old = [
            &update.new_identity_key[..],
            b"identity_update_old",
        ].concat();

        let old_sig = Signature::from_bytes(&update.old_key_signature);
        old_verifier
            .verify(&message_for_old, &old_sig)
            .map_err(|_| ProtocolError::InvalidMessage("Invalid old key signature".to_string()))?;

        // Verify new key signature
        let new_verifier = VerifyingKey::from_bytes(&update.new_identity_key)
            .map_err(|_| ProtocolError::InvalidMessage("Invalid new identity key".to_string()))?;

        let message_for_new = [
            old_identity_key.as_slice(),
            b"identity_update_new",
        ].concat();

        let new_sig = Signature::from_bytes(&update.new_key_signature);
        new_verifier
            .verify(&message_for_new, &new_sig)
            .map_err(|_| ProtocolError::InvalidMessage("Invalid new key signature".to_string()))?;

        Ok(true)
    }
}

/// Handler for device list updates
pub struct DeviceListHandler;

impl DeviceListHandler {
    /// Handle device list update
    pub fn handle(
        update: &DeviceListUpdate,
        on_device_added: impl Fn(&DeviceId, &str),
        on_device_removed: impl Fn(&DeviceId),
        current_devices: &[DeviceId],
    ) {
        debug!("Handling device list update: {} devices", update.devices.len());

        let new_device_ids: Vec<_> = update.devices.iter().map(|d| &d.device_id).collect();

        // Find added devices
        for device in &update.devices {
            if !current_devices.contains(&device.device_id) {
                on_device_added(&device.device_id, &device.name);
            }
        }

        // Find removed devices
        for device_id in current_devices {
            if !new_device_ids.contains(&device_id) {
                on_device_removed(device_id);
            }
        }
    }
}

/// Handler for sync messages (multi-device)
pub struct SyncHandler;

impl SyncHandler {
    /// Handle sync message
    pub fn handle(
        sync: &SyncMessage,
        decrypt: impl Fn(&[u8]) -> Result<Vec<u8>>,
        on_sync: impl FnOnce(crate::protocol::SyncType, Vec<u8>),
    ) -> Result<()> {
        debug!("Handling sync message: {:?}", sync.sync_type);

        let decrypted = decrypt(&sync.data)?;
        on_sync(sync.sync_type, decrypted);
        Ok(())
    }

    /// Create sync message
    pub fn create_sync(
        sync_type: crate::protocol::SyncType,
        data: Vec<u8>,
        encrypt: impl Fn(&[u8]) -> Vec<u8>,
    ) -> SyncMessage {
        SyncMessage {
            sync_type,
            data: encrypt(&data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_creation() {
        let receipt = ReceiptHandler::create_delivery_receipt("msg-123");
        assert_eq!(receipt.message_id.as_str(), "msg-123");
        assert_eq!(receipt.receipt_type, ReceiptType::Delivered);
    }

    #[test]
    fn test_typing_indicator() {
        let indicator = TypingHandler::create_typing_started(UserId::from_string("user-1"));
        assert!(indicator.is_typing);

        let stopped = TypingHandler::create_typing_stopped(UserId::from_string("user-1"));
        assert!(!stopped.is_typing);
    }

    #[test]
    fn test_deletion_creation() {
        let deletion = DeletionHandler::create_deletion(
            "msg-456",
            UserId::from_string("user-1"),
            true,
        );
        assert!(deletion.delete_for_everyone);
    }

    #[test]
    fn test_session_reset() {
        let reset = SessionResetHandler::create_reset(
            UserId::from_string("user-1"),
            DeviceId::from_string("device-1"),
            SessionResetReason::DecryptionFailure,
        );
        assert_eq!(reset.reason, SessionResetReason::DecryptionFailure);
    }
}
