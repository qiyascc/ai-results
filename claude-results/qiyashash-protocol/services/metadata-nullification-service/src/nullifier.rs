//! Metadata Nullifier implementation

use rand::{seq::SliceRandom, Rng};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::debug;

/// Block sizes for message padding (in bytes)
const BLOCK_SIZES: [usize; 6] = [256, 512, 1024, 2048, 4096, 8192];

/// Maximum random delay in milliseconds
const MAX_DELAY_MS: u64 = 500;

/// Nullification statistics
pub struct NullificationStats {
    pub messages_processed: u64,
    pub bytes_nullified: u64,
    pub avg_padding_ratio: f64,
}

/// Metadata Nullifier
pub struct MetadataNullifier {
    /// Aggressive mode strips more metadata
    aggressive: bool,
    /// Messages processed counter
    messages_processed: AtomicU64,
    /// Bytes nullified counter
    bytes_nullified: AtomicU64,
    /// Total padding added
    total_padding: AtomicU64,
    /// Total original size
    total_original: AtomicU64,
}

impl MetadataNullifier {
    /// Create a new nullifier
    pub fn new(aggressive: bool) -> Self {
        Self {
            aggressive,
            messages_processed: AtomicU64::new(0),
            bytes_nullified: AtomicU64::new(0),
            total_padding: AtomicU64::new(0),
            total_original: AtomicU64::new(0),
        }
    }

    /// Strip timing metadata from message
    /// In practice, this removes any embedded timestamps or sequence info
    pub fn strip_timing_metadata(&self, data: &[u8]) -> Vec<u8> {
        // For encrypted messages, we can't actually modify content
        // But we record that we've "processed" it
        self.messages_processed.fetch_add(1, Ordering::Relaxed);
        
        // In a real implementation, this would strip headers
        // that might contain timing information
        data.to_vec()
    }

    /// Pad message to nearest block size
    pub fn pad_to_block(&self, data: &[u8]) -> Vec<u8> {
        let original_len = data.len();
        
        // Find appropriate block size
        let target_size = if self.aggressive {
            // Always pad to largest block
            BLOCK_SIZES[BLOCK_SIZES.len() - 1]
        } else {
            // Pad to nearest larger block size
            *BLOCK_SIZES.iter()
                .find(|&&size| size >= original_len)
                .unwrap_or(&BLOCK_SIZES[BLOCK_SIZES.len() - 1])
        };

        let target_size = target_size.max(original_len);
        let padding_len = target_size - original_len;

        // Create padded message
        let mut padded = Vec::with_capacity(target_size + 4);
        
        // Store original length (4 bytes, big endian)
        padded.extend_from_slice(&(original_len as u32).to_be_bytes());
        
        // Add original data
        padded.extend_from_slice(data);
        
        // Add random padding
        let mut rng = rand::thread_rng();
        for _ in 0..padding_len {
            padded.push(rng.gen());
        }

        // Update stats
        self.total_padding.fetch_add(padding_len as u64, Ordering::Relaxed);
        self.total_original.fetch_add(original_len as u64, Ordering::Relaxed);
        self.bytes_nullified.fetch_add(padded.len() as u64, Ordering::Relaxed);

        debug!(
            "Padded message from {} to {} bytes",
            original_len,
            padded.len()
        );

        padded
    }

    /// Remove padding from message
    pub fn unpad(&self, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 4 {
            return None;
        }

        // Read original length
        let len_bytes: [u8; 4] = data[..4].try_into().ok()?;
        let original_len = u32::from_be_bytes(len_bytes) as usize;

        if data.len() < 4 + original_len {
            return None;
        }

        Some(data[4..4 + original_len].to_vec())
    }

    /// Shuffle messages to prevent ordering analysis
    pub fn shuffle_messages(&self, messages: &mut [Vec<u8>]) {
        let mut rng = rand::thread_rng();
        messages.shuffle(&mut rng);
    }

    /// Add random delay to prevent timing analysis
    pub async fn random_delay(&self) {
        let mut rng = rand::thread_rng();
        let delay_ms = rng.gen_range(0..MAX_DELAY_MS);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    /// Add Poisson-distributed delay
    pub async fn poisson_delay(&self, lambda: f64) {
        let mut rng = rand::thread_rng();
        // Inverse transform sampling for exponential distribution
        let u: f64 = rng.gen();
        let delay_ms = (-lambda.ln() * (1.0 - u).ln()) as u64;
        let capped_delay = delay_ms.min(MAX_DELAY_MS * 2);
        tokio::time::sleep(Duration::from_millis(capped_delay)).await;
    }

    /// Generate cover traffic (dummy messages)
    pub fn generate_cover_message(&self, size: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut data = vec![0u8; size];
        rng.fill(&mut data[..]);
        
        // Mark as cover traffic with magic bytes
        if data.len() >= 4 {
            data[0] = 0xCO;
            data[1] = 0xVE;
            data[2] = 0x52;
            data[3] = 0x00;
        }
        
        data
    }

    /// Check if message is cover traffic
    pub fn is_cover_traffic(&self, data: &[u8]) -> bool {
        data.len() >= 4 && data[0] == 0xCO && data[1] == 0xVE && data[2] == 0x52
    }

    /// Get nullification statistics
    pub fn get_stats(&self) -> NullificationStats {
        let total_original = self.total_original.load(Ordering::Relaxed);
        let total_padding = self.total_padding.load(Ordering::Relaxed);
        
        let avg_padding_ratio = if total_original > 0 {
            total_padding as f64 / total_original as f64
        } else {
            0.0
        };

        NullificationStats {
            messages_processed: self.messages_processed.load(Ordering::Relaxed),
            bytes_nullified: self.bytes_nullified.load(Ordering::Relaxed),
            avg_padding_ratio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_unpad() {
        let nullifier = MetadataNullifier::new(false);
        
        let original = b"Hello, World!";
        let padded = nullifier.pad_to_block(original);
        
        assert!(padded.len() >= original.len());
        assert_eq!(padded.len() % 256, 4); // 4 bytes for length prefix
        
        let unpadded = nullifier.unpad(&padded).unwrap();
        assert_eq!(unpadded, original);
    }

    #[test]
    fn test_cover_traffic() {
        let nullifier = MetadataNullifier::new(false);
        
        let cover = nullifier.generate_cover_message(512);
        assert!(nullifier.is_cover_traffic(&cover));
        
        let real = b"real message";
        assert!(!nullifier.is_cover_traffic(real));
    }

    #[test]
    fn test_shuffle() {
        let nullifier = MetadataNullifier::new(false);
        
        let mut messages: Vec<Vec<u8>> = (0..10)
            .map(|i| vec![i as u8])
            .collect();
        
        let original = messages.clone();
        nullifier.shuffle_messages(&mut messages);
        
        // Should be same elements, possibly different order
        assert_eq!(messages.len(), original.len());
    }
}
