//! Compatibility forwarding for prompt resources.
use super::{GatewayClient, AgentAppPromptsError};
use vw_api_app_identity::AppId;
use vw_api_transport::id::TaskId;
pub use gateway_app_prompt_resources::*;

impl GatewayClient {
    pub async fn pi_prompt_file_read(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind, token: &str) -> Result<PiPromptFileSnapshot, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_file_read(scope, kind, token).await
    }
    pub async fn pi_prompt_file_replace(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind,
        revision: &str, content: &str, token: &str) -> Result<PiPromptFileSnapshot, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_file_replace(scope, kind, revision, content, token).await
    }
    pub async fn pi_prompt_file_delete(&self, scope: &PiPromptResourcesScope, kind: PiPromptFileKind,
        revision: &str, token: &str) -> Result<bool, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_file_delete(scope, kind, revision, token).await
    }
    pub async fn pi_prompt_templates(&self, scope: &PiPromptResourcesScope, token: &str) -> Result<Vec<PiPromptTemplate>, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_templates(scope, token).await
    }
    pub async fn pi_prompt_template_upsert(&self, scope: &PiPromptResourcesScope, slug: &str,
        original_slug: Option<&str>, revision: &str, content: &str, token: &str) -> Result<PiPromptTemplate, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_template_upsert(scope, slug, original_slug, revision, content, token).await
    }
    pub async fn pi_prompt_template_delete(&self, scope: &PiPromptResourcesScope, slug: &str,
        revision: &str, token: &str) -> Result<PiPromptTemplateDeletion, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .pi_prompt_template_delete(scope, slug, revision, token).await
    }
    pub async fn agent_app_prompts_sync(&self, scope: &PiPromptResourcesScope, apps: &[AppId], token: &str) -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .agent_app_prompts_sync(scope, apps, token).await
    }
    pub async fn agent_app_prompts_sync_operation(&self, host: &str, id: &TaskId, token: &str) -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .agent_app_prompts_sync_operation(host, id, token).await
    }
    pub async fn agent_app_prompts_sync_events(&self, host: &str, id: &TaskId, token: &str) -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .agent_app_prompts_sync_events(host, id, token).await
    }
    pub async fn agent_app_prompts_sync_cancel(&self, host: &str, id: &TaskId, token: &str) -> Result<AgentAppPromptSync, AgentAppPromptsError> {
        gateway_app_prompt_resources::GatewayAppPromptResourcesClient::new(self.transport.clone())
            .agent_app_prompts_sync_cancel(host, id, token).await
    }
}
