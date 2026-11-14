//! Project profile maintenance runs exclusively on the authorized gateway host.
use super::{GatewayClient, project_api::encode_path_segment};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vw_api_types::{ProjectId, ProjectScope, project_profiles::*};

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectProfileApiError {
    InvalidRequest(String),
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for ProjectProfileApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, .. } => write!(f, "project profile HTTP {status}"),
        }
    }
}
impl std::error::Error for ProjectProfileApiError {}

/// Project and tenant are checked against the endpoint before any request.
#[derive(Debug, Clone)]
pub struct ProjectProfileApiScope {
    pub project: ProjectScope,
    pub target_host_id: String,
    pub scope: ProjectProfileScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProfileCurrent {
    pub profile_id: Option<ProjectProfileId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProjectProfileDeleted {
    pub deleted: bool,
}

/// Persisted outcome, read after reconnect without resubmitting application.
/// A missing outcome is not evidence that a previous write never started.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectProfileStoredResult {
    pub result: Option<ProjectProfileApplyResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Context<'a> {
    project_id: &'a ProjectId,
    target_host_id: &'a str,
    scope: ProjectProfileScope,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Mutation<'a> {
    #[serde(flatten)]
    context: Context<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

impl GatewayClient {
    pub async fn project_profile_list(&self, scope: &ProjectProfileApiScope) -> Result<Vec<ProjectProfile>, ProjectProfileApiError> {
        self.project_profile_read(scope, "").await
    }

    pub async fn project_profile_current(&self, scope: &ProjectProfileApiScope) -> Result<ProjectProfileCurrent, ProjectProfileApiError> {
        self.project_profile_read(scope, "/current").await
    }

    /// Snapshot availability is decided by the executor; never substitute local state.
    pub async fn project_profile_create(&self, scope: &ProjectProfileApiScope, name: &str) -> Result<ProjectProfile, ProjectProfileApiError> {
        self.project_profile_write(reqwest::Method::POST, "", scope, None, Some(name)).await
    }

    pub async fn project_profile_rename(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64, name: &str) -> Result<ProjectProfile, ProjectProfileApiError> {
        let path = profile_path(id)?;
        self.project_profile_write(reqwest::Method::PATCH, &path, scope, Some(expected_version), Some(name)).await
    }

    /// Replaces only the requested application group on the execution host.
    pub async fn project_profile_resnapshot(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64) -> Result<ProjectProfile, ProjectProfileApiError> {
        let path = format!("{}/resnapshot", profile_path(id)?);
        self.project_profile_write(reqwest::Method::POST, &path, scope, Some(expected_version), None).await
    }

    pub async fn project_profile_delete(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64) -> Result<ProjectProfileDeleted, ProjectProfileApiError> {
        let path = profile_path(id)?;
        self.project_profile_write(reqwest::Method::DELETE, &path, scope, Some(expected_version), None).await
    }

    /// Clears the selected marker without undoing native configuration.
    pub async fn project_profile_clear(&self, scope: &ProjectProfileApiScope) -> Result<ProjectProfileCurrent, ProjectProfileApiError> {
        self.project_profile_write(reqwest::Method::POST, "/current/clear", scope, None, None).await
    }

    /// Read-only preview on the execution host. The current server returns 503
    /// until native observations are wired; its success schema is not frozen.
    pub async fn project_profile_preview(
        &self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        let path = format!("{}/preview", profile_path(id)?);
        self.project_profile_write(reqwest::Method::POST, &path, scope, Some(expected_version), None).await
    }

    /// Submit exactly once using the native no-retry/no-redirect transport.
    /// On a transport error the outcome is unknown: read current/result after
    /// reconnect. Neither this method nor result reads automatically replay apply.
    /// The server currently returns 503; no successful admission schema exists yet.
    pub async fn project_profile_apply(
        &self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        let path = format!("{}/apply", profile_path(id)?);
        self.project_profile_write(reqwest::Method::POST, &path, scope, Some(expected_version), None).await
    }

    /// Read the latest persisted group result, preserving partial, uncaptured,
    /// cancelled and per-item outcomes. This is not an operation-specific receipt.
    pub async fn project_profile_result(
        &self, scope: &ProjectProfileApiScope,
    ) -> Result<ProjectProfileStoredResult, ProjectProfileApiError> {
        self.project_profile_read(scope, "/current/result").await
    }

    /// Query an admitted operation without restarting it. The current server has
    /// no admitted operations and returns 404; retain that structured error.
    pub async fn project_profile_progress(
        &self, scope: &ProjectProfileApiScope, operation_id: &str,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        let path = operation_path(operation_id)?;
        self.project_profile_read(scope, &path).await
    }

    /// Request cancellation once. Only the executor can confirm cancellation;
    /// a failed request does not synthesize a cancelled result.
    pub async fn project_profile_cancel(
        &self, scope: &ProjectProfileApiScope, operation_id: &str,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        let path = format!("{}/cancel", operation_path(operation_id)?);
        self.project_profile_write(reqwest::Method::POST, &path, scope, None, None).await
    }

    fn profile_context<'a>(&self, scope: &'a ProjectProfileApiScope) -> Result<Context<'a>, ProjectProfileApiError> {
        if scope.project.tenant_scope() != self.endpoint().tenant_scope() {
            return Err(ProjectProfileApiError::InvalidRequest("tenant scope does not match gateway endpoint tenant scope".into()));
        }
        if scope.target_host_id.trim().is_empty() {
            return Err(ProjectProfileApiError::InvalidRequest("target host must not be empty".into()));
        }
        Ok(Context { project_id: scope.project.project_id(), target_host_id: &scope.target_host_id, scope: scope.scope })
    }

