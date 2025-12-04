#[cfg(feature = "zotero")]
use crate::lsp_config::ZoteroCredentials;
use crate::lsp_config::{resolve_cache_file, PattoLspConfig};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[cfg(feature = "zotero")]
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json;
#[cfg(feature = "zotero")]
use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
#[cfg(feature = "zotero")]
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use thiserror::Error;
#[cfg(feature = "zotero")]
use tokio::time::{self, Duration};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

const DEFAULT_LIMIT: usize = 100000;
#[cfg(feature = "zotero")]
const HEALTHCHECK_LIMIT: usize = 1;
#[cfg(feature = "zotero")]
const ZOTERO_URL_PREFIX: &str = "zotero://select/library/items/";
#[cfg(feature = "zotero")]
const CACHE_REFRESH_INTERVAL_SECS: u64 = 60;
const CACHE_FILE_NAME: &str = "zotero-papers.json";

type DynPaperClient = dyn PaperClient + Send + Sync;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperReference {
    pub title: String,
    pub key: String,
    pub link: String,
}

#[derive(Debug, Error)]
pub enum PaperClientError {
    #[error("paper client is not configured")]
    NotConfigured,
    #[cfg(not(feature = "zotero"))]
    #[error("paper integration feature \"{0}\" is disabled at compile time")]
    FeatureDisabled(&'static str),
    #[cfg(feature = "zotero")]
    #[error("zotero request failed: {0}")]
    Zotero(#[from] zotero_rs::errors::ZoteroError),
}

#[derive(Debug, Serialize, Deserialize)]
struct PaperCacheFile {
    fetched_at: DateTime<Utc>,
    entries: Vec<PaperReference>,
}

#[derive(Debug)]
struct PaperCacheState {
    entries: RwLock<Vec<PaperReference>>,
    fetched_at: RwLock<Option<DateTime<Utc>>>,
    cache_path: PathBuf,
    #[cfg(feature = "zotero")]
    refresh_started: Mutex<bool>,
}

impl PaperCacheState {
    fn new(cache_path: PathBuf) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            fetched_at: RwLock::new(None),
            cache_path,
            #[cfg(feature = "zotero")]
            refresh_started: Mutex::new(false),
        }
    }

    fn load_from_disk(&self) {
        match fs::read_to_string(&self.cache_path) {
            Ok(content) => match serde_json::from_str::<PaperCacheFile>(&content) {
                Ok(cache) => {
                    *self.entries.write().unwrap() = cache.entries;
                    *self.fetched_at.write().unwrap() = Some(cache.fetched_at);
                }
                Err(err) => {
                    log::warn!(
                        "failed to parse Zotero cache file {}: {}",
                        self.cache_path.display(),
                        err
                    );
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                log::warn!(
                    "failed to read Zotero cache file {}: {}",
                    self.cache_path.display(),
                    err
                );
            }
        }
    }

    fn last_updated(&self) -> Option<DateTime<Utc>> {
        *self.fetched_at.read().unwrap()
    }

    fn search(&self, query: &str, limit: usize) -> Vec<PaperReference> {
        let matcher = SkimMatcherV2::default();
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|paper| matcher.fuzzy_match(&paper.title, query).is_some())
            .take(limit)
            .cloned()
            .collect()
    }

    #[cfg(feature = "zotero")]
    fn update_entries(&self, entries: Vec<PaperReference>, fetched_at: DateTime<Utc>) {
        *self.entries.write().unwrap() = entries.clone();
        *self.fetched_at.write().unwrap() = Some(fetched_at);
        if let Err(err) = self.persist_owned(entries, fetched_at) {
            log::warn!(
                "failed to persist Zotero cache file {}: {}",
                self.cache_path.display(),
                err
            );
        }
    }

    #[cfg(feature = "zotero")]
    fn persist_owned(
        &self,
        entries: Vec<PaperReference>,
        fetched_at: DateTime<Utc>,
    ) -> io::Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = PaperCacheFile {
            fetched_at,
            entries,
        };
        let json = serde_json::to_string_pretty(&payload)?;
        fs::write(&self.cache_path, json)
    }

    #[cfg(feature = "zotero")]
    fn spawn_refresh_loop(self: &Arc<Self>, client: Arc<DynPaperClient>) {
        let mut started = self.refresh_started.lock().unwrap();
        if *started {
            return;
        }
        *started = true;
        let state = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                if let Err(err) = state.refresh_once(client.clone()).await {
                    log::warn!("paper cache refresh failed: {}", err);
                }
                time::sleep(Duration::from_secs(CACHE_REFRESH_INTERVAL_SECS)).await;
            }
        });
    }

    #[cfg(feature = "zotero")]
    async fn refresh_once(&self, client: Arc<DynPaperClient>) -> Result<(), PaperClientError> {
        let entries = client.fetch_all_items().await?;
        let now = Utc::now();
        self.update_entries(entries, now);
        Ok(())
    }
}

#[async_trait]
pub trait PaperClient: Send + Sync {
    async fn health_check(&self) -> Result<(), PaperClientError>;
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperClientError>;
    #[cfg(feature = "zotero")]
    async fn fetch_all_items(&self) -> Result<Vec<PaperReference>, PaperClientError>;
    fn provider_name(&self) -> &'static str;
}

