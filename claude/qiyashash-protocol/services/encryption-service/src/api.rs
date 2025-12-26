//! API routes for encryption service

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::error::ServiceError;
use crate::service::EncryptionService;

/// Configure routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health))
            .route("/encrypt/generate-ephemeral", web::post().to(generate_ephemeral))
            .route("/encrypt/init-session", web::post().to(init_session))
            .route("/encrypt/message", web::post().to(encrypt_message))
            .route("/decrypt/message", web::post().to(decrypt_message))
            .route("/encrypt/derive-key", web::post().to(derive_key))
            .route("/encrypt/verify-chain", web::post().to(verify_chain))
            .route("/session/{session_id}", web::get().to(get_session))
    );
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    session_count: usize,
}

/// Health check endpoint
async fn health(service: web::Data<EncryptionService>) -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "encryption-service".to_string(),
        session_count: service.session_count(),
    })
}

/// Generate ephemeral key request (no body needed)
#[derive(Serialize)]
struct EphemeralKeyResponse {
    public_key: String,
}

async fn generate_ephemeral(
    service: web::Data<EncryptionService>,
) -> HttpResponse {
    let result = service.generate_ephemeral();
    
    HttpResponse::Ok().json(EphemeralKeyResponse {
        public_key: hex::encode(result.public_key),
    })
}

/// Init session request
#[derive(Deserialize)]
struct InitSessionRequest {
    session_id: String,
    shared_secret: String, // hex-encoded
}

/// Init session response
#[derive(Serialize)]
struct InitSessionResponse {
    session_id: String,
    chain_state: String,
}

async fn init_session(
    service: web::Data<EncryptionService>,
    req: web::Json<InitSessionRequest>,
) -> Result<HttpResponse, ServiceError> {
    let secret_bytes = hex::decode(&req.shared_secret)
        .map_err(|_| ServiceError::InvalidRequest("Invalid hex in shared_secret".to_string()))?;
    
    if secret_bytes.len() != 32 {
        return Err(ServiceError::InvalidRequest("shared_secret must be 32 bytes".to_string()));
    }

    let mut secret = [0u8; 32];
    secret.copy_from_slice(&secret_bytes);

    let result = service.init_session(&req.session_id, secret);

    Ok(HttpResponse::Ok().json(InitSessionResponse {
        session_id: result.session_id,
        chain_state: hex::encode(result.chain_state),
    }))
}

/// Encrypt message request
#[derive(Deserialize)]
struct EncryptRequest {
    session_id: String,
    plaintext: String, // base64-encoded
}

/// Encrypt message response
#[derive(Serialize)]
struct EncryptResponse {
    ciphertext: String, // base64
    nonce: String,      // hex
    message_number: u64,
    chain_proof: String,
    chain_state: String,
}

async fn encrypt_message(
    service: web::Data<EncryptionService>,
    req: web::Json<EncryptRequest>,
) -> Result<HttpResponse, ServiceError> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    
    let plaintext = STANDARD.decode(&req.plaintext)
        .map_err(|_| ServiceError::InvalidRequest("Invalid base64 in plaintext".to_string()))?;

    let result = service.encrypt_message(&req.session_id, &plaintext)?;

    Ok(HttpResponse::Ok().json(EncryptResponse {
        ciphertext: STANDARD.encode(&result.ciphertext),
        nonce: hex::encode(&result.nonce),
        message_number: result.message_number,
        chain_proof: hex::encode(result.chain_proof),
        chain_state: hex::encode(result.chain_state),
    }))
}

/// Decrypt message request
#[derive(Deserialize)]
struct DecryptRequest {
    session_id: String,
    ciphertext: String,  // base64
    nonce: String,       // hex
    message_number: u64,
}

/// Decrypt message response
#[derive(Serialize)]
struct DecryptResponse {
    plaintext: String, // base64
    message_number: u64,
}

async fn decrypt_message(
    service: web::Data<EncryptionService>,
    req: web::Json<DecryptRequest>,
) -> Result<HttpResponse, ServiceError> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let ciphertext = STANDARD.decode(&req.ciphertext)
        .map_err(|_| ServiceError::InvalidRequest("Invalid base64 in ciphertext".to_string()))?;
    
    let nonce = hex::decode(&req.nonce)
        .map_err(|_| ServiceError::InvalidRequest("Invalid hex in nonce".to_string()))?;

    let result = service.decrypt_message(
        &req.session_id,
        &ciphertext,
        &nonce,
        req.message_number,
    )?;

    Ok(HttpResponse::Ok().json(DecryptResponse {
        plaintext: STANDARD.encode(&result.plaintext),
        message_number: result.message_number,
    }))
}

/// Derive key request
#[derive(Deserialize)]
struct DeriveKeyRequest {
    inputs: Vec<String>, // hex-encoded
    info: String,        // hex-encoded
}

/// Derive key response
#[derive(Serialize)]
struct DeriveKeyResponse {
    key: String, // hex-encoded
}

async fn derive_key(
    service: web::Data<EncryptionService>,
    req: web::Json<DeriveKeyRequest>,
) -> Result<HttpResponse, ServiceError> {
    let inputs: Result<Vec<Vec<u8>>, _> = req.inputs
        .iter()
        .map(|s| hex::decode(s))
        .collect();
    
    let inputs = inputs
        .map_err(|_| ServiceError::InvalidRequest("Invalid hex in inputs".to_string()))?;

    let info = hex::decode(&req.info)
        .map_err(|_| ServiceError::InvalidRequest("Invalid hex in info".to_string()))?;

    let key = service.derive_key(inputs, &info)?;

    Ok(HttpResponse::Ok().json(DeriveKeyResponse {
        key: hex::encode(key),
    }))
}

/// Verify chain request
#[derive(Deserialize)]
struct VerifyChainRequest {
    session_id: String,
}

/// Verify chain response
#[derive(Serialize)]
struct VerifyChainResponse {
    valid: bool,
    sequence: u64,
    current_state: String,
}

async fn verify_chain(
    service: web::Data<EncryptionService>,
    req: web::Json<VerifyChainRequest>,
) -> Result<HttpResponse, ServiceError> {
    let result = service.verify_chain(&req.session_id)?;

    Ok(HttpResponse::Ok().json(VerifyChainResponse {
        valid: result.valid,
        sequence: result.sequence,
        current_state: hex::encode(result.current_state),
    }))
}

/// Get session response
#[derive(Serialize)]
struct SessionResponse {
    session_id: String,
    message_count: u64,
    chain_sequence: u64,
    current_state: String,
}

async fn get_session(
    service: web::Data<EncryptionService>,
    path: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let session_id = path.into_inner();

    match service.get_session_info(&session_id) {
        Some(info) => Ok(HttpResponse::Ok().json(SessionResponse {
            session_id: info.session_id,
            message_count: info.message_count,
            chain_sequence: info.chain_sequence,
            current_state: hex::encode(info.current_state),
        })),
        None => Err(ServiceError::SessionNotFound(session_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_health() {
        let service = web::Data::new(
            EncryptionService::new("./test-data").unwrap()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(service)
                .configure(configure_routes)
        ).await;

        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_generate_ephemeral() {
        let service = web::Data::new(
            EncryptionService::new("./test-data").unwrap()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(service)
                .configure(configure_routes)
        ).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/encrypt/generate-ephemeral")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
