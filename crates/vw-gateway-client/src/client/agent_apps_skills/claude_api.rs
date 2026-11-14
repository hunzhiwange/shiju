use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::claude_api::*;

impl GatewayClient {
    pub async fn claude_config_status(&self, scope: &ClaudeConfigQuery, token: &str)
        -> Result<ClaudeConfigResponse, String> {
        let request = self.native_transport().map_err(|_| "Claude transport unavailable")?
            .get(format!("{}/v1/agent-apps/claude/config-status", self.endpoint.base_url()))
            .query(scope).header("x-agent-apps-token", claude_token(token)?);
        claude_response(crate::http::apply_request_context(request, &self.endpoint)).await
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
        let request = self.native_transport().map_err(|_| "Claude transport unavailable")?
            .post(format!("{}/v1/agent-apps/claude/{path}", self.endpoint.base_url()))
            .json(body).header("x-agent-apps-token", claude_token(token)?);
        claude_response(crate::http::apply_request_context(request, &self.endpoint)).await
    }
}
fn claude_token(token: &str) -> Result<reqwest::header::HeaderValue, String> {
    let mut value = reqwest::header::HeaderValue::from_str(token)
        .map_err(|_| "invalid Claude control credential")?;
    value.set_sensitive(true);
    Ok(value)
}
async fn claude_response<T: serde::de::DeserializeOwned>(request: reqwest::RequestBuilder)
    -> Result<T, String> {
    let response = request.send().await.map_err(|_| "Claude control transport failed")?;
    if !response.status().is_success() {
        return Err(format!("Claude control HTTP {}", response.status().as_u16()));
    }
    response.json().await.map_err(|_| "invalid Claude control response".into())
}
