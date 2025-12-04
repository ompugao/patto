use crate::lsp_config::PattoLspConfig;
#[cfg(feature = "zotero")]
use crate::lsp_config::ZoteroCredentials;
use async_trait::async_trait;
#[cfg(feature = "zotero")]
use log::debug;
#[cfg(feature = "zotero")]
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

const MIN_QUERY_LEN: usize = 2;
const DEFAULT_LIMIT: usize = 100;
#[cfg(feature = "zotero")]
const HEALTHCHECK_LIMIT: usize = 1;
#[cfg(feature = "zotero")]
const ZOTERO_URL_PREFIX: &str = "zotero://select/library/items/";

type DynPaperClient = dyn PaperClient + Send + Sync;

#[derive(Debug, Clone)]
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

#[async_trait]
pub trait PaperClient: Send + Sync {
    async fn health_check(&self) -> Result<(), PaperClientError>;
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PaperReference>, PaperClientError>;
    fn provider_name(&self) -> &'static str;
}

#[derive(Clone, Default)]
pub struct PaperClientManager {
    client: Option<Arc<DynPaperClient>>,
}

impl PaperClientManager {
    pub fn from_config(config: Option<&PattoLspConfig>) -> Result<Self, PaperClientError> {
        let Some(cfg) = config else {
            return Ok(Self::default());
        };
        let Some(credentials) = cfg.zotero_credentials() else {
            return Ok(Self::default());
        };

        #[cfg(feature = "zotero")]
        {
            let client = Arc::new(ZoteroPaperClient::new(credentials)?);
            return Ok(Self {
                client: Some(client),
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
        if trimmed.len() < MIN_QUERY_LEN {
            return Ok(vec![]);
        }
        if let Some(client) = &self.client {
            client.search(trimmed, DEFAULT_LIMIT).await
        } else {
            Err(PaperClientError::NotConfigured)
        }
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
