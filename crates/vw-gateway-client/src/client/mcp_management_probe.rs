//! Explicit MCP checks and status snapshots share the execution host's diagnostic jobs.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::Serialize;
use vw_api_types::{
    TaskId,
    agent_apps_diagnostics::{DiagnosticJobIdentity, DiagnosticJobSnapshot, DiagnosticScopeQuery},
    mcp_management::McpRecordId,
};

impl GatewayClient {
    /// Start an explicit connection check for the caller generation.
    /// Acceptance is not connection success; match results by task, generation and revision.
    pub async fn mcp_management_check(
        &self,
        scope: &DiagnosticScopeQuery,
        server_ids: &[McpRecordId],
        generation: u64,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AgentAppsSettingsError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ChecksRequest<'a> {
            server_ids: &'a [McpRecordId],
            generation: u64,
        }
        self.mcp_probe_request(
            reqwest::Method::POST,
            &["mcp", "checks"],
            scope,
            Some(&ChecksRequest { server_ids, generation }),
            token,
        )
        .await
    }

    /// Poll the original job's state, progress and revision-bound partial/final results.
    pub async fn mcp_management_check_status(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        self.mcp_probe_job(scope, task_id, token, false).await
    }

    /// Request cancellation. `Cancelling` is not confirmation that cleanup has finished.
    pub async fn mcp_management_check_cancel(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        self.mcp_probe_job(scope, task_id, token, true).await
    }

    async fn mcp_probe_job(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
        cancel: bool,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        let id = task_id.as_ref();
        if id.is_empty() || matches!(id, "." | "..") {
            return Err(AgentAppsSettingsError::Transport(
                "invalid diagnostic job identity".into(),
            ));
        }
        let mut segments = vec!["diagnostics", "jobs", id];
        if cancel {
            segments.push("cancel");
        }
        self.mcp_probe_request::<(), _>(
            if cancel { reqwest::Method::POST } else { reqwest::Method::GET },
            &segments,
            scope,
            None,
            token,
        )
        .await
    }

    async fn mcp_probe_request<B: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        segments: &[&str],
        scope: &DiagnosticScopeQuery,
        body: Option<&B>,
        token: &str,
    ) -> Result<T, AgentAppsSettingsError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, segments, scope, body, token);
            Err(AgentAppsSettingsError::Transport(
                "MCP checks require native transport with redirects and retries disabled".into(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut url =
                reqwest::Url::parse(&format!("{}/v1/agent-apps", self.endpoint.base_url()))
                    .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            url.path_segments_mut()
                .map_err(|_| AgentAppsSettingsError::Transport("invalid gateway URL".into()))?
                .extend(segments);
            let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsSettingsError::Transport("invalid application credential".into())
            })?;
            credential.set_sensitive(true);
            let mut request = self
                .dynamic_host_client
                .request(method, url)
                .query(scope)
                .header("x-agent-apps-token", credential);
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            let status = response.status();
            let bytes = response
                .bytes()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                    serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
                });
                return Err(AgentAppsSettingsError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes)
                .map_err(|err| AgentAppsSettingsError::Decode(err.to_string()))
        }
    }
}
