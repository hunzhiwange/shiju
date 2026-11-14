//! Fixed workspace routes. Content never appears in diagnostic errors.
use super::GatewayClient;
use serde::{Deserialize, Serialize};
use vw_api_types::{agent_apps::AppOperationStatus, id::ProjectId, workspace_memory::*};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMemoryScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMemoryError { InvalidInput, Transport, Http(u16), Decode, Cancelled }
impl std::fmt::Display for WorkspaceMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid workspace request"),
            Self::Transport => f.write_str("workspace gateway unavailable"),
            Self::Http(status) => write!(f, "workspace gateway HTTP {status}"),
            Self::Decode => f.write_str("invalid workspace gateway response"),
            Self::Cancelled => f.write_str("workspace search cancelled"),
        }
    }
}
impl std::error::Error for WorkspaceMemoryError {}

#[derive(Deserialize)]
pub struct WorkspaceSearchResponse {
    #[serde(flatten)]
    pub progress: AppOperationStatus,
    pub results: Option<Vec<DailyMemorySearchResult>>,
}

/// Bound to the submitting client and scope. Keep this handle to release server
/// results with cancel, including after completion. Dropping it does not send IO.
pub struct WorkspaceSearch {
    client: GatewayClient,
    scope: WorkspaceMemoryScope,
    id: String,
    cancelled: bool,
}
impl WorkspaceSearch {
    pub async fn poll(&mut self, token: &str) -> Result<WorkspaceSearchResponse, WorkspaceMemoryError> {
        if self.cancelled { return Err(WorkspaceMemoryError::Cancelled); }
        self.client.workspace_request(reqwest::Method::GET, &format!("search/{}", self.id), &self.scope, token, None).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)
    }
    /// Stop local result consumption before sending cancellation. Failure may be
    /// retried with this handle; polling remains disabled even if DELETE fails.
    pub async fn cancel(&mut self, token: &str) -> Result<AppOperationStatus, WorkspaceMemoryError> {
        self.cancelled = true;
        let response: WorkspaceSearchResponse = self.client.workspace_request(reqwest::Method::DELETE, &format!("search/{}", self.id), &self.scope, token, None).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)?;
        Ok(response.progress)
    }
}

impl GatewayClient {
    pub async fn read_workspace_file(&self, scope: &WorkspaceMemoryScope, filename: WorkspaceFilename, token: &str) -> Result<WorkspaceFileContent, WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::GET, &format!("files/{}", filename.as_str()), scope, token, None).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)
    }
    pub async fn write_workspace_file(&self, scope: &WorkspaceMemoryScope, filename: WorkspaceFilename, content: String, token: &str) -> Result<(), WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::PUT, &format!("files/{}", filename.as_str()), scope, token, Some(serde_json::json!({"content":content}))).await?;
        Ok(())
    }
    pub async fn read_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, token: &str) -> Result<WorkspaceFileContent, WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::GET, &format!("daily/{}", filename.as_str()), scope, token, None).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)
    }
    pub async fn write_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, content: String, token: &str) -> Result<(), WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::PUT, &format!("daily/{}", filename.as_str()), scope, token, Some(serde_json::json!({"content":content}))).await?;
        Ok(())
    }
    pub async fn list_daily_memory_files(&self, scope: &WorkspaceMemoryScope, token: &str) -> Result<Vec<DailyMemoryFileInfo>, WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::GET, "daily", scope, token, None).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)
    }
    pub async fn delete_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, token: &str) -> Result<(), WorkspaceMemoryError> {
        self.workspace_request(reqwest::Method::DELETE, &format!("daily/{}", filename.as_str()), scope, token, None).await?;
        Ok(())
    }
    /// Submission returns progress, never implies search completion. No hidden
    /// polling or detached task continues after callers stop consuming the handle.
    pub async fn search_daily_memory_files(&self, scope: &WorkspaceMemoryScope, query: String, token: &str) -> Result<(WorkspaceSearch, WorkspaceSearchResponse), WorkspaceMemoryError> {
        let response: WorkspaceSearchResponse = self.workspace_request(reqwest::Method::POST, "search", scope, token, Some(serde_json::json!({"query":query}))).await?
            .json().await.map_err(|_| WorkspaceMemoryError::Decode)?;
        // The server issues UUIDs. Reject path syntax instead of accepting an
        // arbitrary resource path from an untrusted response.
        let id = response.progress.operation_id.to_string();
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(WorkspaceMemoryError::Decode);
        }
        Ok((WorkspaceSearch { client: self.clone(), scope: scope.clone(), id, cancelled: false }, response))
    }
    pub(super) async fn workspace_request(&self, method: reqwest::Method, resource: &str, scope: &WorkspaceMemoryScope, token: &str, body: Option<serde_json::Value>) -> Result<reqwest::Response, WorkspaceMemoryError> {
        use WorkspaceMemoryError as E;
        if scope.target_host_id.trim().is_empty() || token.trim().is_empty() { return Err(E::InvalidInput); }
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        credential.set_sensitive(true);
        let mut request = self.native_transport().map_err(|_| E::Transport)?
            .request(method, format!("{}/v1/agent-apps/openclaw/workspace/{resource}", self.endpoint.base_url()))
            .query(scope).header("x-agent-apps-token", credential);
        if let Some(body) = body { request = request.json(&body); }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|_| E::Transport)?;
        if !response.status().is_success() { return Err(E::Http(response.status().as_u16())); }
        Ok(response)
    }
}
