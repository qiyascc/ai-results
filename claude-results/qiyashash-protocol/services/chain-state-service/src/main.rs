//! Chain State Service
//! 
//! Manages conversation chain states for message ordering and integrity
//! in the QiyasHash protocol.

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod error;
mod service;

use service::ChainStateManager;

/// Chain State Service CLI arguments
#[derive(Parser, Debug)]
#[command(name = "chain-state-service")]
#[command(about = "QiyasHash Chain State Management Service")]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value = "8083")]
    port: u16,

    /// Storage path for chain state data
    #[arg(long, default_value = "./data/chain-state")]
    storage_path: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Application state shared across handlers
pub struct AppState {
    pub chain_manager: Arc<ChainStateManager>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting Chain State Service");
    info!("Storage path: {}", args.storage_path);

    // Initialize chain manager
    let chain_manager = Arc::new(
        ChainStateManager::new(&args.storage_path)
            .expect("Failed to initialize chain manager")
    );

    let app_state = web::Data::new(AppState { chain_manager });

    info!("Binding to {}:{}", args.host, args.port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .configure(api::configure_routes)
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}
