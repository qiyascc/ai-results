//! DHT Peer implementation using libp2p

use crate::error::DhtError;
use crate::storage::MessageStore;
use libp2p::{
    identity::Keypair,
    kad::{self, store::MemoryStore, Behaviour as KademliaBehaviour, Config as KadConfig},
    mdns,
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, StreamProtocol,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Combined network behaviour
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "DhtBehaviourEvent")]
pub struct DhtBehaviour {
    pub kademlia: KademliaBehaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
}

/// Events from the network behaviour
#[derive(Debug)]
pub enum DhtBehaviourEvent {
    Kademlia(kad::Event),
    Mdns(mdns::Event),
}

impl From<kad::Event> for DhtBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        DhtBehaviourEvent::Kademlia(event)
    }
}

impl From<mdns::Event> for DhtBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        DhtBehaviourEvent::Mdns(event)
    }
}

/// DHT Peer node
pub struct DhtPeer {
    swarm: Swarm<DhtBehaviour>,
    local_peer_id: PeerId,
    listen_addresses: Vec<Multiaddr>,
    connected_peers: HashSet<PeerId>,
    message_store: Arc<MessageStore>,
}

impl DhtPeer {
    /// Create a new DHT peer
    pub async fn new(
        listen_addr: Multiaddr,
        bootstrap_nodes: Vec<Multiaddr>,
        enable_mdns: bool,
        message_store: Arc<MessageStore>,
    ) -> Result<Self, DhtError> {
        // Generate keypair
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        info!("Local peer ID: {}", local_peer_id);

        // Create swarm
        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| DhtError::NetworkError(format!("Failed to configure TCP: {}", e)))?
            .with_behaviour(|key| {
                // Kademlia DHT
                let local_peer_id = PeerId::from(key.public());
                let store = MemoryStore::new(local_peer_id);
                let mut kad_config = KadConfig::default();
                kad_config.set_protocol_names(vec![StreamProtocol::new("/qiyashash/kad/1.0.0")]);
                kad_config.set_query_timeout(Duration::from_secs(60));
                let kademlia = KademliaBehaviour::with_config(local_peer_id, store, kad_config);

                // mDNS for local discovery
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    local_peer_id,
                ).expect("Failed to create mDNS behaviour");

                DhtBehaviour { kademlia, mdns }
            })
            .map_err(|e| DhtError::NetworkError(format!("Failed to create behaviour: {}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let mut peer = Self {
            swarm,
            local_peer_id,
            listen_addresses: Vec::new(),
            connected_peers: HashSet::new(),
            message_store,
        };

        // Start listening
        peer.swarm.listen_on(listen_addr.clone())
            .map_err(|e| DhtError::NetworkError(format!("Failed to listen: {}", e)))?;
        
        info!("Listening on {}", listen_addr);

        // Connect to bootstrap nodes
        for addr in bootstrap_nodes {
            if let Err(e) = peer.dial(addr.clone()) {
                warn!("Failed to dial bootstrap node {}: {}", addr, e);
            }
        }

        Ok(peer)
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Get listen addresses
    pub fn listen_addresses(&self) -> &[Multiaddr] {
        &self.listen_addresses
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers.iter().cloned().collect()
    }

    /// Get connected peers count
    pub fn connected_peers_count(&self) -> usize {
        self.connected_peers.len()
    }

    /// Dial a peer
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), DhtError> {
        self.swarm.dial(addr.clone())
            .map_err(|e| DhtError::NetworkError(format!("Failed to dial {}: {}", addr, e)))?;
        Ok(())
    }

    /// Put a record in the DHT
    pub fn put_record(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), DhtError> {
        let record = kad::Record {
            key: kad::RecordKey::new(&key),
            value,
            publisher: Some(self.local_peer_id),
            expires: None,
        };

        self.swarm
            .behaviour_mut()
            .kademlia
            .put_record(record, kad::Quorum::One)
            .map_err(|e| DhtError::NetworkError(format!("Failed to put record: {:?}", e)))?;

        Ok(())
    }

    /// Get a record from the DHT
    pub fn get_record(&mut self, key: Vec<u8>) -> kad::QueryId {
        self.swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&key))
    }

    /// Poll the swarm for events
    pub async fn poll_once(&mut self) -> Result<(), DhtError> {
        use futures::StreamExt;

        tokio::select! {
            event = self.swarm.select_next_some() => {
                self.handle_event(event);
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }

        Ok(())
    }

    /// Handle swarm events
    fn handle_event(&mut self, event: SwarmEvent<DhtBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
                self.listen_addresses.push(address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, "/ip4/0.0.0.0/tcp/0".parse().unwrap());
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
            }
            SwarmEvent::Behaviour(DhtBehaviourEvent::Kademlia(kad_event)) => {
                self.handle_kademlia_event(kad_event);
            }
            SwarmEvent::Behaviour(DhtBehaviourEvent::Mdns(mdns_event)) => {
                self.handle_mdns_event(mdns_event);
            }
            _ => {}
        }
    }

    /// Handle Kademlia events
    fn handle_kademlia_event(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed { result, .. } => {
                match result {
                    kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                        debug!(
                            "Found record: key={}, value_len={}",
                            hex::encode(record.record.key.as_ref()),
                            record.record.value.len()
                        );
                        // Store locally
                        let _ = self.message_store.put(
                            record.record.key.as_ref(),
                            &record.record.value,
                            3600 * 24, // 24 hours TTL
                            record.record.publisher.map(|p| p.to_string()),
                        );
                    }
                    kad::QueryResult::GetRecord(Err(err)) => {
                        debug!("Failed to get record: {:?}", err);
                    }
                    kad::QueryResult::PutRecord(Ok(_)) => {
                        debug!("Successfully stored record in DHT");
                    }
                    kad::QueryResult::PutRecord(Err(err)) => {
                        warn!("Failed to store record: {:?}", err);
                    }
                    kad::QueryResult::Bootstrap(Ok(_)) => {
                        info!("Bootstrap successful");
                    }
                    kad::QueryResult::Bootstrap(Err(err)) => {
                        warn!("Bootstrap failed: {:?}", err);
                    }
                    _ => {}
                }
            }
            kad::Event::RoutingUpdated { peer, .. } => {
                debug!("Routing table updated for peer: {}", peer);
            }
            _ => {}
        }
    }

    /// Handle mDNS events
    fn handle_mdns_event(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    info!("mDNS: Discovered peer {} at {}", peer_id, addr);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                    if let Err(e) = self.dial(addr) {
                        debug!("Failed to dial discovered peer: {}", e);
                    }
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer_id, _) in peers {
                    debug!("mDNS: Peer {} expired", peer_id);
                }
            }
        }
    }
}
