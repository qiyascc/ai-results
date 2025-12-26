//! API handlers for Identity Service

use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::ServiceError;
use crate::AppState;

/// Configure API routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/identity")
            .route("/generate", web::post().to(generate_identity))
            .route("/rotate", web::post().to(rotate_identity))
            .route("/verify", web::post().to(verify_identity))
            .route("/prekeys", web::get().to(get_prekeys))
            .route("/prekeys", web::post().to(register_prekeys))
            .route("/bundle/{user_id}", web::get().to(get_bundle))
            .route("/health", web::get().to(health_check)),
    );
}

/// Generate identity request
#[derive(Debug, Deserialize)]
pub struct GenerateIdentityRequest {
    pub device_name: String,
    pub device_type: Option<String>,
}

/// Generate identity response
#[derive(Debug, Serialize)]
pub struct GenerateIdentityResponse {
    pub user_id: String,
    pub device_id: String,
    pub identity_key: String,
    pub fingerprint: String,
    pub signed_prekey: SignedPreKeyResponse,
    pub one_time_prekeys: Vec<OneTimePreKeyResponse>,
}

/// Signed prekey response
#[derive(Debug, Serialize)]
pub struct SignedPreKeyResponse {
    pub id: u32,
    pub public_key: String,
    pub signature: String,
}

/// One-time prekey response
#[derive(Debug, Serialize)]
pub struct OneTimePreKeyResponse {
    pub id: u32,
    pub public_key: String,
}

/// Generate a new identity
async fn generate_identity(
    state: web::Data<AppState>,
    req: web::Json<GenerateIdentityRequest>,
) -> ActixResult<HttpResponse, ServiceError> {
    info!("Generating new identity for device: {}", req.device_name);

    let result = state.service.generate_identity(&req.device_name).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// Rotate identity request
#[derive(Debug, Deserialize)]
pub struct RotateIdentityRequest {
    pub user_id: String,
    pub device_id: String,
    pub reason: Option<String>,
}

/// Rotate identity response
#[derive(Debug, Serialize)]
pub struct RotateIdentityResponse {
    pub new_identity_key: String,
    pub new_fingerprint: String,
    pub rotation_proof: RotationProofResponse,
}

/// Rotation proof
#[derive(Debug, Serialize)]
pub struct RotationProofResponse {
    pub old_public_key: String,
    pub new_public_key: String,
    pub old_signature: String,
    pub new_signature: String,
    pub timestamp: i64,
    pub commitment: String,
}

/// Rotate identity
async fn rotate_identity(
    state: web::Data<AppState>,
    req: web::Json<RotateIdentityRequest>,
) -> ActixResult<HttpResponse, ServiceError> {
    info!("Rotating identity for user: {}", req.user_id);

    let result = state
        .service
        .rotate_identity(&req.user_id, &req.device_id)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// Verify identity request
#[derive(Debug, Deserialize)]
pub struct VerifyIdentityRequest {
    pub user_id: String,
    pub identity_key: String,
    pub signature: String,
    pub message: String,
}

/// Verify identity response
#[derive(Debug, Serialize)]
pub struct VerifyIdentityResponse {
    pub valid: bool,
    pub fingerprint: String,
    pub trusted: bool,
}

/// Verify an identity
async fn verify_identity(
    state: web::Data<AppState>,
    req: web::Json<VerifyIdentityRequest>,
) -> ActixResult<HttpResponse, ServiceError> {
    debug!("Verifying identity for user: {}", req.user_id);

    let result = state
        .service
        .verify_identity(&req.user_id, &req.identity_key, &req.signature, &req.message)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// Get prekeys response
#[derive(Debug, Serialize)]
pub struct GetPreKeysResponse {
    pub count: usize,
    pub needs_replenishment: bool,
    pub signed_prekey_id: u32,
}

/// Get prekey status
async fn get_prekeys(
    state: web::Data<AppState>,
    query: web::Query<UserIdQuery>,
) -> ActixResult<HttpResponse, ServiceError> {
    debug!("Getting prekeys for user: {}", query.user_id);

    let result = state.service.get_prekey_status(&query.user_id).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// User ID query parameter
#[derive(Debug, Deserialize)]
pub struct UserIdQuery {
    pub user_id: String,
}

/// Register prekeys request
#[derive(Debug, Deserialize)]
pub struct RegisterPreKeysRequest {
    pub user_id: String,
    pub device_id: String,
    pub signed_prekey: Option<SignedPreKeyInput>,
    pub one_time_prekeys: Vec<OneTimePreKeyInput>,
}

/// Signed prekey input
#[derive(Debug, Deserialize)]
pub struct SignedPreKeyInput {
    pub id: u32,
    pub public_key: String,
    pub signature: String,
}

/// One-time prekey input
#[derive(Debug, Deserialize)]
pub struct OneTimePreKeyInput {
    pub id: u32,
    pub public_key: String,
}

/// Register prekeys response
#[derive(Debug, Serialize)]
pub struct RegisterPreKeysResponse {
    pub registered: usize,
    pub total_count: usize,
}

/// Register new prekeys
async fn register_prekeys(
    state: web::Data<AppState>,
    req: web::Json<RegisterPreKeysRequest>,
) -> ActixResult<HttpResponse, ServiceError> {
    info!(
        "Registering {} prekeys for user: {}",
        req.one_time_prekeys.len(),
        req.user_id
    );

    let result = state
        .service
        .register_prekeys(&req.user_id, &req.device_id, &req.one_time_prekeys)
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// Get prekey bundle response
#[derive(Debug, Serialize)]
pub struct PreKeyBundleResponse {
    pub user_id: String,
    pub device_id: String,
    pub identity_key: String,
    pub signed_prekey: SignedPreKeyResponse,
    pub one_time_prekey: Option<OneTimePreKeyResponse>,
}

/// Get prekey bundle for a user
async fn get_bundle(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<DeviceIdQuery>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_id = path.into_inner();
    debug!("Getting bundle for user: {}", user_id);

    let result = state
        .service
        .get_prekey_bundle(&user_id, query.device_id.as_deref())
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// Device ID query parameter
#[derive(Debug, Deserialize)]
pub struct DeviceIdQuery {
    pub device_id: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

/// Health check endpoint
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0, // Would track actual uptime
    })
}
