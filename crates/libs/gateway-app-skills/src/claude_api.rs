use crate::GatewayAppSkillsClient;
use vw_api_app_skills::claude_api::*;

impl GatewayAppSkillsClient {
    pub async fn claude_config_status(&self, scope: &ClaudeConfigQuery, token: &str)
        -> Result<ClaudeConfigResponse, String> {
        let request = self.transport.native_transport().map_err(|_| "Claude transport unavailable")?
            .get(format!("{}/v1/agent-apps/claude/config-status", self.transport.endpoint().base_url()))
            .query(scope);
        claude_response(gateway_http::apply_request_context(request, self.transport.endpoint())).await
    }

    pub async fn claude_config_marker(&self, request: &ClaudeConfigRequest, token: &str)
        -> Result<ClaudeConfigChangeResponse, String> {
        self.claude_control("config-marker", request, token).await
    }

    pub async fn claude_onboarding(&self, request: &ClaudeConfigRequest, token: &str)
        -> Result<ClaudeConfigChangeResponse, String> {
        self.claude_control("onboarding", request, token).await
    }

    async fn claude_control(&self, path: &str, body: &ClaudeConfigRequest, token: &str)
        -> Result<ClaudeConfigChangeResponse, String> {
        let request = self.transport.native_transport().map_err(|_| "Claude transport unavailable")?
            .post(format!("{}/v1/agent-apps/claude/{path}", self.transport.endpoint().base_url()))
            .json(body);
        claude_response(gateway_http::apply_request_context(request, self.transport.endpoint())).await
    }
}
async fn claude_response<T: serde::de::DeserializeOwned>(request: reqwest::RequestBuilder)
    -> Result<T, String> {
    let response = request.send().await.map_err(|_| "Claude control transport failed")?;
    if !response.status().is_success() {
        return Err(format!("Claude control HTTP {}", response.status().as_u16()));
    }
    response.json().await.map_err(|_| "invalid Claude control response".into())
}
