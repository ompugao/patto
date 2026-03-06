use serde::Deserialize;

/// What the TUI does after launching the editor command.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorAction {
    /// Disable raw mode, wait for the command to exit, then resume the TUI.
    #[default]
    Suspend,
    /// Spawn the command and immediately exit the TUI.
    Quit,
    /// Spawn the command in the background; keep the TUI running.
    Background,
}

/// Editor launch configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EditorConfig {
    /// Shell command template. Supports `{file}` and `{line}` placeholders.
    /// Falls back to `$EDITOR +{line} {file}` when `None`.
    pub cmd: Option<String>,
    /// What to do with the TUI after launching the command.
    #[serde(default)]
    pub action: EditorAction,
}

fn default_syntax_theme() -> String {
    "base16-ocean.dark".to_string()
}

/// Top-level TUI configuration (`~/.config/patto/patto-preview-tui.toml`).
#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    #[serde(default)]
    pub editor: EditorConfig,
    /// syntect theme name for code block syntax highlighting.
    /// Defaults to `"base16-ocean.dark"` when not set.
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            syntax_theme: default_syntax_theme(),
        }
    }
}

impl TuiConfig {
    /// Load configuration from `~/.config/patto/patto-preview-tui.toml`.
    /// Returns defaults silently if the file is missing or cannot be parsed.
    pub fn load() -> Self {
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from(".config"));
        let path = config_dir.join("patto").join("patto-preview-tui.toml");

        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Self::default();
        };

        toml::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Warning: failed to parse {}: {}", path.display(), e);
            Self::default()
        })
    }
}
