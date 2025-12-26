//! Metadata Nullification Service
//! 
//! Protects user privacy by stripping, obfuscating, and nullifying metadata
//! from messages before they are distributed through the QiyasHash network.

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer, HttpResponse};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod nullifier;
mod error;

use nullifier::MetadataNullifier;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "metadata-nullification-service")]
#[command(about = "QiyasHash Metadata Nullification Service")]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value = "8084")]
    port: u16,

    /// Enable aggressive nullification mode
    #[arg(long)]
    aggressive: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Application state
pub struct AppState {
    pub nullifier: Arc<MetadataNullifier>,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "metadata-nullification-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Nullification request
#[derive(Deserialize)]
struct NullifyRequest {
    /// Message data (base64 encoded)
    data: String,
    /// Strip timing information
    #[serde(default = "default_true")]
    strip_timing: bool,
    /// Pad message to standard size
    #[serde(default = "default_true")]
    pad_message: bool,
    /// Add random delay before responding
    #[serde(default)]
    add_delay: bool,
}

fn default_true() -> bool {
    true
}

/// Nullification response
#[derive(Serialize)]
struct NullifyResponse {
    /// Nullified data (base64 encoded)
    data: String,
    /// Original size
    original_size: usize,
    /// Nullified size
    nullified_size: usize,
    /// Operations performed
    operations: Vec<String>,
}

async fn nullify_message(
    state: web::Data<AppState>,
    body: web::Json<NullifyRequest>,
) -> HttpResponse {
    let data = match base64::decode(&body.data) {
        Ok(d) => d,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid base64 data"
        })),
    };

    let original_size = data.len();
    let mut operations = Vec::new();

    let mut nullified = data;

    // Apply nullification operations
    if body.strip_timing {
        nullified = state.nullifier.strip_timing_metadata(&nullified);
        operations.push("strip_timing".to_string());
    }

    if body.pad_message {
        nullified = state.nullifier.pad_to_block(&nullified);
        operations.push("pad_message".to_string());
    }

    if body.add_delay {
        state.nullifier.random_delay().await;
        operations.push("random_delay".to_string());
    }

    HttpResponse::Ok().json(NullifyResponse {
        data: base64::encode(&nullified),
        original_size,
        nullified_size: nullified.len(),
        operations,
    })
}

/// Batch nullification request
#[derive(Deserialize)]
struct BatchNullifyRequest {
    messages: Vec<String>,
    #[serde(default = "default_true")]
    shuffle: bool,
}

/// Batch response
#[derive(Serialize)]
struct BatchNullifyResponse {
    messages: Vec<String>,
    count: usize,
    shuffled: bool,
}

async fn nullify_batch(
    state: web::Data<AppState>,
    body: web::Json<BatchNullifyRequest>,
) -> HttpResponse {
    let mut messages: Vec<Vec<u8>> = body.messages.iter()
        .filter_map(|m| base64::decode(m).ok())
        .collect();

    // Pad all messages
    messages = messages.iter()
        .map(|m| state.nullifier.pad_to_block(m))
        .collect();

    // Shuffle if requested
    if body.shuffle {
        state.nullifier.shuffle_messages(&mut messages);
    }

    let result: Vec<String> = messages.iter()
        .map(|m| base64::encode(m))
        .collect();

    HttpResponse::Ok().json(BatchNullifyResponse {
        count: result.len(),
        messages: result,
        shuffled: body.shuffle,
    })
}

/// Stats endpoint
#[derive(Serialize)]
struct StatsResponse {
    messages_processed: u64,
    bytes_nullified: u64,
    avg_padding_ratio: f64,
}

async fn get_stats(state: web::Data<AppState>) -> HttpResponse {
    let stats = state.nullifier.get_stats();
    HttpResponse::Ok().json(StatsResponse {
        messages_processed: stats.messages_processed,
        bytes_nullified: stats.bytes_nullified,
        avg_padding_ratio: stats.avg_padding_ratio,
    })
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

    info!("Starting Metadata Nullification Service");

    let nullifier = Arc::new(MetadataNullifier::new(args.aggressive));
    let app_state = web::Data::new(AppState { nullifier });

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
            .route("/api/v1/nullify", web::post().to(nullify_message))
            .route("/api/v1/nullify/batch", web::post().to(nullify_batch))
            .route("/api/v1/stats", web::get().to(get_stats))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}
