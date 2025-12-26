//! CLI configuration

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// CLI configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    /// Storage path
    pub storage_path: PathBuf,
    /// Default server URL
    pub server_url: Option<String>,
    /// Auto-connect on startup
    pub auto_connect: bool,
    /// Show notifications
    pub notifications: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        let storage_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("qiyashash");

        Self {
            storage_path,
            server_url: None,
            auto_connect: false,
            notifications: true,
        }
    }
}

impl CliConfig {
    /// Load config from file or create default
    pub fn load_or_default(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: CliConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();

            // Create parent directories
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Save default config
            let content = toml::to_string_pretty(&config)?;
            std::fs::write(path, content)?;

            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