    async fn project_profile_read<T: DeserializeOwned>(&self, scope: &ProjectProfileApiScope, suffix: &str) -> Result<T, ProjectProfileApiError> {
        let context = self.profile_context(scope)?;
        let request = self.native_transport().map_err(ProjectProfileApiError::Transport)?
            .get(format!("{}/v1/agent-apps/project-profiles{suffix}", self.endpoint().base_url()))
            .query(&context);
        self.project_profile_response(request).await
    }

    async fn project_profile_write<T: DeserializeOwned>(&self, method: reqwest::Method, suffix: &str, scope: &ProjectProfileApiScope, expected_version: Option<u64>, name: Option<&str>) -> Result<T, ProjectProfileApiError> {
        let context = self.profile_context(scope)?;
        let request = self.native_transport().map_err(ProjectProfileApiError::Transport)?
            .request(method, format!("{}/v1/agent-apps/project-profiles{suffix}", self.endpoint().base_url()))
            .json(&Mutation { context, expected_version, name });
        self.project_profile_response(request).await
    }

    async fn project_profile_response<T: DeserializeOwned>(&self, request: reqwest::RequestBuilder) -> Result<T, ProjectProfileApiError> {
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|e| ProjectProfileApiError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| ProjectProfileApiError::Transport(e.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned()));
            return Err(ProjectProfileApiError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|e| ProjectProfileApiError::Decode(e.to_string()))
    }
}

fn profile_path(id: &ProjectProfileId) -> Result<String, ProjectProfileApiError> {
    if id.0.trim().is_empty() || id.0 == "." || id.0 == ".." {
        return Err(ProjectProfileApiError::InvalidRequest("invalid project profile id".into()));
    }
    Ok(format!("/{}", encode_path_segment(&id.0)))
}

fn operation_path(id: &str) -> Result<String, ProjectProfileApiError> {
    if id.trim().is_empty() || id == "." || id == ".." {
        return Err(ProjectProfileApiError::InvalidRequest("invalid project profile operation id".into()));
    }
    Ok(format!("/operations/{}", encode_path_segment(id)))
}
