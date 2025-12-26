//! Protocol configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Client configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Device name
    pub device_name: String,
    /// Number of one-time prekeys to maintain
    pub prekey_count: usize,
    /// Prekey refresh threshold
    pub prekey_refresh_threshold: usize,
    /// Session stale timeout (seconds)
    pub session_stale_timeout_secs: u64,
    /// Session rekey interval (seconds)
    pub session_rekey_interval_secs: u64,
    /// Maximum message size
    pub max_message_size: usize,
    /// Enable disappearing messages by default
    pub default_disappearing_messages: bool,
    /// Default disappearing message duration (seconds)
    pub default_disappearing_duration_secs: u64,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Privacy configuration
    pub privacy: PrivacyConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            device_name: "QiyasHash Device".to_string(),
            prekey_count: 100,
            prekey_refresh_threshold: 20,
            session_stale_timeout_secs: 30 * 24 * 3600, // 30 days
            session_rekey_interval_secs: 7 * 24 * 3600, // 7 days
            max_message_size: 65536,
            default_disappearing_messages: false,
            default_disappearing_duration_secs: 24 * 3600, // 24 hours
            retry: RetryConfig::default(),
            network: NetworkConfig::default(),
            privacy: PrivacyConfig::default(),
        }
    }
}

impl ClientConfig {
    /// Create with device name
    pub fn with_device_name(name: impl Into<String>) -> Self {
        Self {
            device_name: name.into(),
            ..Default::default()
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.prekey_count < 10 {
            return Err("prekey_count must be at least 10".to_string());
        }
        if self.prekey_refresh_threshold >= self.prekey_count {
            return Err("prekey_refresh_threshold must be less than prekey_count".to_string());
        }
        if self.max_message_size == 0 {
            return Err("max_message_size must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Retry configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial retry delay (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum retry delay (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(delay)
    }
}

/// Network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Connection timeout (seconds)
    pub connection_timeout_secs: u64,
    /// Request timeout (seconds)
    pub request_timeout_secs: u64,
    /// Keep-alive interval (seconds)
    pub keepalive_interval_secs: u64,
    /// Use Tor for anonymity
    pub use_tor: bool,
    /// DHT bootstrap nodes
    pub dht_bootstrap_nodes: Vec<String>,
    /// Relay node URLs
    pub relay_nodes: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            connection_timeout_secs: 30,
            request_timeout_secs: 60,
            keepalive_interval_secs: 30,
            use_tor: false,
            dht_bootstrap_nodes: Vec::new(),
            relay_nodes: Vec::new(),
        }
    }
}

/// Privacy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Send read receipts
    pub send_read_receipts: bool,
    /// Send typing indicators
    pub send_typing_indicators: bool,
    /// Show online status
    pub show_online_status: bool,
    /// Add random delays to messages (for traffic analysis resistance)
    pub add_random_delays: bool,
    /// Send dummy traffic
    pub send_dummy_traffic: bool,
    /// Dummy traffic rate (messages per hour)
    pub dummy_traffic_rate: f64,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            send_read_receipts: true,
            send_typing_indicators: true,
            show_online_status: true,
            add_random_delays: true,
            send_dummy_traffic: false,
            dummy_traffic_rate: 1.0,
        }
    }
}

impl PrivacyConfig {
    /// Maximum privacy settings
    pub fn maximum_privacy() -> Self {
        Self {
            send_read_receipts: false,
            send_typing_indicators: false,
            show_online_status: false,
            add_random_delays: true,
            send_dummy_traffic: true,
            dummy_traffic_rate: 5.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_retry_delay() {
        let config = RetryConfig::default();
        
        let delay0 = config.delay_for_attempt(0);
        assert_eq!(delay0.as_millis(), 100);
        
        let delay1 = config.delay_for_attempt(1);
        assert_eq!(delay1.as_millis(), 200);
        
        let delay2 = config.delay_for_attempt(2);
        assert_eq!(delay2.as_millis(), 400);
    }

    #[test]
    fn test_max_delay_cap() {
        let config = RetryConfig {
            max_delay_ms: 1000,
            ..Default::default()
        };
        
        let delay = config.delay_for_attempt(100);
        assert_eq!(delay.as_millis(), 1000);
    }
}
