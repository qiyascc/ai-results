//! DHT Peer Service
//! 
//! Distributed Hash Table peer for QiyasHash message storage and retrieval.
//! Uses libp2p Kademlia DHT for decentralized message distribution.

use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use clap::Parser;
use libp2p::{
    identity, kad, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod error;
mod peer;
mod storage;

use peer::DhtPeer;
use storage::MessageStore;

/// DHT Peer Service CLI arguments
#[derive(Parser, Debug)]
#[command(name = "dht-peer-service")]
#[command(about = "QiyasHash DHT Peer Service")]
struct Args {
    /// Listen address for P2P
    #[arg(long, default_value = "/ip4/0.0.0.0/tcp/4001")]
    listen_addr: String,

    /// HTTP API port for health checks
    #[arg(long, default_value = "4002")]
    api_port: u16,

    /// Bootstrap nodes (comma-separated multiaddrs)
    #[arg(long)]
    bootstrap: Option<String>,

    /// Storage path
    #[arg(long, default_value = "./data/dht")]
    storage_path: String,

    /// Enable mDNS for local peer discovery
    #[arg(long)]
    mdns: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Shared application state
pub struct AppState {
    pub peer: Arc<RwLock<DhtPeer>>,
    pub store: Arc<MessageStore>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    peer_id: String,
    connected_peers: usize,
}

async fn health_check(state: web::Data<AppState>) -> HttpResponse {
    let peer = state.peer.read().await;
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "dht-peer-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        peer_id: peer.local_peer_id().to_string(),
        connected_peers: peer.connected_peers_count(),
    })
}

#[derive(Serialize)]
struct PeerInfoResponse {
    peer_id: String,
    listen_addresses: Vec<String>,
    connected_peers: Vec<String>,
    stored_records: usize,
}

async fn peer_info(state: web::Data<AppState>) -> HttpResponse {
    let peer = state.peer.read().await;
    HttpResponse::Ok().json(PeerInfoResponse {
        peer_id: peer.local_peer_id().to_string(),
        listen_addresses: peer.listen_addresses().iter().map(|a| a.to_string()).collect(),
        connected_peers: peer.connected_peers().iter().map(|p| p.to_string()).collect(),
        stored_records: state.store.record_count(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting DHT Peer Service");
    info!("Storage path: {}", args.storage_path);

    // Initialize message store
    let store = Arc::new(MessageStore::new(&args.storage_path)?);

    // Parse bootstrap nodes
    let bootstrap_nodes: Vec<Multiaddr> = args
        .bootstrap
        .map(|s| {
            s.split(',')
                .filter_map(|addr| addr.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    info!("Bootstrap nodes: {:?}", bootstrap_nodes);

    // Create DHT peer
    let listen_addr: Multiaddr = args.listen_addr.parse()?;
    let peer = Arc::new(RwLock::new(
        DhtPeer::new(listen_addr, bootstrap_nodes, args.mdns, store.clone()).await?
    ));

    let app_state = web::Data::new(AppState {
        peer: peer.clone(),
        store: store.clone(),
    });

    // Start HTTP API server
    let api_port = args.api_port;
    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .route("/api/v1/health", web::get().to(health_check))
            .route("/api/v1/peer", web::get().to(peer_info))
    })
    .bind(("0.0.0.0", api_port))?
    .run();

    info!("HTTP API listening on port {}", api_port);

    // Run DHT peer event loop
    let peer_handle = {
        let peer = peer.clone();
        tokio::spawn(async move {
            loop {
                let mut peer_guard = peer.write().await;
                if let Err(e) = peer_guard.poll_once().await {
                    error!("DHT peer error: {}", e);
                }
                drop(peer_guard);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
    };

    // Wait for shutdown
    tokio::select! {
        result = http_server => {
            if let Err(e) = result {
                error!("HTTP server error: {}", e);
            }
        }
        _ = peer_handle => {
            warn!("DHT peer loop ended unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("DHT Peer Service shutting down");
    Ok(())
}
