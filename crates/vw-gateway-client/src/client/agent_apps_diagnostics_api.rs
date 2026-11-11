//! Diagnostic submission and polling; probes and credentials stay on the execution host.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{
    TaskId,
    agent_apps_diagnostics::{
        DiagnosticEndpointCandidatesQuery, DiagnosticEndpointCandidatesResponse,
        DiagnosticJobIdentity, DiagnosticJobSnapshot, DiagnosticScopeQuery, DiagnosticStartRequest,
    },
};

impl GatewayClient {
    /// Read host-normalized candidates and their sources without probing or saving drafts.
    pub async fn diagnostic_endpoint_candidates(
        &self,
        scope: &DiagnosticScopeQuery,
        query: &DiagnosticEndpointCandidatesQuery,
        token: &str,
    ) -> Result<DiagnosticEndpointCandidatesResponse, AgentAppsSettingsError> {
        self.diagnostic_request(
            reqwest::Method::POST,
            &["endpoint-candidates", "query"],
            scope,
            Some(query),
            token,
        )
        .await
    }

    /// Submit one probe batch. Acceptance identifies a job, not a successful probe.
    /// An ambiguous failure must not be retried automatically.
    pub async fn diagnostic_start(
        &self,
        scope: &DiagnosticScopeQuery,
        request: &DiagnosticStartRequest,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AgentAppsSettingsError> {
        self.diagnostic_request(reqwest::Method::POST, &["jobs"], scope, Some(request), token).await
    }

    /// Read the original task's progress and partial/final evidence in its authorized scope.
    pub async fn diagnostic_status(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        self.diagnostic_job(scope, task_id, token, false).await
    }

    /// Request cleanup. Only `Cancelled` confirms cleanup has finished; keep polling
    /// a `Cancelling` snapshot. Terminal jobs retain their server-issued state.
    pub async fn diagnostic_cancel(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        self.diagnostic_job(scope, task_id, token, true).await
    }

    async fn diagnostic_job(
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
        let segments = if cancel { vec!["jobs", id, "cancel"] } else { vec!["jobs", id] };
        self.diagnostic_request::<(), _>(
            if cancel { reqwest::Method::POST } else { reqwest::Method::GET },
            &segments,
            scope,
            None,
            token,
        )
        .await
    }

    async fn diagnostic_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        segments: &[&str],
        scope: &DiagnosticScopeQuery,
        body: Option<&B>,
        token: &str,
    ) -> Result<T, AgentAppsSettingsError> {
        let mut url =
            reqwest::Url::parse(&format!("{}/v1/agent-apps/diagnostics", self.endpoint.base_url()))
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
