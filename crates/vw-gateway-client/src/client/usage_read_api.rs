//! Read-only usage queries. The gateway authorizes project/host selectors against
//! the endpoint identity; selectors never grant access by themselves.
use super::{GatewayClient, usage_read_types::*};
use serde::de::DeserializeOwned;
use vw_api_types::{id::ProjectId, usage::*};
use std::collections::BTreeMap;

/// Explicit target and timezone for every read; no implicit host or project.
#[derive(Debug, Clone)]
pub struct UsageReadScope {
    pub project_id: ProjectId,
    pub target_host: String,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsageReadError {
    InvalidInput(String),
    Unauthenticated,
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for UsageReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(s) | Self::Transport(s) | Self::Decode(s) => f.write_str(s),
            Self::Unauthenticated => f.write_str("usage read requires gateway credentials"),
            Self::Http { status, .. } => write!(f, "usage HTTP {status}"),
        }
    }
}
impl std::error::Error for UsageReadError {}

impl GatewayClient {
    pub async fn usage_logs(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageLogResult, UsageReadError> {
        self.usage_read(&["logs"], scope, query, None).await
    }
    pub async fn usage_sources(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<UsageSourceDistribution>, UsageReadError> {
        self.usage_read(&["sources"], scope, query, None).await
    }
    pub async fn usage_summary(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageSummary, UsageReadError> {
        self.usage_read(&["summary"], scope, query, None).await
    }
    pub async fn usage_summary_apps(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<BTreeMap<String, UsageSummary>, UsageReadError> {
        self.usage_read(&["summary", "apps"], scope, query, None).await
    }
    pub async fn usage_summary_sessions(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<UsageSessionSummary>, UsageReadError> {
        self.usage_read(&["summary", "sessions"], scope, query, None).await
    }
    pub async fn usage_first_date(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Option<UsageUnixSeconds>, UsageReadError> {
        self.usage_read(&["first-date"], scope, query, None).await
    }
    pub async fn usage_provider_stats(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<ProviderStats>, UsageReadError> {
        self.usage_read(&["stats", "providers"], scope, query, None).await
    }
    pub async fn usage_model_stats(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<Vec<ModelStats>, UsageReadError> {
        self.usage_read(&["stats", "models"], scope, query, None).await
    }
    pub async fn usage_heatmap(&self, scope: &UsageReadScope, query: &UsageQuery) -> Result<UsageSeries, UsageReadError> {
        self.usage_read(&["heatmap"], scope, query, None).await
    }
    pub async fn usage_log_detail(&self, scope: &UsageReadScope, query: &UsageQuery, id: &str) -> Result<UsageLogDetail, UsageReadError> {
        if id.is_empty() || id == "." || id == ".." {
            return Err(UsageReadError::InvalidInput("invalid usage record id".into()));
        }
        self.usage_read(&["logs", id], scope, query, None).await
    }
    pub async fn usage_trends(&self, scope: &UsageReadScope, query: &UsageQuery, interval: Option<UsageSeriesInterval>) -> Result<UsageSeries, UsageReadError> {
        self.usage_read(&["trends"], scope, query, interval).await
    }
    async fn usage_read<T: DeserializeOwned>(&self, resource: &[&str], scope: &UsageReadScope, query: &UsageQuery, interval: Option<UsageSeriesInterval>) -> Result<T, UsageReadError> {
        if !self.endpoint.auth.as_ref().and_then(|a| a.skey.as_deref()).is_some_and(|key| !key.trim().is_empty()) {
            return Err(UsageReadError::Unauthenticated);
        }
        if scope.project_id.as_ref().trim().is_empty() || scope.target_host.trim().is_empty() || scope.time_zone.trim().is_empty() {
            return Err(UsageReadError::InvalidInput("usage project, host and timezone are required".into()));
        }
        let mut url = reqwest::Url::parse(&self.endpoint.base_url())
            .map_err(|e| UsageReadError::InvalidInput(e.to_string()))?;
        url.path_segments_mut().map_err(|_| UsageReadError::InvalidInput("invalid gateway URL".into()))?
            .pop_if_empty().extend(["v1", "usage"]).extend(resource.iter().copied());
        let project: &str = scope.project_id.as_ref();
        let mut pairs = vec![
            ("project_id", project.to_owned()),
            ("target_host", scope.target_host.clone()),
            ("time_zone", scope.time_zone.clone()),
            ("query", serde_json::to_string(query).map_err(|e| UsageReadError::InvalidInput(e.to_string()))?),
        ];
        if let Some(interval) = interval {
            pairs.push(("interval", match interval { UsageSeriesInterval::Hour => "hour", UsageSeriesInterval::Day => "day" }.into()));
        }
        let request = self.native_transport().map_err(UsageReadError::Transport)?.get(url).query(&pairs);
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await
            .map_err(|e| UsageReadError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| UsageReadError::Transport(e.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned()));
            return Err(UsageReadError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|e| UsageReadError::Decode(e.to_string()))
    }
}
