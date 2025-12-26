//! DHT node implementation using libp2p
//!
//! Provides a Kademlia-based DHT node for fragment storage and retrieval.

use futures::StreamExt;
use libp2p::{
    gossipsub, identify, kad, mdns, noise, ping,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

use crate::config::DhtConfig;
use crate::error::{DhtError, Result};
use crate::fragment::{Fragment, FragmentId, MessageFragments};
use crate::storage::DhtStorage;

/// Events emitted by the DHT node
#[derive(Debug)]
pub enum DhtEvent {
    /// Node connected to network
    Connected { peer_count: usize },
    /// New peer discovered
    PeerDiscovered { peer_id: PeerId },
    /// Peer disconnected
    PeerDisconnected { peer_id: PeerId },
    /// Fragment stored successfully
    FragmentStored { fragment_id: FragmentId },
    /// Fragment retrieved
    FragmentRetrieved { fragment: Fragment },
    /// Fragment not found
    FragmentNotFound { fragment_id: FragmentId },
    /// Error occurred
    Error { message: String },
}

/// Commands to send to the DHT node
#[derive(Debug)]
enum DhtCommand {
    /// Store a fragment
    StoreFragment {
        fragment: Fragment,
        response: oneshot::Sender<Result<()>>,
    },
    /// Retrieve a fragment
    GetFragment {
        id: FragmentId,
        response: oneshot::Sender<Result<Option<Fragment>>>,
    },
    /// Store all fragments for a message
    StoreMessage {
        fragments: Vec<Fragment>,
        response: oneshot::Sender<Result<()>>,
    },
    /// Retrieve all fragments for a message
    GetMessage {
        message_id: String,
        data_shards: usize,
        parity_shards: usize,
        message_size: usize,
        response: oneshot::Sender<Result<Vec<u8>>>,
    },
    /// Get connected peer count
    GetPeerCount {
        response: oneshot::Sender<usize>,
    },
    /// Shutdown the node
    Shutdown,
}

/// Network behaviour combining Kademlia, Gossipsub, and other protocols
#[derive(NetworkBehaviour)]
struct QiyasHashBehaviour {
    /// Kademlia DHT
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    /// Gossipsub for pub/sub messaging
    gossipsub: gossipsub::Behaviour,
    /// mDNS for local peer discovery
    mdns: mdns::tokio::Behaviour,
    /// Identify protocol
    identify: identify::Behaviour,
    /// Ping for connection health
    ping: ping::Behaviour,
}

/// DHT node handle for interacting with the node
#[derive(Clone)]
pub struct DhtNode {
    /// Command sender
    command_tx: mpsc::Sender<DhtCommand>,
    /// Our peer ID
    peer_id: PeerId,
    /// Local storage
    storage: Arc<DhtStorage>,
    /// Configuration
    config: DhtConfig,
}

impl DhtNode {
    /// Create and start a new DHT node
    pub async fn start(config: DhtConfig, storage: DhtStorage) -> Result<(Self, mpsc::Receiver<DhtEvent>)> {
        config.validate().map_err(DhtError::Configuration)?;

        let storage = Arc::new(storage);
        let (command_tx, command_rx) = mpsc::channel(256);
        let (event_tx, event_rx) = mpsc::channel(256);

        // Generate identity
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());

        info!("Starting DHT node with peer ID: {}", peer_id);

        // Create swarm
        let swarm = Self::create_swarm(&config, local_key.clone())?;

        // Start event loop
        let storage_clone = storage.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            Self::run_event_loop(swarm, command_rx, event_tx, storage_clone, config_clone).await;
        });

        let node = Self {
            command_tx,
            peer_id,
            storage,
            config,
        };

        Ok((node, event_rx))
    }

    /// Create the libp2p swarm
    fn create_swarm(
        config: &DhtConfig,
        local_key: libp2p::identity::Keypair,
    ) -> Result<Swarm<QiyasHashBehaviour>> {
        let peer_id = PeerId::from(local_key.public());

        // Build swarm
        let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| DhtError::Network(e.to_string()))?
            .with_quic()
            .with_behaviour(|key| {
                // Kademlia
                let store = kad::store::MemoryStore::new(peer_id);
                let kademlia_config = kad::Config::default();
                let kademlia = kad::Behaviour::with_config(peer_id, store, kademlia_config);

                // Gossipsub
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_millis(config.gossipsub.heartbeat_interval_ms))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .build()
                    .expect("Valid gossipsub config");

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("Valid gossipsub behaviour");

                // mDNS
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    peer_id,
                )
                .expect("Valid mDNS behaviour");

                // Identify
                let identify = identify::Behaviour::new(identify::Config::new(
                    "/qiyashash/1.0.0".to_string(),
                    key.public(),
                ));

                // Ping
                let ping = ping::Behaviour::new(ping::Config::new());

                QiyasHashBehaviour {
                    kademlia,
                    gossipsub,
                    mdns,
                    identify,
                    ping,
                }
            })
            .map_err(|e| DhtError::Network(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        Ok(swarm)
    }

    /// Run the event loop
    async fn run_event_loop(
        mut swarm: Swarm<QiyasHashBehaviour>,
        mut command_rx: mpsc::Receiver<DhtCommand>,
        event_tx: mpsc::Sender<DhtEvent>,
        storage: Arc<DhtStorage>,
        config: DhtConfig,
    ) {
        // Start listening
        for addr in &config.listen_addresses {
            if let Ok(multiaddr) = addr.parse::<Multiaddr>() {
                if let Err(e) = swarm.listen_on(multiaddr.clone()) {
                    error!("Failed to listen on {}: {}", addr, e);
                }
            }
        }

        // Bootstrap
        for addr in &config.bootstrap_nodes {
            if let Ok(multiaddr) = addr.parse::<Multiaddr>() {
                if let Err(e) = swarm.dial(multiaddr.clone()) {
                    warn!("Failed to dial bootstrap node {}: {}", addr, e);
                }
            }
        }

        // Pending queries
        let mut pending_gets: HashMap<kad::QueryId, oneshot::Sender<Result<Option<Fragment>>>> =
            HashMap::new();

        loop {
            tokio::select! {
                // Handle swarm events
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {}", address);
                        }
                        SwarmEvent::Behaviour(QiyasHashBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                            for (peer_id, addr) in peers {
                                debug!("mDNS discovered: {} at {}", peer_id, addr);
                                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                                let _ = event_tx.send(DhtEvent::PeerDiscovered { peer_id }).await;
                            }
                        }
                        SwarmEvent::Behaviour(QiyasHashBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { id, result, .. })) => {
                            match result {
                                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                                    if let Some(response) = pending_gets.remove(&id) {
                                        match Fragment::from_bytes(&record.record.value) {
                                            Ok(fragment) => {
                                                let _ = response.send(Ok(Some(fragment)));
                                            }
                                            Err(e) => {
                                                let _ = response.send(Err(e));
                                            }
                                        }
                                    }
                                }
                                kad::QueryResult::GetRecord(Err(_)) => {
                                    if let Some(response) = pending_gets.remove(&id) {
                                        let _ = response.send(Ok(None));
                                    }
                                }
                                _ => {}
                            }
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            debug!("Connected to peer: {}", peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            debug!("Disconnected from peer: {}", peer_id);
                            let _ = event_tx.send(DhtEvent::PeerDisconnected { peer_id }).await;
                        }
                        _ => {}
                    }
                }

                // Handle commands
                Some(command) = command_rx.recv() => {
                    match command {
                        DhtCommand::StoreFragment { fragment, response } => {
                            // Store locally
                            let local_result = storage.store(&fragment);

                            // Store in DHT
                            let key = kad::RecordKey::new(&fragment.id.as_str());
                            if let Ok(value) = fragment.to_bytes() {
                                let record = kad::Record::new(key, value);
                                let _ = swarm.behaviour_mut().kademlia.put_record(
                                    record,
                                    kad::Quorum::One,
                                );
                            }

                            let _ = response.send(local_result);
                        }
                        DhtCommand::GetFragment { id, response } => {
                            // Try local first
                            if let Ok(Some(fragment)) = storage.get(&id) {
                                let _ = response.send(Ok(Some(fragment)));
                                continue;
                            }

                            // Query DHT
                            let key = kad::RecordKey::new(&id.as_str());
                            let query_id = swarm.behaviour_mut().kademlia.get_record(key);
                            pending_gets.insert(query_id, response);
                        }
                        DhtCommand::StoreMessage { fragments, response } => {
                            let mut all_ok = true;
                            for fragment in fragments {
                                if storage.store(&fragment).is_err() {
                                    all_ok = false;
                                }

                                let key = kad::RecordKey::new(&fragment.id.as_str());
                                if let Ok(value) = fragment.to_bytes() {
                                    let record = kad::Record::new(key, value);
                                    let _ = swarm.behaviour_mut().kademlia.put_record(
                                        record,
                                        kad::Quorum::One,
                                    );
                                }
                            }

                            if all_ok {
                                let _ = response.send(Ok(()));
                            } else {
                                let _ = response.send(Err(DhtError::Storage("Some fragments failed to store".to_string())));
                            }
                        }
                        DhtCommand::GetMessage { message_id, data_shards, parity_shards, message_size, response } => {
                            // Try to get from local storage first
                            match storage.get_message_fragments(&message_id) {
                                Ok(fragments) if fragments.len() >= data_shards => {
                                    let mut msg_fragments = MessageFragments::new_empty(
                                        &message_id,
                                        data_shards,
                                        parity_shards,
                                        message_size,
                                    );
                                    for frag in fragments {
                                        let _ = msg_fragments.add_fragment(frag);
                                    }
                                    let _ = response.send(msg_fragments.decode());
                                }
                                _ => {
                                    let _ = response.send(Err(DhtError::MessageNotFound(message_id)));
                                }
                            }
                        }
                        DhtCommand::GetPeerCount { response } => {
                            let count = swarm.connected_peers().count();
                            let _ = response.send(count);
                        }
                        DhtCommand::Shutdown => {
                            info!("DHT node shutting down");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Get our peer ID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Store a fragment
    pub async fn store_fragment(&self, fragment: Fragment) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(DhtCommand::StoreFragment { fragment, response: tx })
            .await
            .map_err(|_| DhtError::Internal("Channel closed".to_string()))?;
        rx.await.map_err(|_| DhtError::Internal("Response channel closed".to_string()))?
    }

    /// Retrieve a fragment
    pub async fn get_fragment(&self, id: &FragmentId) -> Result<Option<Fragment>> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(DhtCommand::GetFragment {
                id: id.clone(),
                response: tx,
            })
            .await
            .map_err(|_| DhtError::Internal("Channel closed".to_string()))?;
        rx.await.map_err(|_| DhtError::Internal("Response channel closed".to_string()))?
    }

    /// Store a complete message (all fragments)
    pub async fn store_message(&self, data: &[u8], message_id: &str) -> Result<()> {
        let fragments = MessageFragments::encode(
            message_id,
            data,
            self.config.fragment_count - 2, // data shards
            2, // parity shards
            self.config.message_expiry_secs,
        )?;

        let frags: Vec<Fragment> = fragments
            .fragments
            .into_iter()
            .flatten()
            .collect();

        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(DhtCommand::StoreMessage { fragments: frags, response: tx })
            .await
            .map_err(|_| DhtError::Internal("Channel closed".to_string()))?;
        rx.await.map_err(|_| DhtError::Internal("Response channel closed".to_string()))?
    }

    /// Retrieve and reconstruct a message
    pub async fn get_message(&self, message_id: &str, message_size: usize) -> Result<Vec<u8>> {
        let data_shards = self.config.fragment_count - 2;
        let parity_shards = 2;

        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(DhtCommand::GetMessage {
                message_id: message_id.to_string(),
                data_shards,
                parity_shards,
                message_size,
                response: tx,
            })
            .await
            .map_err(|_| DhtError::Internal("Channel closed".to_string()))?;
        rx.await.map_err(|_| DhtError::Internal("Response channel closed".to_string()))?
    }

    /// Get connected peer count
    pub async fn peer_count(&self) -> usize {
        let (tx, rx) = oneshot::channel();
        if self
            .command_tx
            .send(DhtCommand::GetPeerCount { response: tx })
            .await
            .is_ok()
        {
            rx.await.unwrap_or(0)
        } else {
            0
        }
    }

    /// Shutdown the node
    pub async fn shutdown(&self) -> Result<()> {
        self.command_tx
            .send(DhtCommand::Shutdown)
            .await
            .map_err(|_| DhtError::Internal("Channel closed".to_string()))?;
        Ok(())
    }

    /// Get local storage reference
    pub fn storage(&self) -> &DhtStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Integration tests would go here
    // They require actual network connectivity so are marked as ignored

    #[tokio::test]
    #[ignore]
    async fn test_node_start() {
        let dir = tempdir().unwrap();
        let config = DhtConfig::with_storage_path(dir.path().join("storage"));
        let storage = DhtStorage::open(dir.path().join("db"), 1024 * 1024).unwrap();

        let (node, _events) = DhtNode::start(config, storage).await.unwrap();
        assert!(!node.peer_id().to_string().is_empty());
        node.shutdown().await.unwrap();
    }
}
