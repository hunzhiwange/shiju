//! Prompt catalog and native selection API. Each call owns its complete scope.
use super::GatewayClient;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vw_api_types::{
    ProjectId,
    agent_apps::{AppId, PromptResource, PromptTargetPreview},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptsScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub app_id: AppId,
}

/// Catalog entries differ from the older generic PromptEntry contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptEntry {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptCatalog {
    pub revision: u64,
    #[serde(default)]
    pub bootstrap_complete: bool,
    pub active_id: Option<String>,
    pub entries: Vec<AgentAppPromptEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAppPromptLocation {
    pub resource: PromptResource,
    pub display_path: Option<String>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentAppPromptImportSource {
    UploadedContent { name: String, content: String, description: Option<String> },
    CurrentFile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentAppPromptCopies {
    pub results: Vec<AgentAppPromptCopy>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AgentAppPromptCopy {
    pub app_id: AppId,
    #[serde(flatten)]
    pub outcome: AgentAppPromptCopyOutcome,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AgentAppPromptCopyOutcome {
    Copied { entry: AgentAppPromptEntry },
    Failed { error: AgentAppPromptCopyFailure },
}
#[derive(Debug, Clone, Deserialize)]
pub struct AgentAppPromptCopyFailure {
    pub code: String,
    pub status: u16,
}

/// Keep business errors machine-readable without displaying prompt bodies in logs.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppPromptsError {
    InvalidInput,
    Transport,
    Http { status: u16, body: serde_json::Value },
    Decode,
}
impl std::fmt::Display for AgentAppPromptsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid prompt request"),
            Self::Transport => f.write_str("prompt gateway unavailable"),
            Self::Http { status, .. } => write!(f, "prompt gateway HTTP {status}"),
            Self::Decode => f.write_str("invalid prompt gateway response"),
        }
    }
}
impl std::error::Error for AgentAppPromptsError {}

#[derive(Serialize)]
struct WriteRequest<'a, O> {
    project_id: &'a ProjectId,
    target_host_id: &'a str,
    app_id: AppId,
    operation: O,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Operation<'a> {
    Save {
        revision: u64,
        id: Option<&'a str>,
        name: &'a str,
        content: &'a str,
        description: Option<&'a str>,
    },
    Delete {
        revision: u64,
        id: &'a str,
    },
    Copy {
        id: &'a str,
        targets: &'a [AppId],
    },
    Import {
        revision: u64,
        source: &'a AgentAppPromptImportSource,
    },
}

impl GatewayClient {
    pub async fn agent_app_prompts(
        &self,
        scope: &AgentAppPromptsScope,
        query: &str,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_read("", scope, Some(query), token).await
    }
    pub async fn agent_app_prompts_current(
        &self,
        scope: &AgentAppPromptsScope,
        token: &str,
    ) -> Result<PromptTargetPreview, AgentAppPromptsError> {
        self.prompt_read("/current", scope, None, token).await
    }
    pub async fn agent_app_prompts_location(
        &self,
        scope: &AgentAppPromptsScope,
        token: &str,
    ) -> Result<AgentAppPromptLocation, AgentAppPromptsError> {
        self.prompt_read("/location", scope, None, token).await
    }
    pub async fn agent_app_prompts_save(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        entry: &AgentAppPromptEntry,
        updating: bool,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_write(
            "/save",
            scope,
            Operation::Save {
                revision,
                id: updating.then_some(entry.id.as_str()),
                name: &entry.name,
                content: &entry.content,
                description: entry.description.as_deref(),
            },
            token,
        )
        .await
    }
    pub async fn agent_app_prompts_delete(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_write("/delete", scope, Operation::Delete { revision, id }, token).await
    }
    pub async fn agent_app_prompts_copy(
        &self,
        scope: &AgentAppPromptsScope,
        id: &str,
        targets: &[AppId],
        token: &str,
    ) -> Result<AgentAppPromptCopies, AgentAppPromptsError> {
        self.prompt_write("/copy", scope, Operation::Copy { id, targets }, token).await
    }
    pub async fn agent_app_prompts_import(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        source: &AgentAppPromptImportSource,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_write("/import", scope, Operation::Import { revision, source }, token).await
    }
    /// Use the exact catalog revision and native content from this scope's preview.
    /// None denotes an absent file; Some("") denotes an existing empty file.
    pub async fn agent_app_prompts_activate(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        expected_content: Option<&str>,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_selection("/activate", scope, revision, id, expected_content, token).await
    }
    pub async fn agent_app_prompts_deactivate(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        expected_content: Option<&str>,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        self.prompt_selection("/deactivate", scope, revision, id, expected_content, token).await
    }
    async fn prompt_selection(
        &self,
        path: &str,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        expected_content: Option<&str>,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        let body = serde_json::json!({"project_id": scope.project_id, "target_host_id": scope.target_host_id, "app_id": scope.app_id, "id": id, "revision": revision, "expected_content": expected_content});
        self.prompt_request(path, scope, None, Some(body), token).await
    }
    async fn prompt_write<T: DeserializeOwned>(
        &self,
        path: &str,
        scope: &AgentAppPromptsScope,
        operation: Operation<'_>,
        token: &str,
    ) -> Result<T, AgentAppPromptsError> {
        let body = serde_json::to_value(WriteRequest {
            project_id: &scope.project_id,
            target_host_id: &scope.target_host_id,
            app_id: scope.app_id,
            operation,
        })
        .map_err(|_| AgentAppPromptsError::InvalidInput)?;
        self.prompt_request(path, scope, None, Some(body), token).await
    }
    async fn prompt_read<T: DeserializeOwned>(
        &self,
        path: &str,
        scope: &AgentAppPromptsScope,
        query: Option<&str>,
        token: &str,
    ) -> Result<T, AgentAppPromptsError> {
        self.prompt_request(path, scope, query, None, token).await
    }
    async fn prompt_request<T: DeserializeOwned>(
        &self,
        path: &str,
        scope: &AgentAppPromptsScope,
        query: Option<&str>,
        body: Option<serde_json::Value>,
        token: &str,
    ) -> Result<T, AgentAppPromptsError> {
        use AgentAppPromptsError as E;
        if scope.target_host_id.trim().is_empty() || token.trim().is_empty() {
            return Err(E::InvalidInput);
        }
        let transport = self.native_transport().map_err(|_| E::Transport)?;
        let mut credential =
            reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        credential.set_sensitive(true);
        let mut request = transport
            .request(
                if body.is_some() { reqwest::Method::POST } else { reqwest::Method::GET },
                format!("{}/v1/agent-apps/prompts{path}", self.endpoint.base_url()),
            )
            .header("x-agent-apps-token", credential);
        if let Some(body) = body {
            request = request.json(&body);
        } else {
            request = request.query(scope);
            if let Some(query) = query {
                request = request.query(&[("query", query)]);
            }
        }
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| E::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| E::Transport)?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
            });
            return Err(E::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|_| E::Decode)
    }
}
