use serde::Deserialize;
use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

const CONFIG_NAMESPACE: &str = "patto";
const CONFIG_FILENAME: &str = "patto-lsp.toml";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PattoLspConfig {
    #[serde(default)]
    pub zotero: Option<ZoteroSection>,
    #[serde(alias = "ZOTERO_USER_ID")]
    pub zotero_user_id: Option<String>,
    #[serde(alias = "ZOTERO_API_KEY")]
    pub zotero_api_key: Option<String>,
    #[serde(alias = "ZOTERO_ENDPOINT")]
    pub zotero_endpoint: Option<String>,
}

impl PattoLspConfig {
    pub fn zotero_credentials(&self) -> Option<ZoteroCredentials> {
        if let Some(section) = &self.zotero {
            let user_id = normalize_field(section.user_id.as_deref())
                .or_else(|| normalize_field(self.zotero_user_id.as_deref()));
            let api_key = normalize_field(section.api_key.as_deref())
                .or_else(|| normalize_field(self.zotero_api_key.as_deref()));
            if let (Some(user_id), Some(api_key)) = (user_id, api_key) {
                let endpoint = normalize_field(section.endpoint.as_deref())
                    .or_else(|| normalize_field(self.zotero_endpoint.as_deref()));
                return Some(ZoteroCredentials {
                    user_id,
                    api_key,
                    endpoint,
                });
            }
        }

        let user_id = normalize_field(self.zotero_user_id.as_deref())?;
        let api_key = normalize_field(self.zotero_api_key.as_deref())?;
        let endpoint = normalize_field(self.zotero_endpoint.as_deref());
        Some(ZoteroCredentials {
            user_id,
            api_key,
            endpoint,
        })
    }
}

fn normalize_field(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ZoteroSection {
    #[serde(alias = "user_id", alias = "userId", alias = "ZOTERO_USER_ID")]
    pub user_id: Option<String>,
    #[serde(alias = "api_key", alias = "apiKey", alias = "ZOTERO_API_KEY")]
    pub api_key: Option<String>,
    #[serde(alias = "endpoint", alias = "base_url", alias = "ZOTERO_ENDPOINT")]
    pub endpoint: Option<String>,
}

#[cfg_attr(not(feature = "zotero"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ZoteroCredentials {
    pub user_id: String,
    pub api_key: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigLoadResult {
    pub config: PattoLspConfig,
    pub path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unable to determine configuration directory via XDG environment variables")]
    MissingConfigDir,
    #[error("failed to read config file at {path:?}: {source}")]
    Io {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("failed to parse config file at {path:?}: {source}")]
    Parse {
        #[source]
        source: toml::de::Error,
        path: PathBuf,
    },
}

pub fn load_config() -> Result<Option<ConfigLoadResult>, ConfigError> {
    let path = match resolve_config_path() {
        Ok(path) => path,
        Err(ConfigError::MissingConfigDir) => return Ok(None),
        Err(err) => return Err(err),
    };

    if !path.exists() {
        return Ok(None);
    }

    let config_text = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
        source,
        path: path.clone(),
    })?;

    let config: PattoLspConfig =
        toml::from_str(&config_text).map_err(|source| ConfigError::Parse {
            source,
            path: path.clone(),
        })?;

    Ok(Some(ConfigLoadResult { config, path }))
}

pub fn resolve_config_path() -> Result<PathBuf, ConfigError> {
    Ok(config_home_dir()?
        .join(CONFIG_NAMESPACE)
        .join(CONFIG_FILENAME))
}

pub fn resolve_cache_file(filename: &str) -> io::Result<PathBuf> {
    Ok(cache_home_dir()?.join(CONFIG_NAMESPACE).join(filename))
}

fn config_home_dir() -> Result<PathBuf, ConfigError> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(dir));
    }

    #[cfg(windows)]
    if let Some(dir) = env::var_os("APPDATA") {
        return Ok(PathBuf::from(dir));
    }

    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config"));
    }

    #[cfg(windows)]
    if let Some(profile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(profile).join("AppData").join("Roaming"));
    }

    Err(ConfigError::MissingConfigDir)
}

fn cache_home_dir() -> io::Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(dir));
    }

    #[cfg(windows)]
    if let Some(dir) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(dir));
    }

    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".cache"));
    }

    #[cfg(windows)]
    if let Some(profile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(profile).join("AppData").join("Local"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "unable to determine cache directory",
    ))
}
