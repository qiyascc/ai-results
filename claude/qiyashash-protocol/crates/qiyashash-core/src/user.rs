//! User management for QiyasHash

use serde::{Deserialize, Serialize};

use crate::types::{DeviceId, Fingerprint, Timestamp, UserId};

/// A user in the QiyasHash system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    /// User ID (derived from identity fingerprint)
    pub id: UserId,
    /// User profile
    pub profile: UserProfile,
    /// Identity fingerprint
    pub fingerprint: Fingerprint,
    /// Registered devices
    pub devices: Vec<Device>,
    /// When user was created
    pub created_at: Timestamp,
    /// When user was last seen
    pub last_seen_at: Option<Timestamp>,
    /// Trust level
    pub trust_level: TrustLevel,
}

impl User {
    /// Create a new user
    pub fn new(fingerprint: Fingerprint, profile: UserProfile) -> Self {
        let id = UserId::from_fingerprint(fingerprint.as_bytes());

        Self {
            id,
            profile,
            fingerprint,
            devices: Vec::new(),
            created_at: Timestamp::now(),
            last_seen_at: None,
            trust_level: TrustLevel::Unknown,
        }
    }

    /// Add a device
    pub fn add_device(&mut self, device: Device) {
        // Check if device already exists
        if !self.devices.iter().any(|d| d.id == device.id) {
            self.devices.push(device);
        }
    }

    /// Remove a device
    pub fn remove_device(&mut self, device_id: &DeviceId) {
        self.devices.retain(|d| &d.id != device_id);
    }

    /// Update last seen
    pub fn update_last_seen(&mut self) {
        self.last_seen_at = Some(Timestamp::now());
    }

    /// Check if user is online (seen in last 5 minutes)
    pub fn is_online(&self) -> bool {
        if let Some(last_seen) = self.last_seen_at {
            !last_seen.is_expired(300)
        } else {
            false
        }
    }

    /// Verify user identity (mark as verified)
    pub fn verify(&mut self) {
        self.trust_level = TrustLevel::Verified;
    }
}

/// User profile information
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserProfile {
    /// Display name
    pub display_name: Option<String>,
    /// Username (unique)
    pub username: Option<String>,
    /// Bio/status
    pub bio: Option<String>,
    /// Profile picture (encrypted reference)
    pub avatar: Option<Avatar>,
    /// Profile color
    pub color: Option<String>,
}

impl UserProfile {
    /// Create a new profile with display name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            display_name: Some(name.into()),
            ..Default::default()
        }
    }

    /// Get display name or fallback
    pub fn name_or_default(&self) -> &str {
        self.display_name
            .as_deref()
            .or(self.username.as_deref())
            .unwrap_or("Unknown")
    }
}

/// Profile avatar
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Avatar {
    /// Encrypted avatar data reference
    pub data_ref: String,
    /// Encryption key
    #[serde(with = "hex::serde")]
    pub key: [u8; 32],
    /// Content hash
    #[serde(with = "hex::serde")]
    pub hash: [u8; 32],
}

/// A user's device
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    /// Device ID
    pub id: DeviceId,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: DeviceType,
    /// Device identity key fingerprint
    pub fingerprint: Fingerprint,
    /// When device was registered
    pub registered_at: Timestamp,
    /// Last seen timestamp
    pub last_seen_at: Option<Timestamp>,
    /// Push notification token (encrypted)
    pub push_token: Option<String>,
}

impl Device {
    /// Create a new device
    pub fn new(
        name: impl Into<String>,
        device_type: DeviceType,
        fingerprint: Fingerprint,
    ) -> Self {
        Self {
            id: DeviceId::new(),
            name: name.into(),
            device_type,
            fingerprint,
            registered_at: Timestamp::now(),
            last_seen_at: None,
            push_token: None,
        }
    }
}

/// Type of device
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// Desktop computer
    Desktop,
    /// Mobile phone
    Mobile,
    /// Tablet
    Tablet,
    /// Web browser
    Web,
    /// CLI tool
    Cli,
    /// Unknown
    Unknown,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Trust level for a user
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Unknown/unverified user
    Unknown,
    /// User seen before but not verified
    Known,
    /// User verified via safety number
    Verified,
    /// User is blocked
    Blocked,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Contact information (for address book)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact {
    /// User ID
    pub user_id: UserId,
    /// Contact alias (local name)
    pub alias: Option<String>,
    /// Notes
    pub notes: Option<String>,
    /// When added to contacts
    pub added_at: Timestamp,
    /// Favorite
    pub is_favorite: bool,
    /// Muted
    pub is_muted: bool,
    /// Blocked
    pub is_blocked: bool,
}

impl Contact {
    /// Create a new contact
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            alias: None,
            notes: None,
            added_at: Timestamp::now(),
            is_favorite: false,
            is_muted: false,
            is_blocked: false,
        }
    }

    /// Set alias
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Mark as favorite
    pub fn favorite(mut self) -> Self {
        self.is_favorite = true;
        self
    }

    /// Block contact
    pub fn block(&mut self) {
        self.is_blocked = true;
    }

    /// Unblock contact
    pub fn unblock(&mut self) {
        self.is_blocked = false;
    }

    /// Mute contact
    pub fn mute(&mut self) {
        self.is_muted = true;
    }

    /// Unmute contact
    pub fn unmute(&mut self) {
        self.is_muted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let fingerprint = Fingerprint::from_bytes([0x42; 32]);
        let profile = UserProfile::with_name("Alice");
        let user = User::new(fingerprint.clone(), profile);

        assert_eq!(user.fingerprint, fingerprint);
        assert_eq!(user.profile.display_name, Some("Alice".to_string()));
        assert_eq!(user.trust_level, TrustLevel::Unknown);
    }

    #[test]
    fn test_device_management() {
        let fingerprint = Fingerprint::from_bytes([0x42; 32]);
        let mut user = User::new(fingerprint, UserProfile::default());

        let device = Device::new(
            "iPhone",
            DeviceType::Mobile,
            Fingerprint::from_bytes([0x43; 32]),
        );
        let device_id = device.id.clone();

        user.add_device(device);
        assert_eq!(user.devices.len(), 1);

        user.remove_device(&device_id);
        assert!(user.devices.is_empty());
    }

    #[test]
    fn test_contact() {
        let contact = Contact::new(UserId::new())
            .with_alias("Bob")
            .favorite();

        assert_eq!(contact.alias, Some("Bob".to_string()));
        assert!(contact.is_favorite);
        assert!(!contact.is_blocked);
    }

    #[test]
    fn test_trust_levels() {
        assert!(TrustLevel::Verified > TrustLevel::Known);
        assert!(TrustLevel::Known > TrustLevel::Unknown);
    }
}
