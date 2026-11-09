//! Submit usage refreshes, manage their operations, and configure usage scripts.

use super::{
    AppsUsageCredentialRefs, AppsUsageSettings, GatewayClient,
    agent_apps_lifecycle_api::operation_path,
};
use crate::http::apply_request_context;
use reqwest::{
    Method, RequestBuilder, Response, StatusCode,
    header::{CACHE_CONTROL, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vw_api_types::{
    agent_apps::{AppActionExecutionResult, AppId, AppOperationStatus},
    agent_apps_auth::{AuthError, AuthErrorCode},
    apps_usage::{
        UsageQueryError, UsageQueryErrorCode, UsageQueryKind, UsageScriptPackage,
        UsageSnapshotQuery,
    },
    id::{ProjectId, TaskId},
    task::TaskStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsUsageWriteError {
    InvalidPrincipalToken,
    InvalidOperationId,
    Transport,
    Http { status: u16, error: Option<AuthError> },
    ScriptTrial { status: u16, code: UsageQueryErrorCode, correlation_id: String },
    Decode,
}

impl std::fmt::Display for AppsUsageWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrincipalToken => f.write_str("invalid application principal token"),
            Self::InvalidOperationId => f.write_str("invalid usage operation ID"),
            Self::Transport => f.write_str("usage request could not be completed"),
            Self::Http { status, error: Some(error) } => {
                write!(f, "usage request failed with HTTP {status} ({:?})", error.code)
            }
            Self::Http { status, error: None } => {
                write!(f, "usage request failed with HTTP {status}")
            }
            Self::ScriptTrial { status, code, .. } => {
                write!(f, "usage script trial failed with HTTP {status} ({code:?})")
            }
            Self::Decode => f.write_str("invalid usage response"),
        }
    }
}

impl std::error::Error for AppsUsageWriteError {}

/// Current progress for an authorized refresh operation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsUsageOperation {
    #[serde(flatten)]
    pub operation: AppOperationStatus,
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub correlation_id: String,
    pub cancel_requested: bool,
    pub items: Vec<AppsUsageOperationItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsUsageOperationItem {
    pub app_id: AppId,
    pub provider_id: String,
    pub account_id: Option<String>,
    pub kind: UsageQueryKind,
    pub status: TaskStatus,
    pub snapshot_version: Option<String>,
    pub error_code: Option<AuthErrorCode>,
}

/// A transient script trial. Leave the selection's query kind unset.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsUsageScriptTrial {
    pub selection: UsageSnapshotQuery,
    /// Stable identifier returned by the usage templates endpoint.
    pub template: String,
    /// Supply draft source for the custom template.
    pub source: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub credentials: AppsUsageCredentialRefs,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchScope<'a> {
    project_id: &'a ProjectId,
    target_host_id: &'a str,
}

#[derive(Serialize)]
struct SaveSettings<'a> {
    selection: &'a UsageSnapshotQuery,
    settings: &'a AppsUsageSettings,
}

