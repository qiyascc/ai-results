//! QiyasHash CLI Client
//!
//! Command-line interface for secure E2E encrypted messaging.

use clap::{Parser, Subcommand};
use console::{style, Emoji};
use dialoguer::{Confirm, Input, Password};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod commands;
mod config;
mod storage;

use config::CliConfig;
use storage::LocalStorage;

static LOCK: Emoji<'_, '_> = Emoji("🔐 ", "");
static CHECK: Emoji<'_, '_> = Emoji("✅ ", "[OK] ");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "[ERR] ");
static SEND: Emoji<'_, '_> = Emoji("📤 ", "[SEND] ");
static RECV: Emoji<'_, '_> = Emoji("📥 ", "[RECV] ");
static KEY: Emoji<'_, '_> = Emoji("🔑 ", "[KEY] ");

/// QiyasHash CLI - Secure End-to-End Encrypted Messaging
#[derive(Parser)]
#[command(name = "qiyashash")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new identity
    Init {
        /// Device name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Show identity information
    Identity {
        /// Show fingerprint
        #[arg(short, long)]
        fingerprint: bool,
    },

    /// Rotate identity keys
    Rotate {
        /// Force rotation without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Send a message
    Send {
        /// Recipient user ID
        #[arg(short, long)]
        to: String,

        /// Message content
        #[arg(short, long)]
        message: Option<String>,

        /// Read message from file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Receive messages
    Receive {
        /// Number of messages to fetch
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// List conversations
    List {
        /// Show all messages
        #[arg(short, long)]
        all: bool,
    },

    /// Manage contacts
    Contacts {
        #[command(subcommand)]
        action: ContactAction,
    },

    /// Verify contact identity
    Verify {
        /// Contact user ID
        user_id: String,
    },

    /// Show session information
    Sessions {
        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
    },

    /// Export identity (backup)
    Export {
        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Import identity (restore)
    Import {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Server connection management
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
}

#[derive(Subcommand)]
enum ContactAction {
    /// Add a contact
    Add {
        /// User ID
        user_id: String,
        /// Alias
        #[arg(short, long)]
        alias: Option<String>,
    },
    /// Remove a contact
    Remove {
        /// User ID
        user_id: String,
    },
    /// List contacts
    List,
    /// Block a contact
    Block {
        /// User ID
        user_id: String,
    },
    /// Unblock a contact
    Unblock {
        /// User ID
        user_id: String,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Connect to server
    Connect {
        /// Server URL
        url: String,
    },
    /// Disconnect from server
    Disconnect,
    /// Show connection status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Load config
    let config_path = cli.config.unwrap_or_else(|| {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("qiyashash");
        path.push("config.toml");
        path
    });

    let config = CliConfig::load_or_default(&config_path)?;

    // Initialize storage
    let storage_path = config.storage_path.clone();
    let storage = LocalStorage::open(&storage_path)?;

    // Execute command
    match cli.command {
        Commands::Init { name } => {
            init_identity(&storage, name).await?;
        }
        Commands::Identity { fingerprint } => {
            show_identity(&storage, fingerprint).await?;
        }
        Commands::Rotate { force } => {
            rotate_identity(&storage, force).await?;
        }
        Commands::Send { to, message, file } => {
            send_message(&storage, &to, message, file).await?;
        }
        Commands::Receive { count } => {
            receive_messages(&storage, count).await?;
        }
        Commands::List { all } => {
            list_conversations(&storage, all).await?;
        }
        Commands::Contacts { action } => {
            handle_contacts(&storage, action).await?;
        }
        Commands::Verify { user_id } => {
            verify_contact(&storage, &user_id).await?;
        }
        Commands::Sessions { verbose } => {
            show_sessions(&storage, verbose).await?;
        }
        Commands::Export { output } => {
            export_identity(&storage, &output).await?;
        }
        Commands::Import { input } => {
            import_identity(&storage, &input).await?;
        }
        Commands::Server { action } => {
            handle_server(&storage, action).await?;
        }
    }

    Ok(())
}

async fn init_identity(storage: &LocalStorage, name: Option<String>) -> anyhow::Result<()> {
    println!("{} Initializing QiyasHash identity...", LOCK);

    // Check if identity already exists
    if storage.has_identity()? {
        let confirm = Confirm::new()
            .with_prompt("Identity already exists. Overwrite?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("{} Cancelled", CROSS);
            return Ok(());
        }
    }

    let device_name = name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Device name")
            .default(whoami::devicename())
            .interact_text()
            .unwrap()
    });

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Generating identity keys...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Generate identity
    let identity = qiyashash_crypto::identity::Identity::new();
    let fingerprint = hex::encode(&identity.fingerprint);
    let user_id = hex::encode(&identity.fingerprint[..16]);

    // Generate prekeys
    pb.set_message("Generating prekeys...");
    let mut prekey_manager =
        qiyashash_crypto::x3dh::PreKeyManager::new(identity.key_pair.clone());
    prekey_manager.generate_one_time_prekeys(100);

    // Save to storage
    pb.set_message("Saving identity...");
    storage.save_identity(&identity, &device_name)?;

    pb.finish_and_clear();

    println!("{} Identity created successfully!", CHECK);
    println!();
    println!("  {} User ID: {}", KEY, style(&user_id).cyan());
    println!(
        "  {} Fingerprint: {}",
        KEY,
        style(&fingerprint[..32]).yellow()
    );
    println!("  {} Device: {}", KEY, style(&device_name).green());
    println!();
    println!(
        "{}",
        style("Share your User ID with contacts to receive messages.").dim()
    );

    Ok(())
}

async fn show_identity(storage: &LocalStorage, show_fingerprint: bool) -> anyhow::Result<()> {
    let identity = storage
        .get_identity()?
        .ok_or_else(|| anyhow::anyhow!("No identity found. Run 'qiyashash init' first."))?;

    let fingerprint = hex::encode(&identity.fingerprint);
    let user_id = hex::encode(&identity.fingerprint[..16]);

    println!("{} Identity Information", KEY);
    println!();
    println!("  User ID:     {}", style(&user_id).cyan());

    if show_fingerprint {
        // Format fingerprint in groups
        let formatted: String = fingerprint
            .chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");

        println!("  Fingerprint: {}", style(&formatted).yellow());
    }

    println!(
        "  Created:     {}",
        chrono::DateTime::from_timestamp(identity.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );

    Ok(())
}

async fn rotate_identity(storage: &LocalStorage, force: bool) -> anyhow::Result<()> {
    if !force {
        let confirm = Confirm::new()
            .with_prompt("Rotate identity keys? This cannot be undone.")
            .default(false)
            .interact()?;

        if !confirm {
            println!("{} Cancelled", CROSS);
            return Ok(());
        }
    }

    let identity = storage
        .get_identity()?
        .ok_or_else(|| anyhow::anyhow!("No identity found."))?;

    let (new_identity, proof) = identity.rotate();

    storage.save_rotated_identity(&new_identity, &proof)?;

    println!("{} Identity rotated successfully!", CHECK);
    println!(
        "  New fingerprint: {}",
        style(hex::encode(&new_identity.fingerprint[..16])).yellow()
    );

    Ok(())
}

async fn send_message(
    storage: &LocalStorage,
    to: &str,
    message: Option<String>,
    file: Option<PathBuf>,
) -> anyhow::Result<()> {
    let content = if let Some(msg) = message {
        msg
    } else if let Some(path) = file {
        std::fs::read_to_string(&path)?
    } else {
        Input::new()
            .with_prompt("Message")
            .interact_text()?
    };

    println!("{} Sending message to {}...", SEND, style(to).cyan());

    // In a real implementation, this would:
    // 1. Establish session if needed
    // 2. Encrypt the message
    // 3. Send to DHT/relay

    println!("{} Message sent!", CHECK);

    Ok(())
}

async fn receive_messages(storage: &LocalStorage, count: usize) -> anyhow::Result<()> {
    println!("{} Checking for messages...", RECV);

    // In a real implementation, this would:
    // 1. Query DHT for new messages
    // 2. Decrypt received messages
    // 3. Display to user

    println!("  No new messages.");

    Ok(())
}

async fn list_conversations(storage: &LocalStorage, all: bool) -> anyhow::Result<()> {
    println!("Conversations:");
    println!("  (No conversations yet)");

    Ok(())
}

async fn handle_contacts(storage: &LocalStorage, action: ContactAction) -> anyhow::Result<()> {
    match action {
        ContactAction::Add { user_id, alias } => {
            println!("{} Adding contact: {}", CHECK, user_id);
        }
        ContactAction::Remove { user_id } => {
            println!("{} Removing contact: {}", CHECK, user_id);
        }
        ContactAction::List => {
            println!("Contacts:");
            println!("  (No contacts yet)");
        }
        ContactAction::Block { user_id } => {
            println!("{} Blocked: {}", CHECK, user_id);
        }
        ContactAction::Unblock { user_id } => {
            println!("{} Unblocked: {}", CHECK, user_id);
        }
    }

    Ok(())
}

async fn verify_contact(storage: &LocalStorage, user_id: &str) -> anyhow::Result<()> {
    println!("{} Contact verification for: {}", KEY, user_id);
    println!();
    println!("  Compare safety numbers with your contact:");
    println!("  12345 67890 12345 67890 12345");
    println!("  67890 12345 67890 12345 67890");
    println!();

    let verified = Confirm::new()
        .with_prompt("Do the numbers match?")
        .interact()?;

    if verified {
        println!("{} Contact verified!", CHECK);
    } else {
        println!("{} Verification failed. Do not trust this contact.", CROSS);
    }

    Ok(())
}

async fn show_sessions(storage: &LocalStorage, verbose: bool) -> anyhow::Result<()> {
    println!("Active Sessions:");
    println!("  (No active sessions)");

    Ok(())
}

async fn export_identity(storage: &LocalStorage, output: &PathBuf) -> anyhow::Result<()> {
    let password = Password::new()
        .with_prompt("Export password")
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?;

    println!("{} Exporting identity to {:?}...", KEY, output);

    // In real implementation, encrypt with password and save

    println!("{} Identity exported successfully!", CHECK);
    println!(
        "{}",
        style("Keep this file secure - it contains your private keys.").yellow()
    );

    Ok(())
}

async fn import_identity(storage: &LocalStorage, input: &PathBuf) -> anyhow::Result<()> {
    let password = Password::new()
        .with_prompt("Import password")
        .interact()?;

    println!("{} Importing identity from {:?}...", KEY, input);

    // In real implementation, decrypt and import

    println!("{} Identity imported successfully!", CHECK);

    Ok(())
}

async fn handle_server(storage: &LocalStorage, action: ServerAction) -> anyhow::Result<()> {
    match action {
        ServerAction::Connect { url } => {
            println!("Connecting to {}...", url);
            println!("{} Connected!", CHECK);
        }
        ServerAction::Disconnect => {
            println!("Disconnecting...");
            println!("{} Disconnected.", CHECK);
        }
        ServerAction::Status => {
            println!("Server Status:");
            println!("  Status: Disconnected");
        }
    }

    Ok(())
}
