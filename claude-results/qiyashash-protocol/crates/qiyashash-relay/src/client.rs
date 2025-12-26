//! Relay client for distributing and retrieving message blobs

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use tracing::{debug, info, warn, error};

use crate::config::{RelayConfig, RelayNodeInfo};
use crate::error::{RelayError, Result};
use crate::storage::BlobMetadata;

/// Blob distribution result
#[derive(Clone, Debug)]
pub struct DistributionResult {
    /// Blob ID
    pub blob_id: String,
    /// Relays where blob parts were stored
    pub relays: Vec<String>,
    /// Retrieval tokens for each relay
    pub retrieval_tokens: HashMap<String, String>,
}

/// Relay client for blob distribution
pub struct RelayClient {
    config: RelayConfig,
    connections: RwLock<HashMap<String, RelayConnection>>,
}

/// Connection to a relay node
struct RelayConnection {
    node: RelayNodeInfo,
    connected: bool,
    last_error: Option<String>,
}

impl RelayClient {
    /// Create a new relay client
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Connect to relay nodes
    pub async fn connect(&self) -> Result<()> {
        info!("Connecting to {} relay nodes", self.config.relay_nodes.len());

        let mut connections = self.connections.write();
        
        for node in &self.config.relay_nodes {
            // Attempt connection
            match self.connect_to_node(node).await {
                Ok(conn) => {
                    debug!("Connected to relay {}", node.id);
                    connections.insert(node.id.clone(), conn);
                }
                Err(e) => {
                    warn!("Failed to connect to relay {}: {}", node.id, e);
                    connections.insert(node.id.clone(), RelayConnection {
                        node: node.clone(),
                        connected: false,
                        last_error: Some(e.to_string()),
                    });
                }
            }
        }

        let connected_count = connections.values().filter(|c| c.connected).count();
        
        if connected_count < self.config.relay_count {
            warn!(
                "Only {} of {} required relays available",
                connected_count, self.config.relay_count
            );
        }

        info!("Connected to {} relay nodes", connected_count);
        Ok(())
    }

    async fn connect_to_node(&self, node: &RelayNodeInfo) -> Result<RelayConnection> {
        // In production, establish QUIC connection
        // For now, simulate connection
        Ok(RelayConnection {
            node: node.clone(),
            connected: true,
            last_error: None,
        })
    }

    /// Distribute a blob across multiple relays
    pub async fn distribute(&self, blob_id: &str, data: Vec<u8>) -> Result<DistributionResult> {
        debug!("Distributing blob {} ({} bytes)", blob_id, data.len());

        // Select relays
        let selected_relays = self.select_relays(self.config.relay_count)?;

        // Split data into parts
        let parts = self.split_data(&data, selected_relays.len());

        let mut retrieval_tokens = HashMap::new();
        let mut successful_relays = Vec::new();

        // Store each part on a different relay
        for (relay, part) in selected_relays.iter().zip(parts.iter()) {
            let part_id = format!("{}:{}", blob_id, relay.id);
            
            match self.store_on_relay(&relay.id, &part_id, part.clone()).await {
                Ok(token) => {
                    retrieval_tokens.insert(relay.id.clone(), token);
                    successful_relays.push(relay.id.clone());
                }
                Err(e) => {
                    warn!("Failed to store on relay {}: {}", relay.id, e);
                }
            }
        }

        // Check if enough parts were stored
        if successful_relays.len() < self.config.relay_count / 2 + 1 {
            return Err(RelayError::NotEnoughRelays {
                have: successful_relays.len(),
                need: self.config.relay_count / 2 + 1,
            });
        }

        info!(
            "Distributed blob {} to {} relays",
            blob_id,
            successful_relays.len()
        );

        Ok(DistributionResult {
            blob_id: blob_id.to_string(),
            relays: successful_relays,
            retrieval_tokens,
        })
    }

    /// Retrieve a blob from relays
    pub async fn retrieve(&self, distribution: &DistributionResult) -> Result<Vec<u8>> {
        debug!("Retrieving blob {}", distribution.blob_id);

        let mut parts = Vec::new();

        for relay_id in &distribution.relays {
            if let Some(token) = distribution.retrieval_tokens.get(relay_id) {
                let part_id = format!("{}:{}", distribution.blob_id, relay_id);
                
                match self.retrieve_from_relay(relay_id, &part_id, token).await {
                    Ok(data) => {
                        parts.push(data);
                    }
                    Err(e) => {
                        warn!("Failed to retrieve from relay {}: {}", relay_id, e);
                    }
                }
            }
        }

        // Reconstruct from parts
        if parts.is_empty() {
            return Err(RelayError::NotEnoughRelays {
                have: 0,
                need: 1,
            });
        }

        let data = self.reconstruct_data(&parts)?;
        
        info!(
            "Retrieved blob {} ({} bytes) from {} relays",
            distribution.blob_id,
            data.len(),
            parts.len()
        );

        Ok(data)
    }

