//! Native Pi resources and saved-state synchronization. Content is never expanded locally.
use super::{GatewayClient, AgentAppsLifecycleError, AgentAppPromptsError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vw_api_types::{ProjectId, agent_apps::{AppId, AppOperationStatus}, id::TaskId};

#[derive(Debug, Clone, Serialize)]
pub struct PiPromptResourcesScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiPromptFileKind { SystemOverride, SystemAppend }
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiPromptFileSnapshot {
    pub exists: bool,
    pub revision: String,
    pub content: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PiPromptTemplate {
    pub slug: String,
    pub content: String,
    pub revision: String,
    pub summary: PiPromptTemplateSummary,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PiPromptTemplateSummary {
    pub description: Option<String>,
    pub argument_hint: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PiPromptTemplateDeletion { pub deleted: bool }
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptSync {
    pub operation: AppOperationStatus,
    pub project_id: ProjectId,
    pub target_host_id: String,
    /// While running, only processed targets are present; an empty list is not success.
    pub targets: Vec<AgentAppPromptSyncTarget>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptSyncTarget {
    pub app_id: AppId,
    pub status: AgentAppPromptSyncTargetStatus,
    pub reason: Option<String>,
    pub duplicate_selection: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAppPromptSyncTargetStatus { Succeeded, Failed, Skipped, NotExecuted }

impl GatewayClient {
    pub async fn pi_prompt_file_read(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind, token: &str)
        -> Result<PiPromptFileSnapshot, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-files/read", scope, serde_json::json!({"kind": kind}), token).await
    }
    pub async fn pi_prompt_file_replace(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind,
        revision: &str, content: &str, token: &str) -> Result<PiPromptFileSnapshot, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-files/replace", scope,
            serde_json::json!({"kind": kind, "expected_revision": revision, "content": content}), token).await
    }
    pub async fn pi_prompt_file_delete(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind,
        revision: &str, token: &str) -> Result<bool, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-files/delete", scope,
            serde_json::json!({"kind": kind, "expected_revision": revision}), token).await
    }
    pub async fn pi_prompt_templates(&self, scope: &PiPromptResourcesScope, token: &str)
        -> Result<Vec<PiPromptTemplate>, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-templates/list", scope, serde_json::json!({}), token).await
    }
    /// Preserve original slug and revision, including the server's `missing` creation token.
    pub async fn pi_prompt_template_upsert(&self, scope: &PiPromptResourcesScope, slug: &str,
        original_slug: Option<&str>, revision: &str, content: &str, token: &str)
        -> Result<PiPromptTemplate, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-templates/upsert", scope,
            serde_json::json!({"slug": slug, "original_slug": original_slug,
                "expected_revision": revision, "content": content}), token).await
    }
    pub async fn pi_prompt_template_delete(&self, scope: &PiPromptResourcesScope, slug: &str,
        revision: &str, token: &str) -> Result<PiPromptTemplateDeletion, AgentAppPromptsError> {
        self.prompt_resource_post("pi/prompt-templates/delete", scope,
            serde_json::json!({"slug": slug, "expected_revision": revision}), token).await
    }
    /// Submit once. After a disconnect, query the returned operation instead of replaying writes.
    pub async fn agent_app_prompts_sync(&self, scope: &PiPromptResourcesScope, apps: &[AppId], token: &str)
        -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        self.prompt_resources_request(reqwest::Method::POST, "prompts/sync", &scope.target_host_id,
            Some(serde_json::json!({"projectId": scope.project_id, "targetHostId": scope.target_host_id, "apps": apps})), token).await
    }
    pub async fn agent_app_prompts_sync_operation(&self, host: &str, id: &TaskId, token: &str)
        -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        self.prompt_resources_request(reqwest::Method::GET,
            &super::agent_apps_lifecycle_api::operation_path(id, false), host, None, token).await
    }
    pub async fn agent_app_prompts_sync_events(&self, host: &str, id: &TaskId, token: &str)
        -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        self.prompt_resources_request(reqwest::Method::GET,
            &super::agent_apps_lifecycle_api::operation_path(id, true), host, None, token).await
    }
    /// Cancellation preserves completed targets and the server's terminal cancellation state.
    pub async fn agent_app_prompts_sync_cancel(&self, host: &str, id: &TaskId, token: &str)
        -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        self.prompt_resources_request(reqwest::Method::DELETE,
            &super::agent_apps_lifecycle_api::operation_path(id, false), host, None, token).await
    }
    async fn prompt_resource_post<T: DeserializeOwned>(&self, path: &str, scope: &PiPromptResourcesScope,
        mut body: serde_json::Value, token: &str) -> Result<T, AgentAppPromptsError> {
        body["project_id"] = serde_json::to_value(&scope.project_id).map_err(|_| AgentAppPromptsError::InvalidInput)?;
        body["target_host_id"] = serde_json::Value::String(scope.target_host_id.clone());
        self.prompt_resources_request(reqwest::Method::POST, path, &scope.target_host_id, Some(body), token).await
    }
    async fn prompt_resources_request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &str,
        host: &str, body: Option<serde_json::Value>, token: &str) -> Result<T, AgentAppPromptsError> {
        if host.trim().is_empty() || token.trim().is_empty() {
            return Err(AgentAppPromptsError::InvalidInput);
        }
        self.agent_apps_lifecycle_request(method, path, host, body.as_ref(), Some(token)).await.map_err(|error| match error {
            AgentAppsLifecycleError::Http { status, body } => AgentAppPromptsError::Http { status, body },
            AgentAppsLifecycleError::Transport(_) => AgentAppPromptsError::Transport,
            AgentAppsLifecycleError::Decode(_) => AgentAppPromptsError::Decode,
        })
    }
}
