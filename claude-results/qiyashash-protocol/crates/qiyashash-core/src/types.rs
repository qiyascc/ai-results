//! Core types used throughout QiyasHash

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// User identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    /// Create a new random user ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Create from identity fingerprint
    pub fn from_fingerprint(fingerprint: &[u8; 32]) -> Self {
        Self(hex::encode(fingerprint))
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Device identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    /// Create a new random device ID
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

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Timestamp in milliseconds since Unix epoch
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Create timestamp for current time
    pub fn now() -> Self {
        Self(chrono::Utc::now().timestamp_millis())
    }

    /// Create from milliseconds
    pub fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Create from seconds
    pub fn from_secs(secs: i64) -> Self {
        Self(secs * 1000)
    }

    /// Get as milliseconds
    pub fn as_millis(&self) -> i64 {
        self.0
    }

    /// Get as seconds
    pub fn as_secs(&self) -> i64 {
        self.0 / 1000
    }

    /// Get as chrono DateTime
    pub fn as_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp_millis(self.0)
            .unwrap_or_else(|| chrono::Utc::now())
    }

    /// Check if expired (older than duration)
    pub fn is_expired(&self, duration_secs: i64) -> bool {
        let now = Self::now();
        now.0 - self.0 > duration_secs * 1000
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_datetime().format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

/// Fingerprint for identity verification
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint(#[serde(with = "hex::serde")] pub [u8; 32]);

impl Fingerprint {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Get as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Format for human display (groups of 4 hex chars)
    pub fn to_display(&self) -> String {
        let hex = self.to_hex();
        hex.chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display())
    }
}

/// Safety number for session verification
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyNumber {
    /// The numeric representation
    pub number: String,
    /// The fingerprints used to compute it
    pub our_fingerprint: Fingerprint,
    pub their_fingerprint: Fingerprint,
}

impl SafetyNumber {
    /// Compute safety number from two fingerprints
    pub fn compute(our_fp: &Fingerprint, their_fp: &Fingerprint) -> Self {
        use sha2::{Digest, Sha256};

        // Concatenate fingerprints in consistent order
        let (first, second) = if our_fp.0 < their_fp.0 {
            (our_fp, their_fp)
        } else {
            (their_fp, our_fp)
        };

        let mut hasher = Sha256::new();
        hasher.update(&first.0);
        hasher.update(&second.0);
        let hash = hasher.finalize();

        // Convert to numeric string (5 groups of 5 digits)
        let mut number = String::with_capacity(30);
        for chunk in hash.chunks(5) {
            let n = chunk.iter().fold(0u64, |acc, &b| acc * 256 + b as u64);
            number.push_str(&format!("{:05} ", n % 100000));
        }

        Self {
            number: number.trim().to_string(),
            our_fingerprint: our_fp.clone(),
            their_fingerprint: their_fp.clone(),
        }
    }
}

impl fmt::Display for SafetyNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.number)
    }
}

/// Content type for messages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// Plain text
    Text,
    /// Image
    Image { mime_type: String },
    /// Video
    Video { mime_type: String },
    /// Audio
    Audio { mime_type: String },
    /// File
    File { mime_type: String, filename: String },
    /// Location
    Location,
    /// Contact
    Contact,
    /// Sticker
    Sticker,
    /// Voice note
    VoiceNote,
    /// Reaction
    Reaction,
    /// System message
    System,
}

impl Default for ContentType {
    fn default() -> Self {
        Self::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);

        let id3 = UserId::from_string("test-user");
        assert_eq!(id3.as_str(), "test-user");
    }

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::now();
        assert!(ts.as_millis() > 0);

        let ts2 = Timestamp::from_secs(1000);
        assert_eq!(ts2.as_millis(), 1_000_000);
    }

    #[test]
    fn test_fingerprint() {
        let bytes = [0x42u8; 32];
        let fp = Fingerprint::from_bytes(bytes);
        assert_eq!(fp.as_bytes(), &bytes);

        let hex = fp.to_hex();
        let fp2 = Fingerprint::from_hex(&hex).unwrap();
        assert_eq!(fp, fp2);
    }

    #[test]
    fn test_safety_number() {
        let fp1 = Fingerprint::from_bytes([0x01u8; 32]);
        let fp2 = Fingerprint::from_bytes([0x02u8; 32]);

        let sn1 = SafetyNumber::compute(&fp1, &fp2);
        let sn2 = SafetyNumber::compute(&fp2, &fp1);

        // Should be same regardless of order
        assert_eq!(sn1.number, sn2.number);
    }
}
