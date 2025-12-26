//! Configuration for anonymity layer

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Anonymity layer configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymityConfig {
    /// Transport type
    pub transport: TransportConfig,
    /// Traffic obfuscation settings
    pub obfuscation: ObfuscationConfig,
    /// Cover traffic settings
    pub cover_traffic: CoverTrafficConfig,
}

impl Default for AnonymityConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            obfuscation: ObfuscationConfig::default(),
            cover_traffic: CoverTrafficConfig::default(),
        }
    }
}

/// Transport configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type to use
    pub transport_type: TransportTypeConfig,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportTypeConfig::Direct,
            connection_timeout_secs: 30,
            request_timeout_secs: 60,
            max_retries: 3,
        }
    }
}

/// Transport type configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransportTypeConfig {
    /// Direct connection (no anonymity)
    Direct,
    /// Tor network
    Tor(TorConfig),
    /// I2P network
    I2P(I2PConfig),
}

/// Tor-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TorConfig {
    /// Tor SOCKS proxy address
    pub socks_addr: String,
    /// Use bridges
    pub use_bridges: bool,
    /// Bridge lines
    pub bridges: Vec<String>,
    /// Circuit isolation (new circuit per destination)
    pub circuit_isolation: bool,
    /// Custom data directory
    pub data_dir: Option<String>,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            socks_addr: "127.0.0.1:9050".to_string(),
            use_bridges: false,
            bridges: Vec::new(),
            circuit_isolation: true,
            data_dir: None,
        }
    }
}

/// I2P-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct I2PConfig {
    /// SAM bridge address
    pub sam_addr: String,
    /// Tunnel length (hops)
    pub tunnel_length: u32,
    /// Tunnel quantity
    pub tunnel_quantity: u32,
    /// Enable backup tunnels
    pub backup_quantity: u32,
}

impl Default for I2PConfig {
    fn default() -> Self {
        Self {
            sam_addr: "127.0.0.1:7656".to_string(),
            tunnel_length: 3,
            tunnel_quantity: 2,
            backup_quantity: 1,
        }
    }
}

/// Traffic obfuscation configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObfuscationConfig {
    /// Enable traffic obfuscation
    pub enabled: bool,
    /// Minimum delay between messages (milliseconds)
    pub min_delay_ms: u64,
    /// Maximum delay between messages (milliseconds)
    pub max_delay_ms: u64,
    /// Padding size range (min, max bytes)
    pub padding_range: (usize, usize),
    /// Enable message batching
    pub batch_messages: bool,
    /// Batch window (milliseconds)
    pub batch_window_ms: u64,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_delay_ms: 100,
            max_delay_ms: 2000,
            padding_range: (64, 1024),
            batch_messages: true,
            batch_window_ms: 500,
        }
    }
}

/// Cover traffic configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverTrafficConfig {
    /// Enable cover traffic
    pub enabled: bool,
    /// Average messages per hour
    pub rate_per_hour: f64,
    /// Randomize timing using Poisson distribution
    pub poisson_timing: bool,
    /// Cover message size range (min, max bytes)
    pub size_range: (usize, usize),
}

impl Default for CoverTrafficConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate_per_hour: 10.0,
            poisson_timing: true,
            size_range: (256, 2048),
        }
    }
}

impl AnonymityConfig {
    /// Create configuration for maximum privacy
    pub fn maximum_privacy() -> Self {
        Self {
            transport: TransportConfig {
                transport_type: TransportTypeConfig::Tor(TorConfig::default()),
                ..Default::default()
            },
            obfuscation: ObfuscationConfig {
                enabled: true,
                min_delay_ms: 500,
                max_delay_ms: 5000,
                padding_range: (256, 4096),
                batch_messages: true,
                batch_window_ms: 1000,
            },
            cover_traffic: CoverTrafficConfig {
                enabled: true,
                rate_per_hour: 30.0,
                poisson_timing: true,
                size_range: (512, 4096),
            },
        }
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.transport.connection_timeout_secs)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.transport.request_timeout_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AnonymityConfig::default();
        assert!(matches!(config.transport.transport_type, TransportTypeConfig::Direct));
    }

    #[test]
    fn test_maximum_privacy() {
        let config = AnonymityConfig::maximum_privacy();
        assert!(config.obfuscation.enabled);
        assert!(config.cover_traffic.enabled);
    }
}
