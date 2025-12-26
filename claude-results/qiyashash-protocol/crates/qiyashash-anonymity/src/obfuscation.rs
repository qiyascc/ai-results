//! Traffic obfuscation for preventing traffic analysis
//!
//! Implements various techniques to make traffic analysis more difficult:
//! - Random delays between messages
//! - Message padding to uniform size
//! - Message batching
//! - Cover traffic generation

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, trace};

use crate::config::{CoverTrafficConfig, ObfuscationConfig};
use crate::error::Result;

/// Traffic obfuscator
pub struct TrafficObfuscator {
    config: ObfuscationConfig,
    cover_config: CoverTrafficConfig,
    message_queue: Arc<Mutex<VecDeque<QueuedMessage>>>,
    last_send: Arc<Mutex<Instant>>,
}

/// Queued message with metadata
struct QueuedMessage {
    data: Vec<u8>,
    queued_at: Instant,
    is_cover: bool,
}

impl TrafficObfuscator {
    /// Create a new traffic obfuscator
    pub fn new(config: ObfuscationConfig, cover_config: CoverTrafficConfig) -> Self {
        Self {
            config,
            cover_config,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            last_send: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Process outgoing message with obfuscation
    pub async fn obfuscate(&self, data: &[u8]) -> Vec<u8> {
        if !self.config.enabled {
            return data.to_vec();
        }

        // Add random delay
        let delay = self.random_delay();
        sleep(delay).await;

        // Add padding
        let padded = self.add_padding(data);

        padded
    }

    /// Queue message for batched sending
    pub fn queue_message(&self, data: Vec<u8>) {
        let mut queue = self.message_queue.lock();
        queue.push_back(QueuedMessage {
            data,
            queued_at: Instant::now(),
            is_cover: false,
        });
    }

    /// Get next batch of messages
    pub fn get_batch(&self) -> Vec<Vec<u8>> {
        let mut queue = self.message_queue.lock();
        let batch_window = Duration::from_millis(self.config.batch_window_ms);
        
        let mut batch = Vec::new();
        let now = Instant::now();
        
        while let Some(msg) = queue.front() {
            if now.duration_since(msg.queued_at) >= batch_window || !self.config.batch_messages {
                if let Some(msg) = queue.pop_front() {
                    batch.push(msg.data);
                }
            } else {
                break;
            }
        }
        
        batch
    }

    /// Add padding to message
    pub fn add_padding(&self, data: &[u8]) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let (min_pad, max_pad) = self.config.padding_range;
        let pad_size = rng.gen_range(min_pad..=max_pad);
        
        // Create padded message: [length: 4 bytes][data][random padding]
        let mut result = Vec::with_capacity(4 + data.len() + pad_size);
        result.extend_from_slice(&(data.len() as u32).to_be_bytes());
        result.extend_from_slice(data);
        
        // Add random padding
        let mut padding = vec![0u8; pad_size];
        rng.fill(&mut padding[..]);
        result.extend_from_slice(&padding);
        
        result
    }

    /// Remove padding from message
    pub fn remove_padding(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Ok(data.to_vec());
        }
        
        let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if 4 + length > data.len() {
            return Ok(data.to_vec()); // Not padded or corrupted
        }
        
        Ok(data[4..4 + length].to_vec())
    }

    /// Generate random delay
    fn random_delay(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let delay_ms = rng.gen_range(self.config.min_delay_ms..=self.config.max_delay_ms);
        Duration::from_millis(delay_ms)
    }

    /// Generate cover message
    pub fn generate_cover_message(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let (min_size, max_size) = self.cover_config.size_range;
        let size = rng.gen_range(min_size..=max_size);
        
        let mut data = vec![0u8; size];
        rng.fill(&mut data[..]);
        
        // Mark as cover traffic (first byte = 0xFF)
        data[0] = 0xFF;
        
        data
    }

    /// Check if message is cover traffic
    pub fn is_cover_traffic(&self, data: &[u8]) -> bool {
        !data.is_empty() && data[0] == 0xFF
    }

