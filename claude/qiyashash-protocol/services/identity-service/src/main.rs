//! Identity Service for QiyasHash
//!
//! Provides identity key management, rotation, and verification.

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod error;
mod service;
mod storage;

use service::IdentityServiceImpl;
use storage::RocksDbStorage;

/// Identity Service CLI arguments
#[derive(Parser, Debug)]
#[command(name = "identity-service")]
#[command(about = "QiyasHash Identity Management Service")]
struct Args {
    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Storage path
    #[arg(short, long, default_value = "./data/identity")]
    storage_path: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

/// Application state
pub struct AppState {
    pub service: Arc<IdentityServiceImpl>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting Identity Service on {}:{}", args.host, args.port);

    // Initialize storage
    let storage = RocksDbStorage::open(&args.storage_path)
        .expect("Failed to open storage");

    // Initialize service
    let service = Arc::new(IdentityServiceImpl::new(storage));

    let app_state = web::Data::new(AppState { service });

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .configure(api::configure)
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}
