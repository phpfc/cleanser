//! CLI configuration management.

use anyhow::Result;
use cleanser_core::Platform;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Whether to check for updates on startup
    #[serde(default = "default_check_updates")]
    pub check_updates: bool,
}

fn default_check_updates() -> bool {
    true
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            check_updates: true,
        }
    }
}

impl CliConfig {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let home = cleanser_core::home_dir_or_err()?;
        let config_dir = match Platform::current() {
            Platform::MacOS => home
                .join("Library")
                .join("Application Support")
                .join("cleanser"),
            Platform::Linux => home.join(".config").join("cleanser"),
            Platform::Windows => home.join("AppData").join("Local").join("cleanser"),
        };
        Ok(config_dir.join("config.json"))
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Check if updates should be checked (respects env var)
    pub fn should_check_updates(&self) -> bool {
        // Respect environment variable
        if std::env::var("CLEANSER_NO_UPDATE_CHECK").is_ok() {
            return false;
        }

        self.check_updates
    }
}
