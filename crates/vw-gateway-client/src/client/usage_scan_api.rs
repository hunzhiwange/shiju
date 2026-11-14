//! Host scan operations and recoverable snapshots over the existing JSON SSE transport.
use super::GatewayClient;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_api_types::id::ProjectId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScanScope {
    pub project_id: ProjectId,
    pub target_host: String,
}
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
    pub scope: vw_api_types::ProjectScope,
    pub target_host: String,
    pub statistics_revisions: BTreeMap<String, u64>,
    pub cache_updates: Vec<UsageCacheRevision>,
    pub recover: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum UsageScanError {
    Unauthenticated,
    InvalidInput(String),
    Transport(String),
    Http { status: u16, body: Value },
    Decode(String),
}
impl std::fmt::Display for UsageScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthenticated => f.write_str("usage scans require gateway credentials"),
            Self::InvalidInput(s) | Self::Transport(s) | Self::Decode(s) => f.write_str(s),
            Self::Http { status, .. } => write!(f, "usage HTTP {status}"),
        }
    }
}
impl std::error::Error for UsageScanError {}
impl GatewayClient {
    pub async fn usage_start_scan(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["scans"], scope, None).await
    }
    pub async fn usage_last_scan(&self, scope: &UsageScanScope) -> Result<UsageLastScan, UsageScanError> {
        self.usage_scan_request(reqwest::Method::GET, &["scans", "last"], scope, None).await
    }
    pub async fn usage_job(&self, scope: &UsageScanScope, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        validate_id(id)?;
        self.usage_scan_request(reqwest::Method::GET, &["jobs", id], scope, None).await
    }
    pub async fn usage_cancel_job(&self, scope: &UsageScanScope, id: &str) -> Result<UsageJobSnapshot, UsageScanError> {
        validate_id(id)?;
        self.usage_scan_request(reqwest::Method::POST, &["jobs", id, "cancel"], scope, None).await
    }
    pub async fn usage_rebuild_codex(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "codex-rebuild"], scope, None).await
    }
    pub async fn usage_rollup(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "rollup"], scope, None).await
    }
    pub async fn usage_backfill(&self, scope: &UsageScanScope) -> Result<UsageJobSnapshot, UsageScanError> {
        self.usage_scan_request(reqwest::Method::POST, &["maintenance", "backfill"], scope, None).await
    }
    pub async fn usage_source_settings(&self, scope: &UsageScanScope) -> Result<Vec<UsageSourceSettingsState>, UsageScanError> {
        self.usage_scan_request(reqwest::Method::GET, &["settings"], scope, None).await
    }
    pub async fn usage_save_source_settings(&self, scope: &UsageScanScope, app: &str, expected_revision: u64, settings: &UsageSourceSettings) -> Result<UsageSettingsSaved, UsageScanError> {
        self.usage_scan_request(reqwest::Method::PUT, &["settings"], scope,
            Some(json!({"app":app,"expected_revision":expected_revision,"settings":settings}))).await
    }
    pub async fn usage_activity(&self, scope: &UsageScanScope, active: bool, paused: bool) -> Result<(), UsageScanError> {
        let _: Value = self.usage_scan_request(reqwest::Method::POST, &["activity"], scope,
            Some(json!({"active":active,"paused":paused}))).await?;
        Ok(())
    }
    /// A new connection receives a recovery snapshot; progress is queried with
    /// usage_job. HTTP/stream failures require reconnect and a fresh query.
    pub async fn stream_usage_revisions(&self, scope: &UsageScanScope, mut on_event: impl FnMut(Result<UsageRevisionEvent, UsageScanError>) -> bool) -> Result<crate::stream::GatewayStreamAttemptResult, UsageScanError> {
        self.validate_usage_scan_scope(scope)?;
        self.native_transport().map_err(UsageScanError::Transport)?;
        let project: &str = scope.project_id.as_ref();
        let query = vec![("project_id".into(), project.into()), ("target_host".into(), scope.target_host.clone())];
        crate::stream::stream_json_sse(&self.endpoint, "/v1/usage/events", &query, |event| {
            let decoded = serde_json::from_value::<UsageRevisionEvent>(event.data)
                .map_err(|e| UsageScanError::Decode(e.to_string()))
                .and_then(|event| {
                    if event.event_type != "usage-revision"
                        || event.scope.project_id() != &scope.project_id
                        || event.scope.tenant_scope() != self.endpoint.tenant_scope()
                        || event.target_host != scope.target_host {
                        return Err(UsageScanError::Decode("usage event scope or type mismatch".into()));
                    }
                    Ok(event)
                });
            let invalid = decoded.is_err();
            let keep_reading = on_event(decoded);
            keep_reading && !invalid
        }).await.map_err(UsageScanError::Transport)
    }
    fn validate_usage_scan_scope(&self, scope: &UsageScanScope) -> Result<(), UsageScanError> {
        if !self.endpoint.auth.as_ref().and_then(|a| a.skey.as_deref()).is_some_and(|s| !s.trim().is_empty()) {
            return Err(UsageScanError::Unauthenticated);
        }
        if scope.project_id.as_ref().trim().is_empty() || scope.target_host.trim().is_empty() {
            return Err(UsageScanError::InvalidInput("usage project and host are required".into()));
        }
        Ok(())
    }
    async fn usage_scan_request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &[&str], scope: &UsageScanScope, extra: Option<Value>) -> Result<T, UsageScanError> {
        self.validate_usage_scan_scope(scope)?;
        let mut url = reqwest::Url::parse(&self.endpoint.base_url()).map_err(|e| UsageScanError::InvalidInput(e.to_string()))?;
        url.path_segments_mut().map_err(|_| UsageScanError::InvalidInput("invalid gateway URL".into()))?
            .pop_if_empty().extend(["v1", "usage"]).extend(path.iter().copied());
        let mut request = self.native_transport().map_err(UsageScanError::Transport)?.request(method.clone(), url);
        if method == reqwest::Method::GET {
            request = request.query(scope);
        } else {
            let mut body = serde_json::to_value(scope).map_err(|e| UsageScanError::InvalidInput(e.to_string()))?;
            if let Some(Value::Object(fields)) = extra { body.as_object_mut().expect("scope object").extend(fields); }
            request = request.json(&body);
        }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|e| UsageScanError::Transport(e.to_string()))?;
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
