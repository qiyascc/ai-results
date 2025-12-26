//! Relay server for storing and serving message blobs

use std::sync::Arc;
use std::net::SocketAddr;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error, instrument};

use crate::config::RelayServerConfig;
use crate::error::{RelayError, Result};
use crate::storage::{RelayStorage, MemoryRelayStorage, StorageStats};

/// Relay server events
#[derive(Clone, Debug)]
pub enum ServerEvent {
    /// Server started
    Started { address: SocketAddr },
    /// Client connected
    ClientConnected { peer: SocketAddr },
    /// Client disconnected
    ClientDisconnected { peer: SocketAddr },
    /// Blob stored
    BlobStored { id: String, size: usize },
    /// Blob retrieved
    BlobRetrieved { id: String },
    /// Blob deleted
    BlobDeleted { id: String },
    /// Error occurred
    Error { message: String },
    /// Server stopping
    Stopping,
}

/// Relay server state
enum ServerState {
    /// Not started
    Stopped,
    /// Starting up
    Starting,
    /// Running
    Running,
    /// Shutting down
    ShuttingDown,
}

/// Relay server
pub struct RelayServer {
    config: RelayServerConfig,
    storage: Arc<dyn RelayStorage>,
    state: RwLock<ServerState>,
    event_tx: Option<mpsc::Sender<ServerEvent>>,
}

impl RelayServer {
    /// Create a new relay server
    pub fn new(config: RelayServerConfig) -> Self {
        let storage = Arc::new(MemoryRelayStorage::new(config.max_storage_size));
        
        Self {
            config,
            storage,
            state: RwLock::new(ServerState::Stopped),
            event_tx: None,
        }
    }

    /// Create with custom storage
    pub fn with_storage(config: RelayServerConfig, storage: Arc<dyn RelayStorage>) -> Self {
        Self {
            config,
            storage,
            state: RwLock::new(ServerState::Stopped),
            event_tx: None,
        }
    }

    /// Set event channel
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<ServerEvent>) {
        self.event_tx = Some(tx);
    }

    /// Start the server
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        {
            let mut state = self.state.write();
            match *state {
                ServerState::Running => return Ok(()),
                ServerState::ShuttingDown => {
                    return Err(RelayError::Internal("Server is shutting down".to_string()));
                }
                _ => *state = ServerState::Starting,
            }
        }

        info!("Starting relay server on {}", self.config.listen_address);

        // Parse listen address
        let addr: SocketAddr = self.config.listen_address.parse()
            .map_err(|e| RelayError::Internal(format!("Invalid address: {}", e)))?;

        // In production, start QUIC server here
        // For now, simulate startup

        *self.state.write() = ServerState::Running;
        self.emit_event(ServerEvent::Started { address: addr }).await;

        // Start cleanup task
        self.spawn_cleanup_task();

        info!("Relay server started");
        Ok(())
    }

    /// Stop the server
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        *self.state.write() = ServerState::ShuttingDown;
        self.emit_event(ServerEvent::Stopping).await;

        // In production, close all connections
        
        *self.state.write() = ServerState::Stopped;
        info!("Relay server stopped");
        Ok(())
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        matches!(*self.state.read(), ServerState::Running)
    }

    /// Store a blob
    #[instrument(skip(self, data))]
    pub async fn store(&self, id: &str, data: Vec<u8>, expiry_secs: u64) -> Result<String> {
        if !self.is_running() {
            return Err(RelayError::Internal("Server not running".to_string()));
        }

        let size = data.len();
        
        if size > self.config.max_storage_size as usize {
            return Err(RelayError::BlobTooLarge {
                size,
                max: self.config.max_storage_size as usize,
            });
        }

        let metadata = self.storage.store(id, data, expiry_secs)?;
        
        self.emit_event(ServerEvent::BlobStored {
            id: id.to_string(),
            size,
        }).await;

        // Return retrieval token
        let token = self.generate_retrieval_token(id);
        Ok(token)
    }

    /// Retrieve a blob
    #[instrument(skip(self))]
    pub async fn retrieve(&self, id: &str, token: &str) -> Result<Vec<u8>> {
        if !self.is_running() {
            return Err(RelayError::Internal("Server not running".to_string()));
        }

        // Verify token
        if !self.verify_retrieval_token(id, token) {
            return Err(RelayError::InvalidBlob("Invalid retrieval token".to_string()));
        }

        match self.storage.retrieve(id)? {
            Some(blob) => {
                self.emit_event(ServerEvent::BlobRetrieved { id: id.to_string() }).await;
                Ok(blob.data)
            }
            None => Err(RelayError::BlobNotFound(id.to_string())),
        }
    }

    /// Delete a blob
    #[instrument(skip(self))]
    pub async fn delete(&self, id: &str) -> Result<bool> {
        if !self.is_running() {
            return Err(RelayError::Internal("Server not running".to_string()));
        }

        let deleted = self.storage.delete(id)?;
        
        if deleted {
            self.emit_event(ServerEvent::BlobDeleted { id: id.to_string() }).await;
        }

        Ok(deleted)
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats> {
        self.storage.stats()
    }

    // Helper methods

    async fn emit_event(&self, event: ServerEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    fn spawn_cleanup_task(&self) {
        let storage = Arc::clone(&self.storage);
        let interval = self.config.cleanup_interval_secs;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval)
            );
            
            loop {
                interval.tick().await;
                
                match storage.cleanup_expired() {
                    Ok(count) if count > 0 => {
                        info!("Cleaned up {} expired blobs", count);
                    }
                    Err(e) => {
                        error!("Cleanup failed: {}", e);
                    }
                    _ => {}
                }
            }
        });
    }

    fn generate_retrieval_token(&self, id: &str) -> String {
        use sha2::{Sha256, Digest};
        
        // In production, use HMAC with server secret
        let mut hasher = Sha256::new();
        hasher.update(b"relay-token-v1");
        hasher.update(id.as_bytes());
        hasher.update(&chrono::Utc::now().timestamp().to_be_bytes());
        
        let result = hasher.finalize();
        hex::encode(&result[..16])
    }

    fn verify_retrieval_token(&self, _id: &str, _token: &str) -> bool {
        // In production, verify HMAC
        // For now, always accept
        true
    }
}

