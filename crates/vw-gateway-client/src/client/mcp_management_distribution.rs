//! Typed wire contracts for the delivered MCP distribution routes.
//! Native paths, credential resolution and transactions remain on the execution host.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vw_api_types::{
    ProjectId,
    agent_apps::{AppActionExecutionResult, AppId, AppInstallationId},
    mcp_management::{
        McpAppTarget, McpDistributionResult, McpFailureCode, McpRecordId, McpRecordRef,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportSource {
    pub target: McpAppTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub sources: Vec<McpImportSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpImportResponse {
    pub sources: Vec<McpImportSourceResult>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpImportSourceResult {
    pub target: McpAppTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<McpImportEntryResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<McpFailureCode>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpImportEntryResult {
    pub id: Option<McpRecordId>,
    #[serde(flatten)]
    pub outcome: McpImportEntryOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum McpImportEntryOutcome {
    Added { record: McpRecordRef },
    Merged { record: McpRecordRef },
    Unchanged { record: McpRecordRef },
    Failed { code: McpFailureCode },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBindingRequest {
    pub target: McpAppTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub expected_revision: u64,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBulkRequest {
    pub target: McpAppTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub records: Vec<McpRecordRef>,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResyncTarget {
    pub app_id: AppId,
    pub installation_id: AppInstallationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResyncRequest {
    pub target_host_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub targets: Vec<McpResyncTarget>,
}

impl GatewayClient {
    /// Import definitions from trusted installations without writing their native files.
    pub async fn mcp_management_import(
        &self,
        request: &McpImportRequest,
        token: &str,
    ) -> Result<McpImportResponse, AgentAppsSettingsError> {
        self.mcp_distribution_request(reqwest::Method::POST, &["import"], request, token).await
    }

    /// Save binding intent and preserve each native transaction outcome.
    pub async fn mcp_management_bind(
        &self,
        id: &McpRecordId,
        request: &McpBindingRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        if id.0.is_empty() || matches!(id.0.as_str(), "." | "..") {
            return Err(AgentAppsSettingsError::Transport("invalid MCP record identity".into()));
        }
        self.mcp_distribution_request(reqwest::Method::PUT, &[&id.0, "bindings"], request, token)
            .await
    }

    /// Enable or disable the specified revisions on one installation.
    pub async fn mcp_management_bulk(
        &self,
        request: &McpBulkRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        self.mcp_distribution_request(reqwest::Method::POST, &["bulk"], request, token).await
    }

    /// Accept a reconciliation job. Queued is not native success.
    /// Recover/cancel with `agent_apps_operation_authorized` and
    /// `agent_apps_operation_cancel_authorized`, using the same host and token.
    pub async fn mcp_management_resync(
        &self,
        request: &McpResyncRequest,
        token: &str,
    ) -> Result<AppActionExecutionResult, AgentAppsSettingsError> {
        self.mcp_distribution_request(reqwest::Method::POST, &["resync"], request, token).await
    }

    async fn mcp_distribution_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        segments: &[&str],
        body: &B,
        token: &str,
    ) -> Result<T, AgentAppsSettingsError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, segments, body, token);
            Err(AgentAppsSettingsError::Transport(
                "MCP distribution requires native transport with redirects and retries disabled"
                    .into(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut url =
                reqwest::Url::parse(&format!("{}/v1/agent-apps/mcp", self.endpoint.base_url()))
                    .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            url.path_segments_mut()
                .map_err(|_| AgentAppsSettingsError::Transport("invalid gateway URL".into()))?
                .extend(segments);
            let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsSettingsError::Transport("invalid application credential".into())
            })?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .request(method, url)
                .header("x-agent-apps-token", credential)
                .json(body);
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
