//! Application state

use qiyashash_core::types::{DeviceId, UserId};
use serde::{Deserialize, Serialize};

/// Application state
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    /// Whether app is initialized
    pub initialized: bool,
    /// Current user ID
    pub user_id: Option<UserId>,
    /// Current device ID
    pub device_id: Option<DeviceId>,
    /// Online status
    pub is_online: bool,
    /// Syncing status
    pub is_syncing: bool,
    /// Unread count total
    pub total_unread: usize,
    /// Active conversation
    pub active_conversation: Option<UserId>,
    /// App settings
    pub settings: AppSettings,
}

impl AppState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set active conversation
    pub fn set_active_conversation(&mut self, user_id: Option<UserId>) {
        self.active_conversation = user_id;
    }

    /// Update online status
    pub fn set_online(&mut self, online: bool) {
        self.is_online = online;
    }

    /// Update syncing status
    pub fn set_syncing(&mut self, syncing: bool) {
        self.is_syncing = syncing;
    }

    /// Update total unread
    pub fn set_total_unread(&mut self, count: usize) {
        self.total_unread = count;
    }
}

/// Application settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    /// Theme (light/dark/system)
    pub theme: String,
    /// Notification settings
    pub notifications: NotificationSettings,
    /// Privacy settings
    pub privacy: PrivacySettings,
    /// Sync settings
    pub sync: SyncSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            notifications: NotificationSettings::default(),
            privacy: PrivacySettings::default(),
            sync: SyncSettings::default(),
        }
    }
}

/// Notification settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Enable notifications
    pub enabled: bool,
    /// Show message preview
    pub show_preview: bool,
    /// Play sound
    pub play_sound: bool,
    /// Badge count
    pub show_badge: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_preview: true,
            play_sound: true,
            show_badge: true,
        }
    }
}

/// Privacy settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Send read receipts
    pub send_read_receipts: bool,
    /// Send typing indicators
    pub send_typing_indicators: bool,
    /// Show online status
    pub show_online_status: bool,
    /// Screen lock enabled
    pub screen_lock: bool,
    /// Auto-lock timeout (seconds)
    pub auto_lock_timeout: u64,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            send_read_receipts: true,
            send_typing_indicators: true,
            show_online_status: true,
            screen_lock: false,
            auto_lock_timeout: 300,
        }
    }
}

/// Sync settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncSettings {
    /// Sync contacts
    pub sync_contacts: bool,
    /// Sync in background
    pub background_sync: bool,
    /// Sync interval (seconds)
    pub sync_interval: u64,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            sync_contacts: true,
            background_sync: true,
            sync_interval: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = AppState::new();
        assert!(!state.initialized);
        assert!(state.user_id.is_none());
    }

    #[test]
    fn test_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.theme, "system");
        assert!(settings.notifications.enabled);
    }
}
