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
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;
use tokio::time::{self, Duration};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
const DEFAULT_LIMIT: usize = 100000;
const CACHE_REFRESH_INTERVAL_SECS: u64 = 600;
const CACHE_FILE_NAME: &str = "paper-catalog.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperReference {
    pub title: String,
    pub key: String,
    pub link: String,
}

#[derive(Debug, Error)]
pub enum PaperProviderError {
    #[error("paper provider is not configured")]
    NotConfigured,
    #[cfg(not(feature = "zotero"))]
    #[error("paper integration feature \"{0}\" is disabled at compile time")]
    FeatureDisabled(&'static str),
    #[cfg(feature = "zotero")]
    #[error("zotero request failed: {0}")]
    Zotero(#[from] zotero_rs::errors::ZoteroError),
}

#[async_trait]
pub trait PaperProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health_check(&self) -> Result<(), PaperProviderError>;
    async fn full_snapshot(&self) -> Result<Vec<PaperReference>, PaperProviderError>;
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperProviderError>;
}

type DynPaperProvider = dyn PaperProvider + Send + Sync;

#[derive(Debug)]
struct PaperCache {
    entries: RwLock<Vec<PaperReference>>,
    fetched_at: RwLock<Option<DateTime<Utc>>>,
    cache_path: PathBuf,
    refresh_started: Mutex<bool>,
}

impl PaperCache {
    fn new(cache_path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(Vec::new()),
            fetched_at: RwLock::new(None),
            cache_path,
            refresh_started: Mutex::new(false),
        })
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
                        "failed to parse paper cache file {}: {}",
                        self.cache_path.display(),
                        err
                    );
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                log::warn!(
                    "failed to read paper cache file {}: {}",
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

    fn update_entries(&self, entries: Vec<PaperReference>, fetched_at: DateTime<Utc>) {
        *self.entries.write().unwrap() = entries.clone();
        *self.fetched_at.write().unwrap() = Some(fetched_at);
        if let Err(err) = self.persist(entries, fetched_at) {
            log::warn!(
                "failed to persist paper cache file {}: {}",
                self.cache_path.display(),
                err
            );
        }
    }

    fn persist(&self, entries: Vec<PaperReference>, fetched_at: DateTime<Utc>) -> io::Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = PaperCacheFile { fetched_at, entries };
        let json = serde_json::to_string_pretty(&payload)?;
        fs::write(&self.cache_path, json)
    }

    fn spawn_refresh_loop(self: &Arc<Self>, provider: Arc<DynPaperProvider>) {
        let mut started = self.refresh_started.lock().unwrap();
        if *started {
            return;
        }
        *started = true;
        let cache = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                if let Err(err) = cache.refresh_once(provider.clone()).await {
                    log::warn!("paper cache refresh failed: {}", err);
                }
                time::sleep(Duration::from_secs(CACHE_REFRESH_INTERVAL_SECS)).await;
            }
        });
    }

    async fn refresh_once(
        &self,
        provider: Arc<DynPaperProvider>,
    ) -> Result<(), PaperProviderError> {
        let snapshot = provider.full_snapshot().await?;
        self.update_entries(snapshot, Utc::now());
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PaperCacheFile {
    fetched_at: DateTime<Utc>,
    entries: Vec<PaperReference>,
}

#[derive(Clone)]
pub struct PaperCatalog {
    provider: Option<Arc<DynPaperProvider>>,
    cache: Arc<PaperCache>,
}

impl Default for PaperCatalog {
    fn default() -> Self {
        PaperCatalog::from_config(None)
            .expect("paper catalog default initialization should not fail")
    }
}

impl PaperCatalog {
    pub fn from_config(config: Option<&PattoLspConfig>) -> Result<Self, PaperProviderError> {
        let cache_path = resolve_cache_file(CACHE_FILE_NAME).unwrap_or_else(|err| {
            log::warn!(
                "failed to resolve paper cache directory via XDG: {}, using temp dir",
                err
            );
            env::temp_dir().join(CACHE_FILE_NAME)
        });
        let cache = PaperCache::new(cache_path);
        cache.load_from_disk();

        #[allow(unused_mut)]
        let mut provider: Option<Arc<DynPaperProvider>> = None;

        if let Some(cfg) = config {
            #[cfg(feature = "zotero")]
            {
                if let Some(credentials) = cfg.zotero_credentials() {
                    provider = Some(Arc::new(ZoteroPaperProvider::new(credentials)?));
                }
            }

            #[cfg(not(feature = "zotero"))]
            {
                if cfg.zotero_credentials().is_some() {
                    return Err(PaperProviderError::FeatureDisabled("zotero"));
                }
            }
        }

        if let Some(provider) = &provider {
            cache.spawn_refresh_loop(provider.clone());
        }

        Ok(Self { provider, cache })
    }

    pub fn is_configured(&self) -> bool {
        self.provider.is_some()
    }

    pub fn provider_name(&self) -> Option<&'static str> {
        self.provider.as_ref().map(|provider| provider.name())
    }

    pub async fn health_check(&self) -> Result<(), PaperProviderError> {
        if let Some(provider) = &self.provider {
            provider.health_check().await
        } else {
            Err(PaperProviderError::NotConfigured)
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<PaperReference>, PaperProviderError> {
        let trimmed = query.trim();

        let cached = self.cache.search(trimmed, DEFAULT_LIMIT);
        if !cached.is_empty() || self.cache.last_updated().is_some() {
            return Ok(cached);
        }

        if let Some(provider) = &self.provider {
            provider.search(trimmed, DEFAULT_LIMIT).await
        } else {
            Err(PaperProviderError::NotConfigured)
        }
    }
}

#[cfg(feature = "zotero")]
struct ZoteroPaperProvider {
    inner: zotero_rs::ZoteroAsync,
}

#[cfg(feature = "zotero")]
impl ZoteroPaperProvider {
    fn new(credentials: crate::lsp_config::ZoteroCredentials) -> Result<Self, PaperProviderError> {
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
    ) -> Result<Vec<PaperReference>, PaperProviderError> {
        let mut owned_params = vec![
            ("sort".to_string(), "dateAdded".to_string()),
            ("direction".to_string(), "desc".to_string()),
        ];

        if let Some(q) = query.filter(|q| !q.trim().is_empty()) {
            owned_params.push(("q".to_string(), q.to_string()));
            owned_params.push(("limit".to_string(), limit.to_string()));
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
impl PaperProvider for ZoteroPaperProvider {
    fn name(&self) -> &'static str {
        "zotero"
    }

    async fn health_check(&self) -> Result<(), PaperProviderError> {
        let _ = self.fetch_items(None, 1).await?;
        Ok(())
    }

    async fn full_snapshot(&self) -> Result<Vec<PaperReference>, PaperProviderError> {
        let raw_items = self.inner.get_items(None).await?;
        Ok(extract_references(&raw_items))
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperProviderError> {
        let items = self.fetch_items(Some(query), limit).await?;
        debug!("fetched {} papers from {}", items.len(), self.name());
        Ok(items)
    }
}

#[cfg(feature = "zotero")]
fn extract_references(value: &Value) -> Vec<PaperReference> {
    const ZOTERO_URL_PREFIX: &str = "zotero://select/library/items/";
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

#[cfg(not(feature = "zotero"))]
#[allow(dead_code)]
fn extract_references(_: &serde_json::Value) -> Vec<PaperReference> {
    vec![]
}
