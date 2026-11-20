//! Read-only queries for the gateway’s local usage ledger.
use crate::{GatewayUsageReadClient, usage_read_types::*};
use serde::de::DeserializeOwned;
use vw_api_usage::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum UsageReadError {
    InvalidInput(String),
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for UsageReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(s) | Self::Transport(s) | Self::Decode(s) => f.write_str(s),
            Self::Http { status, .. } => write!(f, "usage HTTP {status}"),
        }
    }
}
impl std::error::Error for UsageReadError {}

impl GatewayUsageReadClient {
    pub async fn usage_logs(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageLogResult, UsageReadError> {
        self.usage_read(&["logs"], time_zone, query, None).await
    }
    pub async fn usage_log_page(&self, time_zone: &str, query: &UsageQuery, page: u64) -> Result<UsageLogResult, UsageReadError> {
        self.usage_read(&["logs"], time_zone, query, Some(("page", page.to_string()))).await
    }
    pub async fn usage_sources(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<UsageSourceDistribution>, UsageReadError> {
        self.usage_read(&["sources"], time_zone, query, None).await
    }
    pub async fn usage_summary(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageSummary, UsageReadError> {
        self.usage_read(&["summary"], time_zone, query, None).await
    }
    pub async fn usage_summary_apps(&self, time_zone: &str, query: &UsageQuery) -> Result<BTreeMap<String, UsageSummary>, UsageReadError> {
        self.usage_read(&["summary", "apps"], time_zone, query, None).await
    }
    pub async fn usage_summary_sessions(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<UsageSessionSummary>, UsageReadError> {
        self.usage_read(&["summary", "sessions"], time_zone, query, None).await
    }
    pub async fn usage_first_date(&self, time_zone: &str, query: &UsageQuery) -> Result<Option<UsageUnixSeconds>, UsageReadError> {
        self.usage_read(&["first-date"], time_zone, query, None).await
    }
    pub async fn usage_provider_stats(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<ProviderStats>, UsageReadError> {
        self.usage_read(&["stats", "providers"], time_zone, query, None).await
    }
    pub async fn usage_model_stats(&self, time_zone: &str, query: &UsageQuery) -> Result<Vec<ModelStats>, UsageReadError> {
        self.usage_read(&["stats", "models"], time_zone, query, None).await
    }
    pub async fn usage_heatmap(&self, time_zone: &str, query: &UsageQuery) -> Result<UsageSeries, UsageReadError> {
        self.usage_read(&["heatmap"], time_zone, query, None).await
    }
    pub async fn usage_log_detail(&self, time_zone: &str, query: &UsageQuery, id: &str) -> Result<UsageLogDetail, UsageReadError> {
        if id.is_empty() || id == "." || id == ".." {
            return Err(UsageReadError::InvalidInput("invalid usage record id".into()));
        }
        self.usage_read(&["logs", id], time_zone, query, None).await
    }
    pub async fn usage_trends(&self, time_zone: &str, query: &UsageQuery, interval: Option<UsageSeriesInterval>) -> Result<UsageSeries, UsageReadError> {
        let interval = interval.map(|interval| {
            let value = match interval { UsageSeriesInterval::Hour => "hour", UsageSeriesInterval::Day => "day" };
            ("interval", value.to_owned())
        });
        self.usage_read(&["trends"], time_zone, query, interval).await
    }
    async fn usage_read<T: DeserializeOwned>(&self, resource: &[&str], time_zone: &str, query: &UsageQuery, parameter: Option<(&str, String)>) -> Result<T, UsageReadError> {
        if time_zone.trim().is_empty() {
            return Err(UsageReadError::InvalidInput("usage timezone is required".into()));
        }
        let mut url = reqwest::Url::parse(&self.transport.endpoint().base_url())
            .map_err(|e| UsageReadError::InvalidInput(e.to_string()))?;
        url.path_segments_mut().map_err(|_| UsageReadError::InvalidInput("invalid gateway URL".into()))?
            .pop_if_empty().extend(["v1", "usage"]).extend(resource.iter().copied());
        let mut pairs = vec![
            ("time_zone", time_zone.to_owned()),
            ("query", serde_json::to_string(query).map_err(|e| UsageReadError::InvalidInput(e.to_string()))?),
        ];
        pairs.extend(parameter);
        let request = self.transport.native_transport().map_err(UsageReadError::Transport)?.get(url).query(&pairs);
        let response = gateway_http::apply_request_context(request, self.transport.endpoint()).send().await
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
