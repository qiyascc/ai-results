//! DHT configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// DHT node configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtConfig {
    /// Listen addresses
    pub listen_addresses: Vec<String>,
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    /// Storage path
    pub storage_path: String,
    /// Maximum storage size in bytes
    pub max_storage_bytes: u64,
    /// Fragment count for Reed-Solomon
    pub fragment_count: usize,
    /// Fragment threshold for reconstruction
    pub fragment_threshold: usize,
    /// Message expiry duration
    pub message_expiry_secs: u64,
    /// Replication factor
    pub replication_factor: usize,
    /// Query timeout
    pub query_timeout_secs: u64,
    /// Connection timeout
    pub connection_timeout_secs: u64,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Gossipsub configuration
    pub gossipsub: GossipsubConfig,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".to_string(),
                "/ip4/0.0.0.0/udp/0/quic-v1".to_string(),
            ],
            bootstrap_nodes: Vec::new(),
            storage_path: "./dht_storage".to_string(),
            max_storage_bytes: 1024 * 1024 * 1024, // 1 GB
            fragment_count: 5,
            fragment_threshold: 3,
            message_expiry_secs: 30 * 24 * 3600, // 30 days
            replication_factor: 3,
            query_timeout_secs: 30,
            connection_timeout_secs: 10,
            enable_mdns: true,
            max_connections: 100,
            gossipsub: GossipsubConfig::default(),
        }
    }
}

impl DhtConfig {
    /// Create with custom storage path
    pub fn with_storage_path(path: impl Into<String>) -> Self {
        Self {
            storage_path: path.into(),
            ..Default::default()
        }
    }

    /// Add bootstrap node
    pub fn add_bootstrap_node(&mut self, addr: impl Into<String>) {
        self.bootstrap_nodes.push(addr.into());
    }

    /// Query timeout as Duration
    pub fn query_timeout(&self) -> Duration {
        Duration::from_secs(self.query_timeout_secs)
    }

    /// Connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Message expiry as Duration
    pub fn message_expiry(&self) -> Duration {
        Duration::from_secs(self.message_expiry_secs)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.fragment_threshold > self.fragment_count {
            return Err("fragment_threshold must be <= fragment_count".to_string());
        }
        if self.fragment_count == 0 {
            return Err("fragment_count must be > 0".to_string());
        }
        if self.replication_factor == 0 {
            return Err("replication_factor must be > 0".to_string());
        }
        Ok(())
    }
}

/// Gossipsub configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GossipsubConfig {
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Message validity duration in seconds
    pub message_validity_secs: u64,
    /// Maximum message size
    pub max_message_size: usize,
    /// Mesh parameters
    pub mesh_n: usize,
    pub mesh_n_low: usize,
    pub mesh_n_high: usize,
}

impl Default for GossipsubConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 1000,
            message_validity_secs: 300,
            max_message_size: 65536,
            mesh_n: 6,
            mesh_n_low: 4,
            mesh_n_high: 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DhtConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config() {
        let mut config = DhtConfig::default();
        config.fragment_threshold = 10;
        config.fragment_count = 5;
        assert!(config.validate().is_err());
    }
}
