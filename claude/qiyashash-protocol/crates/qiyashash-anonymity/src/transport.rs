//! Anonymous transport layer

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::config::{AnonymityConfig, TransportTypeConfig};
use crate::error::{AnonymityError, Result};

/// Transport type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportType {
    /// Direct connection
    Direct,
    /// Tor network
    Tor,
    /// I2P network
    I2P,
}

/// Anonymous transport trait
#[async_trait]
pub trait AnonymousTransport: Send + Sync {
    /// Connect to a destination
    async fn connect(&self, destination: &str) -> Result<Box<dyn Connection>>;

    /// Get transport type
    fn transport_type(&self) -> TransportType;

    /// Check if transport is available
    async fn is_available(&self) -> bool;

    /// Get circuit info (for debugging)
    fn circuit_info(&self) -> Option<CircuitInfo>;
}

/// Connection trait
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send data
    async fn send(&mut self, data: &[u8]) -> Result<()>;

    /// Receive data
    async fn receive(&mut self) -> Result<Vec<u8>>;

    /// Close connection
    async fn close(&mut self) -> Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;
}

/// Circuit information
#[derive(Clone, Debug)]
pub struct CircuitInfo {
    /// Circuit ID
    pub id: String,
    /// Number of hops
    pub hops: usize,
    /// Exit node (if applicable)
    pub exit_node: Option<String>,
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create transport from configuration
pub fn create_transport(config: &AnonymityConfig) -> Result<Arc<dyn AnonymousTransport>> {
    match &config.transport.transport_type {
        TransportTypeConfig::Direct => Ok(Arc::new(DirectTransport::new())),
        
        #[cfg(feature = "tor")]
        TransportTypeConfig::Tor(tor_config) => {
            Ok(Arc::new(crate::tor::TorTransport::new(tor_config.clone())?))
        }
        
        #[cfg(not(feature = "tor"))]
        TransportTypeConfig::Tor(_) => {
            Err(AnonymityError::TorUnavailable("Tor feature not enabled".to_string()))
        }
        
        #[cfg(feature = "i2p")]
        TransportTypeConfig::I2P(i2p_config) => {
            Ok(Arc::new(crate::i2p::I2PTransport::new(i2p_config.clone())?))
        }
        
        #[cfg(not(feature = "i2p"))]
        TransportTypeConfig::I2P(_) => {
            Err(AnonymityError::I2PUnavailable("I2P feature not enabled".to_string()))
        }
    }
}

/// Direct transport (no anonymity)
pub struct DirectTransport;

impl DirectTransport {
    /// Create new direct transport
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AnonymousTransport for DirectTransport {
    async fn connect(&self, destination: &str) -> Result<Box<dyn Connection>> {
        debug!("Direct connection to {}", destination);
        
        // Parse destination
        let stream = tokio::net::TcpStream::connect(destination)
            .await
            .map_err(|e| AnonymityError::ConnectionFailed(e.to_string()))?;
        
        Ok(Box::new(DirectConnection { stream: Some(stream) }))
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Direct
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn circuit_info(&self) -> Option<CircuitInfo> {
        None
    }
}

/// Direct TCP connection
struct DirectConnection {
    stream: Option<tokio::net::TcpStream>,
}

#[async_trait]
impl Connection for DirectConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        if let Some(ref mut stream) = self.stream {
            stream.write_all(data)
                .await
                .map_err(|e| AnonymityError::Transport(e.to_string()))?;
            Ok(())
        } else {
            Err(AnonymityError::Transport("Not connected".to_string()))
        }
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        
        if let Some(ref mut stream) = self.stream {
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf)
                .await
                .map_err(|e| AnonymityError::Transport(e.to_string()))?;
            buf.truncate(n);
            Ok(buf)
        } else {
            Err(AnonymityError::Transport("Not connected".to_string()))
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.stream = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_transport_type() {
        let transport = DirectTransport::new();
        assert_eq!(transport.transport_type(), TransportType::Direct);
    }

    #[tokio::test]
    async fn test_direct_available() {
        let transport = DirectTransport::new();
        assert!(transport.is_available().await);
    }
}
