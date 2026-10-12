use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{Repository, RepositoryMessage};

pub const WORKSPACE_CONFIG_FILENAME: &str = ".patto.toml";

/// Workspace-level settings, stored as `.patto.toml` in the notes directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PattoWorkspaceConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub pinned_files: Vec<String>,
}

pub fn load_workspace_config(dir: &Path) -> PattoWorkspaceConfig {
    let path = dir.join(WORKSPACE_CONFIG_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => PattoWorkspaceConfig::default(),
    }
}

pub fn save_workspace_config(dir: &Path, config: &PattoWorkspaceConfig) -> anyhow::Result<()> {
    let path = dir.join(WORKSPACE_CONFIG_FILENAME);
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

impl Repository {
    /// Pin a file (relative path). Saves config and broadcasts WorkspaceConfigChanged.
    pub fn pin_file(&self, path: &str) -> anyhow::Result<()> {
        self.edit_pinned_files(|pinned| {
            if !pinned.iter().any(|p| p == path) {
                pinned.push(path.to_string());
            }
        })
    }

    /// Unpin a file (relative path). Saves config and broadcasts WorkspaceConfigChanged.
    pub fn unpin_file(&self, path: &str) -> anyhow::Result<()> {
        self.edit_pinned_files(|pinned| pinned.retain(|p| p != path))
    }

    fn edit_pinned_files(&self, edit: impl FnOnce(&mut Vec<String>)) -> anyhow::Result<()> {
        let config = {
            let mut config = self.workspace_config.lock().unwrap();
            edit(&mut config.pinned_files);
            save_workspace_config(&self.root_dir, &config)?;
            config.clone()
        };
        let _ = self
            .tx
            .send(RepositoryMessage::WorkspaceConfigChanged(config));
        Ok(())
    }

    pub(super) fn reload_workspace_config(&self) {
        let config = load_workspace_config(&self.root_dir);
        *self.workspace_config.lock().unwrap() = config.clone();
        let _ = self
            .tx
            .send(RepositoryMessage::WorkspaceConfigChanged(config));
    }
}
