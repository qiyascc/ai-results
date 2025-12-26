//! Relay configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Relay node configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayConfig {
    /// Known relay nodes
    pub relay_nodes: Vec<RelayNodeInfo>,
    /// Number of relays to use for each message
    pub relay_count: usize,
    /// Message expiry in seconds
    pub message_expiry_secs: u64,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum blob size
    pub max_blob_size: usize,
    /// Retry configuration
    pub retry: RetryConfig,
    /// TLS configuration
    pub tls: TlsConfig,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            relay_nodes: Vec::new(),
            relay_count: crate::DEFAULT_RELAY_COUNT,
            message_expiry_secs: crate::DEFAULT_MESSAGE_EXPIRY_SECS,
            connection_timeout_secs: 30,
            request_timeout_secs: 60,
            max_blob_size: crate::MAX_BLOB_SIZE,
            retry: RetryConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

impl RelayConfig {
    /// Create with relay nodes
    pub fn with_nodes(nodes: Vec<RelayNodeInfo>) -> Self {
        Self {
            relay_nodes: nodes,
            ..Default::default()
        }
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.relay_count == 0 {
            return Err("relay_count must be > 0".to_string());
        }
        if self.max_blob_size == 0 {
            return Err("max_blob_size must be > 0".to_string());
        }
        Ok(())
    }
}

/// Relay node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayNodeInfo {
    /// Node identifier
    pub id: String,
    /// Node address (host:port)
    pub address: String,
    /// Node public key for TLS
    #[serde(with = "hex::serde")]
    pub public_key: [u8; 32],
    /// Node region (for latency optimization)
    pub region: Option<String>,
    /// Priority (lower = preferred)
    pub priority: u32,
}

impl RelayNodeInfo {
    /// Create new relay node info
    pub fn new(id: impl Into<String>, address: impl Into<String>, public_key: [u8; 32]) -> Self {
        Self {
            id: id.into(),
            address: address.into(),
            public_key,
            region: None,
            priority: 100,
        }
    }
}

/// Retry configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retries
    pub max_retries: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(delay)
    }
}

/// TLS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable certificate verification
    pub verify_certificates: bool,
    /// Custom CA certificates (PEM format)
    pub ca_certificates: Option<Vec<String>>,
    /// Client certificate (PEM format)
    pub client_certificate: Option<String>,
    /// Client key (PEM format)
    pub client_key: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_certificates: true,
            ca_certificates: None,
            client_certificate: None,
            client_key: None,
        }
    }
}

/// Server configuration for running a relay node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayServerConfig {
    /// Listen address
    pub listen_address: String,
    /// Maximum connections
    pub max_connections: u32,
    /// Storage path
    pub storage_path: String,
    /// Maximum storage size in bytes
    pub max_storage_size: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,
    /// TLS certificate path
    pub tls_cert_path: String,
    /// TLS key path
    pub tls_key_path: String,
    /// Rate limiting
    pub rate_limit: RateLimitConfig,
}

impl Default for RelayServerConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0:4433".to_string(),
            max_connections: 1000,
            storage_path: "./relay-data".to_string(),
            max_storage_size: 10 * 1024 * 1024 * 1024, // 10 GB
            cleanup_interval_secs: 3600,
            tls_cert_path: "./certs/relay.crt".to_string(),
            tls_key_path: "./certs/relay.key".to_string(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per second per IP
    pub requests_per_second: u32,
    /// Burst size
    pub burst_size: u32,
    /// Storage quota per IP in bytes
    pub storage_quota_per_ip: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: 50,
            storage_quota_per_ip: 100 * 1024 * 1024, // 100 MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RelayConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_retry_delay() {
        let config = RetryConfig::default();
        
        assert_eq!(config.delay_for_attempt(0).as_millis(), 100);
        assert_eq!(config.delay_for_attempt(1).as_millis(), 200);
        assert_eq!(config.delay_for_attempt(2).as_millis(), 400);
    }

    #[test]
    fn test_relay_node_info() {
        let node = RelayNodeInfo::new("node-1", "relay.example.com:4433", [0x42; 32]);
        assert_eq!(node.id, "node-1");
        assert_eq!(node.priority, 100);
    }
}