/// Handle incoming request
#[derive(Clone, Debug)]
pub enum Request {
    /// Store blob
    Store {
        id: String,
        data: Vec<u8>,
        expiry_secs: u64,
    },
    /// Retrieve blob
    Retrieve {
        id: String,
        token: String,
    },
    /// Delete blob
    Delete {
        id: String,
    },
    /// Get stats
    Stats,
}

/// Response to request
#[derive(Clone, Debug)]
pub enum Response {
    /// Store successful
    Stored { token: String },
    /// Retrieved data
    Retrieved { data: Vec<u8> },
    /// Deleted
    Deleted { success: bool },
    /// Stats
    Stats { stats: StorageStats },
    /// Error
    Error { code: u32, message: String },
}

impl Response {
    /// Create error response
    pub fn error(code: u32, message: impl Into<String>) -> Self {
        Response::Error {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_lifecycle() {
        let config = RelayServerConfig::default();
        let server = RelayServer::new(config);

        assert!(!server.is_running());
        
        server.start().await.unwrap();
        assert!(server.is_running());
        
        server.stop().await.unwrap();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_store_retrieve() {
        let config = RelayServerConfig::default();
        let server = RelayServer::new(config);
        server.start().await.unwrap();

        // Store
        let token = server.store("test-1", vec![0x42; 100], 3600).await.unwrap();
        assert!(!token.is_empty());

        // Retrieve
        let data = server.retrieve("test-1", &token).await.unwrap();
        assert_eq!(data.len(), 100);

        // Delete
        assert!(server.delete("test-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_stats() {
        let config = RelayServerConfig::default();
        let server = RelayServer::new(config);
        server.start().await.unwrap();

        server.store("blob-1", vec![0x42; 100], 3600).await.unwrap();
        server.store("blob-2", vec![0x42; 200], 3600).await.unwrap();

        let stats = server.stats().unwrap();
        assert_eq!(stats.blob_count, 2);
        assert_eq!(stats.total_size, 300);
    }
}
