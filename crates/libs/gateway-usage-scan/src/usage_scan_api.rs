//! Host scan operations and recoverable snapshots over the existing JSON SSE transport.
use super::GatewayUsageScanClient;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSourceSettings {
    pub scan_enabled: bool,
    pub show_usage: bool,
    pub refresh_seconds: Option<u32>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsageSourceSettingsState {
    pub app: String,
    pub settings: UsageSourceSettings,
    pub revision: u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsageSettingsSaved {
    pub settings: UsageSourceSettings,
    pub revision: u64,
}
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct UsageScanPreferences {
    pub enabled: bool,
    pub refresh_seconds: u32,
}
/// Both scanner and maintenance snapshots retain all host progress/error fields.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageJobSnapshot {
    pub task_id: String,
    #[serde(flatten)]
    pub progress: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsageLastScan {
    pub last_complete_scan: Option<std::time::SystemTime>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UsageCacheRevision {
    pub app: String,
    pub provider_id: String,
    pub revision: String,
}
/// Invalidation snapshot, not a task completion notification. Requery the named
/// job/settings/read APIs after recover=true or when revisions change.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageRevisionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub tenant: vw_api_transport::TenantScope,
    pub statistics_revisions: BTreeMap<String, u64>,
    pub cache_updates: Vec<UsageCacheRevision>,
    pub recover: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum UsageScanError {
    InvalidInput(String),
    Transport(String),
    Http { status: u16, body: Value },
    Decode(String),
}
impl std::fmt::Display for UsageScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(s) | Self::Transport(s) | Self::Decode(s) => f.write_str(s),
            Self::Http { status, .. } => write!(f, "usage HTTP {status}"),
        }
    }
}
impl std::error::Error for UsageScanError {}
impl GatewayUsageScanClient {
    pub async fn usage_start_scan(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["scans"], None).await
    }
    pub async fn usage_last_scan(&self) -> Result<UsageLastScan, UsageScanError> {
        self.usage_scan_request(reqwest::Method::GET, &["scans", "last"], None).await
    }
    pub async fn usage_job(&self, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        validate_id(id)?;
        self.usage_scan_request(reqwest::Method::GET, &["jobs", id], None).await
    }
    pub async fn usage_cancel_job(&self, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        validate_id(id)?;
        self.usage_scan_request(reqwest::Method::POST, &["jobs", id, "cancel"], None).await
    }
    pub async fn usage_rebuild_codex(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "codex-rebuild"], None).await
    }
    pub async fn usage_rollup(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "rollup"], None).await
    }
    pub async fn usage_backfill(&self) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "backfill"], None).await
    }
    pub async fn usage_source_settings(&self) -> Result<Vec<UsageSourceSettingsState>, UsageScanError> {
        self.usage_scan_request(reqwest::Method::GET, &["settings"], None).await
    }
    pub async fn usage_save_source_settings(&self, app: &str, expected_revision: u64, settings: &UsageSourceSettings) -> Result<UsageSettingsSaved, UsageScanError> {
        self.usage_scan_request(reqwest::Method::PUT, &["settings"],
            Some(json!({"app":app,"expected_revision":expected_revision,"settings":settings}))).await
    }
    pub async fn usage_scan_preferences(&self) -> Result<UsageScanPreferences, UsageScanError> {
        self.usage_scan_request(reqwest::Method::GET, &["settings", "scan"], None).await
    }
    pub async fn usage_save_scan_preferences(&self, preferences: UsageScanPreferences) -> Result<UsageScanPreferences, UsageScanError> {
        self.usage_scan_request(reqwest::Method::PUT, &["settings", "scan"],
            Some(json!({"enabled":preferences.enabled,"refresh_seconds":preferences.refresh_seconds}))).await
    }
    pub async fn usage_page_activity(&self, active: bool) -> Result<(), UsageScanError> {
        let _: Value = self.usage_scan_request(reqwest::Method::POST, &["activity"],
            Some(json!({"active":active}))).await?;
        Ok(())
    }
    pub async fn usage_activity(&self, active: bool, paused: bool) -> Result<(), UsageScanError> {
        let _: Value = self.usage_scan_request(reqwest::Method::POST, &["activity"],
            Some(json!({"active":active,"paused":paused}))).await?;
        Ok(())
    }
    /// A new connection receives a recovery snapshot; progress is queried with
    /// usage_job. HTTP/stream failures require reconnect and a fresh query.
    pub async fn stream_usage_revisions(&self, mut on_event: impl FnMut(Result<UsageRevisionEvent, UsageScanError>) -> bool) -> Result<gateway_chat_stream::GatewayStreamAttemptResult, UsageScanError> {
        self.transport.native_transport().map_err(UsageScanError::Transport)?;
        gateway_chat_stream::stream_json_sse(self.transport.endpoint(), "/v1/usage/events", &[], |event| {
            let decoded = serde_json::from_value::<UsageRevisionEvent>(event.data)
                .map_err(|e| UsageScanError::Decode(e.to_string()))
                .and_then(|event| {
                    if event.event_type != "usage-revision"
                        || &event.tenant != self.transport.endpoint().tenant_scope() {
                        return Err(UsageScanError::Decode("usage event tenant or type mismatch".into()));
                    }
                    Ok(event)
                });
            let invalid = decoded.is_err();
            let keep_reading = on_event(decoded);
            keep_reading && !invalid
        }).await.map_err(UsageScanError::Transport)
    }
    async fn usage_scan_request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &[&str], extra: Option<Value>) -> Result<T, UsageScanError> {
        let mut url = reqwest::Url::parse(&self.transport.endpoint().base_url()).map_err(|e| UsageScanError::InvalidInput(e.to_string()))?;
        url.path_segments_mut().map_err(|_| UsageScanError::InvalidInput("invalid gateway URL".into()))?
            .pop_if_empty().extend(["v1", "usage"]).extend(path.iter().copied());
        let with_body = method != reqwest::Method::GET;
        let mut request = self.transport.native_transport().map_err(UsageScanError::Transport)?.request(method, url);
        if with_body {
            request = request.json(&extra.unwrap_or_else(|| json!({})));
        }
        let response = gateway_http::apply_request_context(request, self.transport.endpoint()).send().await.map_err(|e| UsageScanError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| UsageScanError::Transport(e.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
            return Err(UsageScanError::Http { status: status.as_u16(), body });
        }
        let body = if bytes.is_empty() { b"null".as_slice() } else { bytes.as_ref() };
        serde_json::from_slice(body).map_err(|e| UsageScanError::Decode(e.to_string()))
    }
}
fn validate_id(id: &str) -> Result<(), UsageScanError> {
    if id.trim().is_empty() || matches!(id, "." | "..") { return Err(UsageScanError::InvalidInput("invalid usage task id".into())); }
    Ok(())
}
