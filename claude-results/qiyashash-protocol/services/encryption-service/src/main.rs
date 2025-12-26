//! QiyasHash Encryption Service
//!
//! Provides message encryption/decryption using the QiyasHash protocol.
//! Manages ephemeral keys and chain proofs.

use actix_web::{web, App, HttpServer, middleware};
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod error;
mod service;

use service::EncryptionService;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "encryption-service")]
#[command(about = "QiyasHash Encryption Service")]
struct Args {
    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "8082")]
    port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Storage path
    #[arg(short, long, default_value = "./data/encryption")]
    storage_path: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(true)
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting QiyasHash Encryption Service");
    info!("Binding to {}:{}", args.host, args.port);

    // Create service
    let service = EncryptionService::new(&args.storage_path)
        .expect("Failed to create encryption service");
    let service = web::Data::new(service);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(service.clone())
            .wrap(middleware::Logger::default())
            .wrap(actix_cors::Cors::permissive())
            .configure(api::configure_routes)
    })
    .bind(format!("{}:{}", args.host, args.port))?
    .run()
    .await
}