#[derive(Clone)]
pub struct PaperClientManager {
    client: Option<Arc<DynPaperClient>>,
    cache: Arc<PaperCacheState>,
}

impl Default for PaperClientManager {
    fn default() -> Self {
        let cache = Self::build_cache_state();
        Self {
            client: None,
            cache,
        }
    }
}

impl PaperClientManager {
    fn build_cache_state() -> Arc<PaperCacheState> {
        let cache_path = resolve_cache_file(CACHE_FILE_NAME).unwrap_or_else(|err| {
            log::warn!(
                "failed to resolve Zotero cache directory via XDG: {}, falling back to temp dir",
                err
            );
            env::temp_dir().join(CACHE_FILE_NAME)
        });
        let state = Arc::new(PaperCacheState::new(cache_path));
        state.load_from_disk();
        state
    }

    pub fn from_config(config: Option<&PattoLspConfig>) -> Result<Self, PaperClientError> {
        let cache = Self::build_cache_state();
        let Some(cfg) = config else {
            return Ok(Self {
                client: None,
                cache,
            });
        };
        let Some(credentials) = cfg.zotero_credentials() else {
            return Ok(Self {
                client: None,
                cache,
            });
        };

        #[cfg(feature = "zotero")]
        {
            let client: Arc<DynPaperClient> = Arc::new(ZoteroPaperClient::new(credentials)?);
            cache.spawn_refresh_loop(client.clone());
            return Ok(Self {
                client: Some(client),
                cache,
            });
        }

        #[cfg(not(feature = "zotero"))]
        {
            let _ = credentials;
            return Err(PaperClientError::FeatureDisabled("zotero"));
        }
    }

    pub fn is_configured(&self) -> bool {
        self.client.is_some()
    }

    pub fn provider_name(&self) -> Option<&'static str> {
        self.client.as_ref().map(|client| client.provider_name())
    }

    pub async fn health_check(&self) -> Result<(), PaperClientError> {
        if let Some(client) = &self.client {
            client.health_check().await
        } else {
            Err(PaperClientError::NotConfigured)
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<PaperReference>, PaperClientError> {
        let trimmed = query.trim();
        let cached = self.filter_cached(trimmed);
        if !cached.is_empty() || self.cache.last_updated().is_some() {
            return Ok(cached);
        }

        if let Some(client) = &self.client {
            client.search(trimmed, DEFAULT_LIMIT).await
        } else {
            Err(PaperClientError::NotConfigured)
        }
    }

    fn filter_cached(&self, query: &str) -> Vec<PaperReference> {
        self.cache.search(query, DEFAULT_LIMIT)
    }
}

#[cfg(feature = "zotero")]
struct ZoteroPaperClient {
    inner: zotero_rs::ZoteroAsync,
}

#[cfg(feature = "zotero")]
impl ZoteroPaperClient {
    fn new(credentials: ZoteroCredentials) -> Result<Self, PaperClientError> {
        let mut client =
            zotero_rs::ZoteroAsync::user_lib(&credentials.user_id, &credentials.api_key)?;
        if let Some(endpoint) = credentials.endpoint.as_deref() {
            client.set_endpoint(endpoint);
        }
        Ok(Self { inner: client })
    }

    async fn fetch_items(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperClientError> {
        let mut owned_params = vec![
            ("limit".to_string(), limit.to_string()),
            ("sort".to_string(), "dateAdded".to_string()),
            ("direction".to_string(), "desc".to_string()),
        ];

        if let Some(q) = query.filter(|q| !q.trim().is_empty()) {
            owned_params.push(("q".to_string(), q.to_string()));
        }

        let borrowed_params: Vec<(&str, &str)> = owned_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let raw_items = if borrowed_params.is_empty() {
            self.inner.get_items(None).await?
        } else {
            self.inner.get_items(Some(&borrowed_params)).await?
        };

        Ok(extract_references(&raw_items))
    }
}

#[cfg(feature = "zotero")]
#[async_trait]
impl PaperClient for ZoteroPaperClient {
    async fn health_check(&self) -> Result<(), PaperClientError> {
        let _ = self.fetch_items(None, HEALTHCHECK_LIMIT).await?;
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperClientError> {
        let items = self.fetch_items(Some(query), limit).await?;
        debug!("fetched {} papers from Zotero", items.len());
        Ok(items)
    }

    async fn fetch_all_items(&self) -> Result<Vec<PaperReference>, PaperClientError> {
        let raw_items = self.inner.get_items(None).await?;
        let entries = extract_references(&raw_items);
        debug!(
            "cached {} Zotero papers via background refresh",
            entries.len()
        );
        Ok(entries)
    }

    fn provider_name(&self) -> &'static str {
        "zotero"
    }
}

#[cfg(feature = "zotero")]
fn extract_references(value: &Value) -> Vec<PaperReference> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let data = item.get("data")?;
                    let title = data.get("title")?.as_str()?.trim();
                    if title.is_empty() {
                        return None;
                    }
                    let key = data.get("key")?.as_str()?.trim();
                    if key.is_empty() {
                        return None;
                    }
                    Some(PaperReference {
                        title: title.to_string(),
                        key: key.to_string(),
                        link: format!("{}{}", ZOTERO_URL_PREFIX, key),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
