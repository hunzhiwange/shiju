//! Compatibility forwarding for the prompt catalog client.
use super::GatewayClient;
pub use gateway_app_prompts::*;
use vw_api_app_identity::AppId;
use vw_api_app_operations::PromptTargetPreview;

impl GatewayClient {
    /// A successful HTTP response may retain a disabled import after activation failed.
    pub async fn agent_app_prompts_import_share(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        import: &vw_api_app_operations::ImportPromptShareRequest,
        token: &str,
    ) -> Result<vw_api_app_operations::ImportPromptShareResult, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_import_share(scope, revision, import, token).await
    }
    pub async fn agent_app_prompts(
        &self,
        scope: &AgentAppPromptsScope,
        query: &str,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts(scope, query, token).await
    }
    pub async fn agent_app_prompts_current(
        &self,
        scope: &AgentAppPromptsScope,
        token: &str,
    ) -> Result<PromptTargetPreview, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_current(scope, token).await
    }
    pub async fn agent_app_prompts_location(
        &self,
        scope: &AgentAppPromptsScope,
        token: &str,
    ) -> Result<AgentAppPromptLocation, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_location(scope, token).await
    }
    pub async fn agent_app_prompts_save(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        entry: &AgentAppPromptEntry,
        updating: bool,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_save(scope, revision, entry, updating, token).await
    }
    pub async fn agent_app_prompts_delete(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_delete(scope, revision, id, token).await
    }
    pub async fn agent_app_prompts_copy(
        &self,
        scope: &AgentAppPromptsScope,
        id: &str,
        targets: &[AppId],
        token: &str,
    ) -> Result<AgentAppPromptCopies, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_copy(scope, id, targets, token).await
    }
    pub async fn agent_app_prompts_import(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        source: &AgentAppPromptImportSource,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_import(scope, revision, source, token).await
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
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_activate(scope, revision, id, expected_content, token).await
    }
    pub async fn agent_app_prompts_deactivate(
        &self,
        scope: &AgentAppPromptsScope,
        revision: u64,
        id: &str,
        expected_content: Option<&str>,
        token: &str,
    ) -> Result<AgentAppPromptCatalog, AgentAppPromptsError> {
        gateway_app_prompts::GatewayAppPromptsClient::new(self.transport.clone())
            .agent_app_prompts_deactivate(scope, revision, id, expected_content, token).await
    }
}