    /// Start cover traffic generator
    pub fn start_cover_traffic(&self) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(100);
        let config = self.cover_config.clone();
        
        if !config.enabled {
            return rx;
        }
        
        tokio::spawn(async move {
            let avg_interval_secs = 3600.0 / config.rate_per_hour;
            
            loop {
                // Calculate next interval
                let interval_secs = if config.poisson_timing {
                    // Poisson distribution for inter-arrival times
                    let mut rng = rand::thread_rng();
                    let u: f64 = rng.gen();
                    -avg_interval_secs * u.ln()
                } else {
                    avg_interval_secs
                };
                
                sleep(Duration::from_secs_f64(interval_secs)).await;
                
                // Generate and send cover message
                let mut rng = rand::thread_rng();
                let (min_size, max_size) = config.size_range;
                let size = rng.gen_range(min_size..=max_size);
                
                let mut data = vec![0u8; size];
                rng.fill(&mut data[..]);
                data[0] = 0xFF; // Mark as cover
                
                if tx.send(data).await.is_err() {
                    break;
                }
            }
        });
        
        rx
    }
}

/// Message timing analyzer (for detection of traffic analysis)
pub struct TimingAnalyzer {
    message_times: Vec<Instant>,
    window_size: usize,
}

impl TimingAnalyzer {
    /// Create new timing analyzer
    pub fn new(window_size: usize) -> Self {
        Self {
            message_times: Vec::new(),
            window_size,
        }
    }

    /// Record message send time
    pub fn record_send(&mut self) {
        self.message_times.push(Instant::now());
        
        // Keep only recent messages
        if self.message_times.len() > self.window_size {
            self.message_times.remove(0);
        }
    }

    /// Calculate average inter-message delay
    pub fn average_delay(&self) -> Option<Duration> {
        if self.message_times.len() < 2 {
            return None;
        }
        
        let mut total = Duration::ZERO;
        for i in 1..self.message_times.len() {
            total += self.message_times[i].duration_since(self.message_times[i - 1]);
        }
        
        Some(total / (self.message_times.len() - 1) as u32)
    }

    /// Check if timing pattern is suspicious (too regular)
    pub fn is_suspicious(&self) -> bool {
        if self.message_times.len() < 10 {
            return false;
        }
        
        // Calculate variance of inter-message delays
        let delays: Vec<f64> = self.message_times
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_secs_f64())
            .collect();
        
        let mean = delays.iter().sum::<f64>() / delays.len() as f64;
        let variance = delays.iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f64>() / delays.len() as f64;
        
        // Low variance indicates regular timing (suspicious)
        let coefficient_of_variation = variance.sqrt() / mean;
        coefficient_of_variation < 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding() {
        let config = ObfuscationConfig::default();
        let cover_config = CoverTrafficConfig::default();
        let obfuscator = TrafficObfuscator::new(config, cover_config);
        
        let original = b"Hello, World!";
        let padded = obfuscator.add_padding(original);
        
        assert!(padded.len() > original.len());
        
        let unpadded = obfuscator.remove_padding(&padded).unwrap();
        assert_eq!(original.as_slice(), unpadded.as_slice());
    }

    #[test]
    fn test_cover_message() {
        let config = ObfuscationConfig::default();
        let cover_config = CoverTrafficConfig {
            enabled: true,
            size_range: (100, 200),
            ..Default::default()
        };
        let obfuscator = TrafficObfuscator::new(config, cover_config);
        
        let cover = obfuscator.generate_cover_message();
        
        assert!(cover.len() >= 100);
        assert!(cover.len() <= 200);
        assert!(obfuscator.is_cover_traffic(&cover));
    }

    #[test]
    fn test_timing_analyzer() {
        let mut analyzer = TimingAnalyzer::new(100);
        
        analyzer.record_send();
        std::thread::sleep(Duration::from_millis(10));
        analyzer.record_send();
        std::thread::sleep(Duration::from_millis(10));
        analyzer.record_send();
        
        let avg = analyzer.average_delay();
        assert!(avg.is_some());
    }
}