    /// Delete a blob from all relays
    pub async fn delete(&self, distribution: &DistributionResult) -> Result<()> {
        debug!("Deleting blob {} from relays", distribution.blob_id);

        for relay_id in &distribution.relays {
            let part_id = format!("{}:{}", distribution.blob_id, relay_id);
            
            if let Err(e) = self.delete_from_relay(relay_id, &part_id).await {
                warn!("Failed to delete from relay {}: {}", relay_id, e);
            }
        }

        Ok(())
    }

    /// Get connected relay count
    pub fn connected_count(&self) -> usize {
        self.connections.read().values().filter(|c| c.connected).count()
    }

    // Helper methods

    fn select_relays(&self, count: usize) -> Result<Vec<RelayNodeInfo>> {
        let connections = self.connections.read();
        
        let mut available: Vec<_> = connections
            .values()
            .filter(|c| c.connected)
            .map(|c| c.node.clone())
            .collect();

        if available.len() < count {
            return Err(RelayError::NotEnoughRelays {
                have: available.len(),
                need: count,
            });
        }

        // Sort by priority and shuffle within same priority
        available.sort_by_key(|n| n.priority);
        
        // Take required count
        available.truncate(count);
        
        // Shuffle for randomness
        let mut rng = rand::thread_rng();
        available.shuffle(&mut rng);

        Ok(available)
    }

    fn split_data(&self, data: &[u8], parts: usize) -> Vec<Vec<u8>> {
        // Simple splitting - in production use Reed-Solomon
        let part_size = (data.len() + parts - 1) / parts;
        
        data.chunks(part_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    fn reconstruct_data(&self, parts: &[Vec<u8>]) -> Result<Vec<u8>> {
        // Simple reconstruction - in production use Reed-Solomon
        Ok(parts.concat())
    }

    async fn store_on_relay(&self, relay_id: &str, part_id: &str, data: Vec<u8>) -> Result<String> {
        // In production, send via QUIC
        // Return retrieval token
        let token = format!("token-{}-{}", relay_id, part_id);
        Ok(token)
    }

    async fn retrieve_from_relay(&self, relay_id: &str, part_id: &str, token: &str) -> Result<Vec<u8>> {
        // In production, request via QUIC
        Err(RelayError::NotConnected)
    }

    async fn delete_from_relay(&self, relay_id: &str, part_id: &str) -> Result<()> {
        // In production, send delete request via QUIC
        Ok(())
    }
}

/// Builder for RelayClient
pub struct RelayClientBuilder {
    config: RelayConfig,
}

impl RelayClientBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            config: RelayConfig::default(),
        }
    }

    /// Add relay node
    pub fn add_relay(mut self, node: RelayNodeInfo) -> Self {
        self.config.relay_nodes.push(node);
        self
    }

    /// Set relay count
    pub fn relay_count(mut self, count: usize) -> Self {
        self.config.relay_count = count;
        self
    }

    /// Set message expiry
    pub fn message_expiry_secs(mut self, secs: u64) -> Self {
        self.config.message_expiry_secs = secs;
        self
    }

    /// Build the client
    pub fn build(self) -> RelayClient {
        RelayClient::new(self.config)
    }
}

impl Default for RelayClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let client = RelayClientBuilder::new()
            .relay_count(3)
            .message_expiry_secs(86400)
            .build();

        assert_eq!(client.config.relay_count, 3);
    }

    #[test]
    fn test_split_data() {
        let client = RelayClient::new(RelayConfig::default());
        let data = vec![0u8; 100];
        
        let parts = client.split_data(&data, 5);
        
        assert_eq!(parts.len(), 5);
        assert_eq!(parts.iter().map(|p| p.len()).sum::<usize>(), 100);
    }

    #[test]
    fn test_reconstruct_data() {
        let client = RelayClient::new(RelayConfig::default());
        let original = vec![0x42u8; 100];
        
        let parts = client.split_data(&original, 5);
        let reconstructed = client.reconstruct_data(&parts).unwrap();
        
        assert_eq!(original, reconstructed);
    }
}
