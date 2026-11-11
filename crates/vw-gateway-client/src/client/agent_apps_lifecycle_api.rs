//! Typed lifecycle requests. Recovery reads the existing operation; writes are never replayed.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{agent_apps::*, id::TaskId};

/// Transport, malformed responses and original gateway business errors stay distinct.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppsLifecycleError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AgentAppsLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, body } => write!(f, "application lifecycle HTTP {status}: {body}"),
        }
    }
}
impl std::error::Error for AgentAppsLifecycleError {}

impl GatewayClient {
    /// Preview an action; only the returned preview reference can be submitted.
    pub async fn agent_apps_action_preview(
        &self,
        host: &str,
        request: &AppActionPreviewRequest,
    ) -> Result<AppActionPreview, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request(
            reqwest::Method::POST,
            "actions/preview",
            host,
            Some(request),
            None,
        )
        .await
    }
    /// Submit once. An uncertain transport result must be reconciled, never auto-replayed.
    pub async fn agent_apps_action_submit(
        &self,
        host: &str,
        request: &AppActionSubmitRequest,
    ) -> Result<AppActionExecutionResult, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request(
            reqwest::Method::POST,
            "actions",
            host,
            Some(request),
            None,
        )
        .await
    }
    /// Recover current state using the operation ID returned by submission.
    pub async fn agent_apps_operation(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::GET,
            &operation_path(id, false),
            host,
            None,
            None,
        )
        .await
    }
    /// Wait for the next current-state snapshot via the gateway's long-poll contract.
    /// After disconnect, query `agent_apps_operation` with the same ID before polling again.
    pub async fn agent_apps_operation_events(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::GET,
            &operation_path(id, true),
            host,
            None,
            None,
        )
        .await
    }
    /// Cancel the same operation; the gateway waits for its process to stop.
    pub async fn agent_apps_operation_cancel(
        &self,
        host: &str,
        id: &TaskId,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::DELETE,
            &operation_path(id, false),
            host,
            None,
            None,
        )
        .await
    }
    /// Scoped application credential, with redirects and retries disabled.
    pub async fn agent_apps_action_preview_authorized(
        &self,
        host: &str,
        request: &AppActionPreviewRequest,
        token: &str,
    ) -> Result<AppActionPreview, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request(
            reqwest::Method::POST,
            "actions/preview",
            host,
            Some(request),
            Some(token),
        )
        .await
    }
    /// Scoped application credential, with redirects and retries disabled.
    pub async fn agent_apps_action_submit_authorized(
        &self,
        host: &str,
        request: &AppActionSubmitRequest,
        token: &str,
    ) -> Result<AppActionExecutionResult, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request(
            reqwest::Method::POST,
            "actions",
            host,
            Some(request),
            Some(token),
        )
        .await
    }
    /// Scoped application credential, with redirects and retries disabled.
    pub async fn agent_apps_operation_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::GET,
            &operation_path(id, false),
            host,
            None,
            Some(token),
        )
        .await
    }
    /// Scoped application credential, with redirects and retries disabled.
    pub async fn agent_apps_operation_events_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::GET,
            &operation_path(id, true),
            host,
            None,
            Some(token),
        )
        .await
    }
    /// Scoped application credential, with redirects and retries disabled.
    pub async fn agent_apps_operation_cancel_authorized(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<AppOperationStatus, AgentAppsLifecycleError> {
        self.agent_apps_lifecycle_request::<(), _>(
            reqwest::Method::DELETE,
            &operation_path(id, false),
            host,
            None,
            Some(token),
        )
        .await
    }
    pub(super) async fn agent_apps_lifecycle_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        host: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<T, AgentAppsLifecycleError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, resource, host, body, token);
            Err(AgentAppsLifecycleError::Transport(
                "lifecycle requests require a native transport with redirects and retries disabled"
                    .into(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // This existing transport disables redirects and retry, including on ambiguous writes.
            let mut request = self
                .dynamic_host_client
                .request(method, format!("{}/v1/agent-apps/{resource}", self.endpoint.base_url()))
                .query(&[("targetHostId", host)]);
            if let Some(token) = token {
                let mut value = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                    AgentAppsLifecycleError::Transport("invalid application credential".into())
                })?;
                value.set_sensitive(true);
                request = request.header("x-agent-apps-token", value);
            }
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|err| AgentAppsLifecycleError::Transport(err.to_string()))?;
            let status = response.status();
            let bytes = response
                .bytes()
                .await
                .map_err(|err| AgentAppsLifecycleError::Transport(err.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                    serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
                });
                return Err(AgentAppsLifecycleError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes)
                .map_err(|err| AgentAppsLifecycleError::Decode(err.to_string()))
        }
    }
}
pub(super) fn operation_path(id: &TaskId, events: bool) -> String {
    let mut encoded = String::new();
    for byte in id.0.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte))
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("operations/{encoded}{}", if events { "/events" } else { "" })
}
