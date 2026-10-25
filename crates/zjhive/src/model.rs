//! Local projections of Hive's operational API, independent of native ACP dependencies.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub revision: u64,
    pub sessions: Vec<Session>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub agent: String,
    pub cwd: String,
    pub title: Option<String>,
    pub lifecycle: Option<String>,
    pub recovery: Option<String>,
    pub recovery_reason: Option<String>,
    pub activity: Option<String>,
    pub last_activity_at: Option<u64>,
    pub control_owner: Option<String>,
    pub generation: Option<u64>,
    pub observers: Option<u64>,
    pub origin: Option<String>,
    pub native_id: Option<String>,
    pub external_activity: Option<bool>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(default)]
    pub tokens: Tokens,
    pub context_used: Option<u64>,
    pub context_size: Option<u64>,
    pub estimated_cost: Option<Cost>,
    pub actual_session_cost: Option<Cost>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub total_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Cost {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub name: String,
    pub version: String,
    pub sessions: u64,
    pub default_agent: Option<String>,
    #[serde(default)]
    pub active_by_agent: BTreeMap<String, u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error(
        "zjhive::dashboard::url: Invalid Hive URL: {0}; configure an HTTP(S) base URL without credentials, query or fragment"
    )]
    Url(String),
    #[error(
        "zjhive::dashboard::http: Hive returned HTTP {0}; check the configured URL and daemon availability"
    )]
    Http(u16),
    #[error(
        "zjhive::dashboard::response: Invalid Hive response: {0}; check daemon compatibility and refresh"
    )]
    Response(String),
    #[error(
        "zjhive::dashboard::overdue: Hive request overdue; retry is bounded, reload the plugin if requests remain stuck"
    )]
    Overdue,
}

pub fn base_url(input: &str) -> Result<url::Url, DashboardError> {
    let mut url = url::Url::parse(input).map_err(|e| DashboardError::Url(e.to_string()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DashboardError::Url("unsupported base URL".into()));
    }
    // A trailing slash makes joining preserve an optional reverse-proxy prefix.
    url.set_path(&format!("{}/", url.path().trim_end_matches('/')));
    Ok(url)
}

pub fn catalog(body: &[u8]) -> Result<Catalog, DashboardError> {
    let catalog: Catalog =
        serde_json::from_slice(body).map_err(|e| DashboardError::Response(e.to_string()))?;
    let mut ids = std::collections::HashSet::new();
    // Duplicate or empty identities would make selection silently target another row.
    if catalog
        .sessions
        .iter()
        .any(|s| s.session_id.is_empty() || s.agent.is_empty() || !ids.insert(&s.session_id))
    {
        return Err(DashboardError::Response(
            "empty or duplicate session identity".into(),
        ));
    }
    Ok(catalog)
}
