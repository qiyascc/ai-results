//! REST API handlers for Chain State Service

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ChainStateError;
use crate::service::{AppendRequest, ChainEntry, ChainState};
use crate::AppState;

/// Configure API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check))
            .route("/chains", web::post().to(create_chain))
            .route("/chains", web::get().to(list_chains))
            .route("/chains/{chain_id}", web::get().to(get_chain))
            .route("/chains/{chain_id}/entries", web::post().to(append_entry))
            .route("/chains/{chain_id}/entries", web::get().to(get_entries))
            .route("/chains/{chain_id}/entries/{sequence}", web::get().to(get_entry))
            .route("/chains/{chain_id}/verify", web::post().to(verify_chain)),
    );
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

/// Health check endpoint
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "chain-state-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create chain request
#[derive(Deserialize)]
struct CreateChainRequest {
    chain_id: String,
}

/// Create a new chain
async fn create_chain(
    state: web::Data<AppState>,
    body: web::Json<CreateChainRequest>,
) -> Result<HttpResponse, ChainStateError> {
    info!("Creating chain: {}", body.chain_id);
    let chain_state = state.chain_manager.create_chain(&body.chain_id)?;
    Ok(HttpResponse::Created().json(chain_state))
}

/// List chains query parameters
#[derive(Deserialize)]
struct ListChainsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    50
}

/// List chains response
#[derive(Serialize)]
struct ListChainsResponse {
    chains: Vec<ChainState>,
    count: usize,
}

/// List all chains
async fn list_chains(
    state: web::Data<AppState>,
    query: web::Query<ListChainsQuery>,
) -> Result<HttpResponse, ChainStateError> {
    let chains = state.chain_manager.list_chains(query.limit, query.offset)?;
    Ok(HttpResponse::Ok().json(ListChainsResponse {
        count: chains.len(),
        chains,
    }))
}

/// Get chain state
async fn get_chain(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ChainStateError> {
    let chain_id = path.into_inner();
    let chain_state = state.chain_manager.get_chain(&chain_id)?;
    Ok(HttpResponse::Ok().json(chain_state))
}

/// Append entry request
#[derive(Deserialize)]
struct AppendEntryRequest {
    content_hash: String,
    expected_previous_hash: Option<String>,
    metadata: Option<serde_json::Value>,
}

/// Append entry to chain
async fn append_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<AppendEntryRequest>,
) -> Result<HttpResponse, ChainStateError> {
    let chain_id = path.into_inner();
    
    let request = AppendRequest {
        chain_id,
        content_hash: body.content_hash.clone(),
        expected_previous_hash: body.expected_previous_hash.clone(),
        metadata: body.metadata.clone(),
    };

    let entry = state.chain_manager.append_entry(request)?;
    Ok(HttpResponse::Created().json(entry))
}

/// Get entries query parameters
#[derive(Deserialize)]
struct GetEntriesQuery {
    #[serde(default = "default_from")]
    from: u64,
    #[serde(default = "default_to")]
    to: u64,
}

fn default_from() -> u64 {
    1
}

fn default_to() -> u64 {
    100
}

/// Get entries response
#[derive(Serialize)]
struct GetEntriesResponse {
    entries: Vec<ChainEntry>,
    count: usize,
}

/// Get entries in range
async fn get_entries(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<GetEntriesQuery>,
) -> Result<HttpResponse, ChainStateError> {
    let chain_id = path.into_inner();
    let entries = state.chain_manager.get_entries(&chain_id, query.from, query.to)?;
    
    Ok(HttpResponse::Ok().json(GetEntriesResponse {
        count: entries.len(),
        entries,
    }))
}

/// Path parameters for single entry
#[derive(Deserialize)]
struct EntryPath {
    chain_id: String,
    sequence: u64,
}

/// Get single entry
async fn get_entry(
    state: web::Data<AppState>,
    path: web::Path<EntryPath>,
) -> Result<HttpResponse, ChainStateError> {
    let entry = state.chain_manager.get_entry(&path.chain_id, path.sequence)?;
    Ok(HttpResponse::Ok().json(entry))
}

/// Verify chain response
#[derive(Serialize)]
struct VerifyChainResponse {
    chain_id: String,
    valid: bool,
    message: String,
}

/// Verify chain integrity
async fn verify_chain(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ChainStateError> {
    let chain_id = path.into_inner();
    let valid = state.chain_manager.verify_chain(&chain_id)?;
    
    Ok(HttpResponse::Ok().json(VerifyChainResponse {
        chain_id,
        valid,
        message: if valid {
            "Chain integrity verified".to_string()
        } else {
            "Chain integrity compromised".to_string()
        },
    }))
}
