//! Fixed Claude Desktop operations with sensitive headers and payload-free errors.
use crate::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_provider_profiles::claude_desktop::*;
impl GatewayAppProviderClient {
    pub async fn claude_desktop_provider_import_live(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        self.claude_desktop_request("live", "POST", "/import", body, token).await
    }
    pub async fn claude_desktop_provider_save(
        &self,
        id: &str,
        body: &ClaudeDesktopWrite,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        self.claude_desktop_request(id, "PUT", "/config", body, token).await
    }
    pub async fn claude_desktop_provider_switch(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        self.claude_desktop_request(id, "POST", "/switch", body, token).await
    }
    pub async fn claude_desktop_provider_reapply(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        self.claude_desktop_request(id, "POST", "/reapply", body, token).await
    }
    pub(super) async fn claude_desktop_request<
        B: serde::Serialize,
        R: serde::de::DeserializeOwned,
    >(
        &self,
        id: &str,
        method: &str,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<R, Error> {
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, action, body, token);
            Err(Error::Transport("ClaudeDesktop requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let prefix = "/v1/app-providers/claude-desktop";
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid method".into()))?;
            let request = self.transport.dynamic_host_client()
                .request(method, format!("{}{prefix}/{id}{action}", self.transport.endpoint().base_url()))

                .json(body);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("ClaudeDesktop request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid ClaudeDesktop response".into()))
        }
    }
}
