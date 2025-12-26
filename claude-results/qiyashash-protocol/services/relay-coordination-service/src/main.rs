//! Relay Coordination Service
//! 
//! Coordinates relay nodes for offline message delivery in QiyasHash.
//! Manages relay node registration, health monitoring, and load balancing.

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use chrono::{DateTime, Utc};
use clap::Parser;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

mod error;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "relay-coordination-service")]
#[command(about = "QiyasHash Relay Coordination Service")]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value = "8085")]
    port: u16,

    /// Health check interval in seconds
    #[arg(long, default_value = "30")]
    health_interval: u64,

    /// Node timeout in seconds
    #[arg(long, default_value = "120")]
    node_timeout: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Relay node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNode {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub public_key: String,
    pub region: Option<String>,
    pub capacity: u32,
    pub current_load: u32,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: NodeStatus,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Active,
    Degraded,
    Offline,
    Maintenance,
}

/// Application state
pub struct AppState {
    pub nodes: Arc<DashMap<String, RelayNode>>,
    pub node_timeout: Duration,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    active_nodes: usize,
}

async fn health_check(state: web::Data<AppState>) -> HttpResponse {
    let active_count = state.nodes.iter()
        .filter(|n| n.status == NodeStatus::Active)
        .count();
    
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "relay-coordination-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        active_nodes: active_count,
    })
}

/// Register node request
#[derive(Deserialize)]
struct RegisterNodeRequest {
    address: String,
    port: u16,
    public_key: String,
    region: Option<String>,
    capacity: u32,
}

/// Register response
#[derive(Serialize)]
struct RegisterResponse {
    node_id: String,
    heartbeat_interval: u64,
}

async fn register_node(
    state: web::Data<AppState>,
    body: web::Json<RegisterNodeRequest>,
) -> HttpResponse {
    let node_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let node = RelayNode {
        id: node_id.clone(),
        address: body.address.clone(),
        port: body.port,
        public_key: body.public_key.clone(),
        region: body.region.clone(),
        capacity: body.capacity,
        current_load: 0,
        registered_at: now,
        last_heartbeat: now,
        status: NodeStatus::Active,
    };

    state.nodes.insert(node_id.clone(), node);
    info!("Registered relay node: {} at {}:{}", node_id, body.address, body.port);

    HttpResponse::Created().json(RegisterResponse {
        node_id,
        heartbeat_interval: 30,
    })
}

/// Heartbeat request
#[derive(Deserialize)]
struct HeartbeatRequest {
    current_load: u32,
    status: Option<NodeStatus>,
}

async fn heartbeat(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<HeartbeatRequest>,
) -> HttpResponse {
    let node_id = path.into_inner();

    if let Some(mut node) = state.nodes.get_mut(&node_id) {
        node.last_heartbeat = Utc::now();
        node.current_load = body.current_load;
        if let Some(status) = body.status {
            node.status = status;
        }
        HttpResponse::Ok().json(serde_json::json!({
            "acknowledged": true
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "Node not found"
        }))
    }
}

/// Unregister node
async fn unregister_node(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let node_id = path.into_inner();

    if state.nodes.remove(&node_id).is_some() {
        info!("Unregistered relay node: {}", node_id);
        HttpResponse::Ok().json(serde_json::json!({
            "unregistered": true
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "Node not found"
        }))
    }
}

/// List nodes query
#[derive(Deserialize)]
struct ListNodesQuery {
    region: Option<String>,
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

/// List nodes response
#[derive(Serialize)]
struct ListNodesResponse {
    nodes: Vec<RelayNode>,
    count: usize,
}

async fn list_nodes(
    state: web::Data<AppState>,
    query: web::Query<ListNodesQuery>,
) -> HttpResponse {
    let nodes: Vec<RelayNode> = state.nodes.iter()
        .map(|entry| entry.value().clone())
        .filter(|node| {
            if let Some(ref region) = query.region {
                if node.region.as_ref() != Some(region) {
                    return false;
                }
            }
            if let Some(ref status) = query.status {
                let target_status = match status.as_str() {
                    "active" => NodeStatus::Active,
                    "degraded" => NodeStatus::Degraded,
                    "offline" => NodeStatus::Offline,
                    "maintenance" => NodeStatus::Maintenance,
                    _ => return true,
                };
                if node.status != target_status {
                    return false;
                }
            }
            true
        })
        .take(query.limit)
        .collect();

    HttpResponse::Ok().json(ListNodesResponse {
        count: nodes.len(),
        nodes,
    })
}

/// Get best relay nodes for a recipient
#[derive(Deserialize)]
struct GetRelaysQuery {
    recipient_id: String,
    count: Option<usize>,
    region: Option<String>,
}

#[derive(Serialize)]
struct GetRelaysResponse {
    relays: Vec<RelayNode>,
}

async fn get_relays(
    state: web::Data<AppState>,
    query: web::Query<GetRelaysQuery>,
) -> HttpResponse {
    let count = query.count.unwrap_or(3);

    // Get active nodes sorted by load
    let mut candidates: Vec<RelayNode> = state.nodes.iter()
        .filter(|entry| {
            let node = entry.value();
            node.status == NodeStatus::Active &&
            node.current_load < node.capacity &&
            query.region.as_ref().map_or(true, |r| node.region.as_ref() == Some(r))
        })
        .map(|entry| entry.value().clone())
        .collect();

    // Sort by load ratio (lowest first)
    candidates.sort_by(|a, b| {
        let ratio_a = a.current_load as f64 / a.capacity as f64;
        let ratio_b = b.current_load as f64 / b.capacity as f64;
        ratio_a.partial_cmp(&ratio_b).unwrap()
    });

    let relays: Vec<RelayNode> = candidates.into_iter().take(count).collect();

    HttpResponse::Ok().json(GetRelaysResponse { relays })
}

/// Background task to check node health
async fn health_check_task(state: web::Data<AppState>, timeout: Duration) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let now = Utc::now();
        let mut to_remove = Vec::new();

        for entry in state.nodes.iter() {
            let node = entry.value();
            let age = now.signed_duration_since(node.last_heartbeat);

            if age > chrono::Duration::from_std(timeout).unwrap() {
                if node.status == NodeStatus::Active {
                    warn!("Node {} timed out, marking as offline", node.id);
                }
                to_remove.push(entry.key().clone());
            }
        }

        for node_id in to_remove {
            if let Some(mut node) = state.nodes.get_mut(&node_id) {
                node.status = NodeStatus::Offline;
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting Relay Coordination Service");

    let node_timeout = Duration::from_secs(args.node_timeout);
    let app_state = web::Data::new(AppState {
        nodes: Arc::new(DashMap::new()),
        node_timeout,
    });

    // Spawn health check task
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        health_check_task(state_clone, node_timeout).await;
    });

    info!("Binding to {}:{}", args.host, args.port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .route("/api/v1/health", web::get().to(health_check))
            .route("/api/v1/nodes", web::post().to(register_node))
            .route("/api/v1/nodes", web::get().to(list_nodes))
            .route("/api/v1/nodes/{node_id}/heartbeat", web::post().to(heartbeat))
            .route("/api/v1/nodes/{node_id}", web::delete().to(unregister_node))
            .route("/api/v1/relays", web::get().to(get_relays))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}
