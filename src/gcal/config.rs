//! Configuration for Google Calendar sync.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Google Calendar sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcalConfig {
    /// Calendar ID to sync to (default: "primary")
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,

    /// Prefix for event titles to identify patto-synced events
    #[serde(default = "default_event_prefix")]
    pub event_prefix: String,

    /// Whether to sync tasks marked as done
    #[serde(default)]
    pub sync_done_tasks: bool,

    /// Whether to include file path in event description
    #[serde(default = "default_true")]
    pub include_file_path: bool,

    /// Time zone for all-day events (e.g., "America/New_York")
    #[serde(default)]
    pub timezone: Option<String>,

    /// Default event duration for tasks without specific time (in hours)
    #[serde(default = "default_duration")]
    pub default_duration_hours: u32,

    /// OAuth client ID (from Google Cloud Console)
    pub client_id: Option<String>,

    /// OAuth client secret (from Google Cloud Console)
    pub client_secret: Option<String>,
}

fn default_calendar_id() -> String {
    "primary".to_string()
}

fn default_event_prefix() -> String {
    "[Patto]".to_string()
}

fn default_true() -> bool {
    true
}

fn default_duration() -> u32 {
    1
}

impl Default for GcalConfig {
    fn default() -> Self {
        Self {
            calendar_id: default_calendar_id(),
            event_prefix: default_event_prefix(),
            sync_done_tasks: false,
            include_file_path: true,
            timezone: None,
            default_duration_hours: 1,
            client_id: None,
            client_secret: None,
        }
    }
}

impl GcalConfig {
    /// Load configuration from the patto config file
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_file_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let full_config: toml::Value = toml::from_str(&content)?;
            if let Some(gcal_section) = full_config.get("google_calendar") {
                let config: GcalConfig = gcal_section.clone().try_into()?;
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    /// Get the path to the patto config directory
    pub fn config_dir() -> anyhow::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("patto");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir)
    }

    /// Get the path to the patto config file
    pub fn config_file_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("patto-lsp.toml"))
    }

    /// Get the path to the credentials file
    pub fn credentials_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("gcal-credentials.json"))
    }

    /// Get the path to the sync state file
    pub fn state_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("gcal-sync-state.json"))
    }
}