impl GatewayClient {
    /// Queue one refresh and return its server-assigned operation ID.
    pub async fn apps_usage_refresh(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<AppActionExecutionResult, AppsUsageWriteError> {
        let request = self.usage_request(Method::POST, "/v1/apps/usage/refresh").json(selection);
        usage_json(self.send_usage_request(request, principal_token).await?).await
    }

    /// Queue a refresh for the authorized selections on this project and host.
    pub async fn apps_usage_refresh_all(
        &self,
        project_id: &ProjectId,
        target_host_id: &str,
        principal_token: &str,
    ) -> Result<AppActionExecutionResult, AppsUsageWriteError> {
        let request = self
            .usage_request(Method::POST, "/v1/apps/usage/refresh-all")
            .json(&BatchScope { project_id, target_host_id });
        usage_json(self.send_usage_request(request, principal_token).await?).await
    }

    /// Read the current refresh status, including after reconnecting.
    pub async fn apps_usage_operation(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        self.usage_operation_request(Method::GET, target_host_id, id, false, principal_token).await
    }

    /// Long-poll for a status change or return the current status when the poll ends.
    pub async fn apps_usage_operation_events(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        self.usage_operation_request(Method::GET, target_host_id, id, true, principal_token).await
    }

    /// Cancel the specified refresh and return its terminal progress.
    pub async fn apps_usage_operation_cancel(
        &self,
        target_host_id: &str,
        id: &TaskId,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        self.usage_operation_request(Method::DELETE, target_host_id, id, false, principal_token)
            .await
    }

    /// Replace usage settings for this selection. Leave the query kind unset.
    pub async fn apps_usage_save_settings(
        &self,
        selection: &UsageSnapshotQuery,
        settings: &AppsUsageSettings,
        principal_token: &str,
    ) -> Result<(), AppsUsageWriteError> {
        let request = self
            .usage_request(Method::PUT, "/v1/apps/usage/settings")
            .json(&SaveSettings { selection, settings });
        let response = self.send_usage_request(request, principal_token).await?;
        if response.status() != StatusCode::NO_CONTENT {
            return Err(AppsUsageWriteError::Decode);
        }
        Ok(())
    }

    /// Run a transient script trial on the selected host.
    pub async fn apps_usage_script_test(
        &self,
        trial: &AppsUsageScriptTrial,
        principal_token: &str,
    ) -> Result<Vec<UsageScriptPackage>, AppsUsageWriteError> {
        let request = self.usage_request(Method::POST, "/v1/apps/usage/script-test").json(trial);
        usage_json(self.send_usage_request(request, principal_token).await?).await
    }

    async fn usage_operation_request(
        &self,
        method: Method,
        target_host_id: &str,
        id: &TaskId,
        events: bool,
        principal_token: &str,
    ) -> Result<AppsUsageOperation, AppsUsageWriteError> {
        // Dot segments would be normalized by the URL parser.
        if id.0.trim().is_empty() || matches!(id.0.as_str(), "." | "..") {
            return Err(AppsUsageWriteError::InvalidOperationId);
        }
        let path = format!("/v1/agent-apps/{}", operation_path(id, events));
        let request = self.usage_request(method, &path).query(&[("targetHostId", target_host_id)]);
        let progress: AppsUsageOperation =
            usage_json(self.send_usage_request(request, principal_token).await?).await?;
        if progress.operation.operation_id != *id || progress.target_host_id != target_host_id {
            return Err(AppsUsageWriteError::Decode);
        }
        Ok(progress)
    }

    fn usage_request(&self, method: Method, path: &str) -> RequestBuilder {
        self.dynamic_host_client
            .request(method, format!("{}{path}", self.endpoint.base_url()))
            .header(CACHE_CONTROL, "no-cache, no-store")
    }

    async fn send_usage_request(
        &self,
        request: RequestBuilder,
        principal_token: &str,
    ) -> Result<Response, AppsUsageWriteError> {
        let mut token = HeaderValue::from_str(principal_token)
            .map_err(|_| AppsUsageWriteError::InvalidPrincipalToken)?;
        token.set_sensitive(true);
        let request = request.header("x-agent-apps-token", token);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| AppsUsageWriteError::Transport)?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let status = status.as_u16();
        let bytes = response.bytes().await.map_err(|_| AppsUsageWriteError::Transport)?;
        if let Ok(error) = serde_json::from_slice::<UsageQueryError>(&bytes) {
            return Err(AppsUsageWriteError::ScriptTrial {
                status,
                code: error.code,
                correlation_id: error.correlation_id,
            });
        }
        Err(AppsUsageWriteError::Http { status, error: serde_json::from_slice(&bytes).ok() })
    }
}

async fn usage_json<T: DeserializeOwned>(response: Response) -> Result<T, AppsUsageWriteError> {
    response.json().await.map_err(|error| {
        if error.is_decode() { AppsUsageWriteError::Decode } else { AppsUsageWriteError::Transport }
    })
}
