//! Message fragmentation with Reed-Solomon erasure coding
//!
//! Messages are split into fragments using Reed-Solomon encoding,
//! allowing reconstruction from any subset of fragments.

use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{DhtError, Result};

/// Fragment identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FragmentId(String);

impl FragmentId {
    /// Create new fragment ID from message ID and index
    pub fn new(message_id: &str, index: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(message_id.as_bytes());
        hasher.update(&index.to_be_bytes());
        let hash = hasher.finalize();
        Self(hex::encode(&hash[..16]))
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FragmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A message fragment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fragment {
    /// Fragment ID
    pub id: FragmentId,
    /// Parent message ID
    pub message_id: String,
    /// Fragment index
    pub index: usize,
    /// Total number of fragments
    pub total: usize,
    /// Fragment data (may be data or parity)
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
    /// Whether this is a parity shard
    pub is_parity: bool,
    /// Shard size (for reconstruction)
    pub shard_size: usize,
    /// Original message size
    pub message_size: usize,
    /// Expiry timestamp (Unix seconds)
    pub expiry: u64,
    /// Creation timestamp
    pub created_at: u64,
}

impl Fragment {
    /// Check if fragment is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expiry
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

/// Collection of fragments for a message
pub struct MessageFragments {
    /// Message ID
    pub message_id: String,
    /// All fragments
    pub fragments: Vec<Option<Fragment>>,
    /// Data shard count
    pub data_shards: usize,
    /// Parity shard count
    pub parity_shards: usize,
    /// Original message size
    pub message_size: usize,
}

impl MessageFragments {
    /// Create from encoded message
    pub fn encode(
        message_id: impl Into<String>,
        data: &[u8],
        data_shards: usize,
        parity_shards: usize,
        expiry_secs: u64,
    ) -> Result<Self> {
        let message_id = message_id.into();
        let total_shards = data_shards + parity_shards;

        // Create Reed-Solomon encoder
        let rs = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| DhtError::EncodingError(e.to_string()))?;

        // Calculate shard size (must be equal for all shards)
        let shard_size = (data.len() + data_shards - 1) / data_shards;

        // Create shards with padding
        let mut shards: Vec<Vec<u8>> = (0..data_shards)
            .map(|i| {
                let start = i * shard_size;
                let end = std::cmp::min(start + shard_size, data.len());
                let mut shard = vec![0u8; shard_size];
                if start < data.len() {
                    let copy_len = end - start;
                    shard[..copy_len].copy_from_slice(&data[start..end]);
                }
                shard
            })
            .collect();

        // Add parity shards
        for _ in 0..parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // Encode parity
        rs.encode(&mut shards)
            .map_err(|e| DhtError::EncodingError(e.to_string()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expiry = now + expiry_secs;

        // Create fragments
        let fragments: Vec<Option<Fragment>> = shards
            .into_iter()
            .enumerate()
            .map(|(i, shard_data)| {
                Some(Fragment {
                    id: FragmentId::new(&message_id, i),
                    message_id: message_id.clone(),
                    index: i,
                    total: total_shards,
                    data: shard_data,
                    is_parity: i >= data_shards,
                    shard_size,
                    message_size: data.len(),
                    expiry,
                    created_at: now,
                })
            })
            .collect();

        Ok(Self {
            message_id,
            fragments,
            data_shards,
            parity_shards,
            message_size: data.len(),
        })
    }

    /// Try to reconstruct message from fragments
    pub fn decode(&self) -> Result<Vec<u8>> {
        let total_shards = self.data_shards + self.parity_shards;

        // Check we have enough fragments
        let available = self.fragments.iter().filter(|f| f.is_some()).count();
        if available < self.data_shards {
            return Err(DhtError::ReconstructionFailed {
                needed: self.data_shards,
                have: available,
            });
        }

        // Create Reed-Solomon decoder
        let rs = ReedSolomon::new(self.data_shards, self.parity_shards)
            .map_err(|e| DhtError::DecodingError(e.to_string()))?;

        // Get shard size from first available fragment
        let shard_size = self
            .fragments
            .iter()
            .filter_map(|f| f.as_ref())
            .next()
            .map(|f| f.shard_size)
            .ok_or_else(|| DhtError::DecodingError("No fragments available".to_string()))?;

        // Prepare shards for reconstruction
        let mut shards: Vec<Option<Vec<u8>>> = (0..total_shards)
            .map(|i| self.fragments.get(i).and_then(|f| f.as_ref()).map(|f| f.data.clone()))
            .collect();

        // Reconstruct missing shards
        rs.reconstruct(&mut shards)
            .map_err(|e| DhtError::DecodingError(e.to_string()))?;

        // Combine data shards
        let mut result = Vec::with_capacity(self.message_size);
        for shard in shards.iter().take(self.data_shards) {
            if let Some(data) = shard {
                result.extend_from_slice(data);
            }
        }

        // Truncate to original size
        result.truncate(self.message_size);

        Ok(result)
    }

    /// Add a fragment
    pub fn add_fragment(&mut self, fragment: Fragment) -> Result<()> {
        if fragment.message_id != self.message_id {
            return Err(DhtError::InvalidFragment("Message ID mismatch".to_string()));
        }
        if fragment.index >= self.fragments.len() {
            return Err(DhtError::InvalidFragment("Invalid index".to_string()));
        }

        self.fragments[fragment.index] = Some(fragment);
        Ok(())
    }

    /// Check if we have enough fragments to reconstruct
    pub fn can_reconstruct(&self) -> bool {
        self.fragments.iter().filter(|f| f.is_some()).count() >= self.data_shards
    }

    /// Get missing fragment indices
    pub fn missing_indices(&self) -> Vec<usize> {
        self.fragments
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_none())
            .map(|(i, _)| i)
            .collect()
    }

    /// Get fragment IDs for all fragments
    pub fn fragment_ids(&self) -> Vec<FragmentId> {
        (0..self.fragments.len())
            .map(|i| FragmentId::new(&self.message_id, i))
            .collect()
    }

    /// Create empty container for receiving fragments
    pub fn new_empty(
        message_id: impl Into<String>,
        data_shards: usize,
        parity_shards: usize,
        message_size: usize,
    ) -> Self {
        let total = data_shards + parity_shards;
        Self {
            message_id: message_id.into(),
            fragments: vec![None; total],
            data_shards,
            parity_shards,
            message_size,
        }
    }
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
    fn test_fragment_encoding() {
        let message = b"Hello, QiyasHash! This is a test message for fragmentation.";
        let fragments = MessageFragments::encode("msg-123", message, 3, 2, 3600).unwrap();

        assert_eq!(fragments.fragments.len(), 5);
        assert!(fragments.can_reconstruct());
    }

    #[test]
    fn test_fragment_reconstruction() {
        let message = b"Hello, QiyasHash! This is a test message for fragmentation.";
        let fragments = MessageFragments::encode("msg-123", message, 3, 2, 3600).unwrap();

        let reconstructed = fragments.decode().unwrap();
        assert_eq!(message.as_slice(), reconstructed.as_slice());
    }

    #[test]
    fn test_reconstruction_with_missing_shards() {
        let message = b"Hello, QiyasHash! This is a test message.";
        let mut fragments = MessageFragments::encode("msg-123", message, 3, 2, 3600).unwrap();

        // Remove 2 fragments (still have 3, which is minimum needed)
        fragments.fragments[0] = None;
        fragments.fragments[3] = None;

        assert!(fragments.can_reconstruct());
        let reconstructed = fragments.decode().unwrap();
        assert_eq!(message.as_slice(), reconstructed.as_slice());
    }

    #[test]
    fn test_reconstruction_not_enough_shards() {
        let message = b"Hello, QiyasHash!";
        let mut fragments = MessageFragments::encode("msg-123", message, 3, 2, 3600).unwrap();

        // Remove 3 fragments (only have 2, need 3)
        fragments.fragments[0] = None;
        fragments.fragments[1] = None;
        fragments.fragments[2] = None;

        assert!(!fragments.can_reconstruct());
        assert!(fragments.decode().is_err());
    }

    #[test]
    fn test_fragment_id() {
        let id1 = FragmentId::new("msg-123", 0);
        let id2 = FragmentId::new("msg-123", 1);
        let id3 = FragmentId::new("msg-123", 0);

        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_fragment_serialization() {
        let message = b"Test message";
        let fragments = MessageFragments::encode("msg-123", message, 2, 1, 3600).unwrap();

        let fragment = fragments.fragments[0].as_ref().unwrap();
        let bytes = fragment.to_bytes().unwrap();
        let restored = Fragment::from_bytes(&bytes).unwrap();

        assert_eq!(fragment.id, restored.id);
        assert_eq!(fragment.data, restored.data);
    }
}
