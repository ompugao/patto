use serde::Deserialize;

/// Background color used when compositing images that have an alpha channel.
#[derive(Debug, Clone)]
pub enum ImageBackground {
    White,
    Black,
    /// Arbitrary RGB color.
    Custom([u8; 3]),
    /// Pass the image through unchanged; no compositing is applied.
    None,
}

impl ImageBackground {
    /// Return the RGB triplet for this background, or `None` to skip compositing.
    pub fn to_rgb(&self) -> Option<[u8; 3]> {
        match self {
            ImageBackground::White => Some([255, 255, 255]),
            ImageBackground::Black => Some([0, 0, 0]),
            ImageBackground::Custom(rgb) => Some(*rgb),
            ImageBackground::None => Option::None,
        }
    }
}

impl Default for ImageBackground {
    fn default() -> Self {
        ImageBackground::White
    }
}

impl<'de> Deserialize<'de> for ImageBackground {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "white" => Ok(ImageBackground::White),
            "black" => Ok(ImageBackground::Black),
            "none" | "as-is" => Ok(ImageBackground::None),
            hex if hex.starts_with('#') && hex.len() == 7 => {
                let r = u8::from_str_radix(&hex[1..3], 16).map_err(serde::de::Error::custom)?;
                let g = u8::from_str_radix(&hex[3..5], 16).map_err(serde::de::Error::custom)?;
                let b = u8::from_str_radix(&hex[5..7], 16).map_err(serde::de::Error::custom)?;
                Ok(ImageBackground::Custom([r, g, b]))
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown image_background '{}': expected \"white\", \"black\", \"none\", \"as-is\", or \"#rrggbb\"",
                other
            ))),
        }
    }
}

/// Position of the floating tasks panel within the content area.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TasksPanelPosition {
    #[default]
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

fn default_tasks_width() -> f64 {
    0.4
}

fn default_tasks_height() -> f64 {
    0.3
}

/// Configuration for the floating tasks panel.
#[derive(Debug, Clone, Deserialize)]
pub struct TasksPanelConfig {
    /// Which corner of the content area to anchor the panel to.
    #[serde(default)]
    pub position: TasksPanelPosition,
    /// Panel width as a fraction of the content area width (0.0 – 1.0).
    #[serde(default = "default_tasks_width")]
    pub width: f64,
    /// Panel height as a fraction of the content area height (0.0 – 1.0).
    #[serde(default = "default_tasks_height")]
    pub height: f64,
}

impl Default for TasksPanelConfig {
    fn default() -> Self {
        Self {
            position: TasksPanelPosition::default(),
            width: default_tasks_width(),
            height: default_tasks_height(),
        }
    }
}

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
    /// Floating tasks panel appearance and position.
    #[serde(default)]
    pub tasks: TasksPanelConfig,
    /// Background color for compositing images with transparency.
    /// Accepts `"white"`, `"black"`, or a hex string like `"#rrggbb"`.
    /// Defaults to `"white"`.
    #[serde(default)]
    pub image_background: ImageBackground,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            syntax_theme: default_syntax_theme(),
            tasks: TasksPanelConfig::default(),
            image_background: ImageBackground::default(),
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
