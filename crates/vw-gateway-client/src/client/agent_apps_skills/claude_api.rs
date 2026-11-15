use super::super::GatewayClient;
use vw_api_app_skills::claude_api::*;

impl GatewayClient {
    pub async fn claude_config_status(&self, scope: &ClaudeConfigQuery, token: &str) -> Result<ClaudeConfigResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .claude_config_status(scope, token).await
    }
    pub async fn claude_config_marker(&self, request: &ClaudeConfigRequest, token: &str) -> Result<ClaudeConfigChangeResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .claude_config_marker(request, token).await
    }
    pub async fn claude_onboarding(&self, request: &ClaudeConfigRequest, token: &str) -> Result<ClaudeConfigChangeResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .claude_onboarding(request, token).await
    }
}
