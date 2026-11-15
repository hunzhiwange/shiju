use crate::{AgentAppsReadError, GatewayExternalSessionClient};
use vw_api_external_session::{SessionContentRequest, SessionContentResponse};

impl GatewayExternalSessionClient {
    /// Read one Unicode character page of an authorized transcript's text reference.
    pub async fn external_session_content(
        &self,
        request: &SessionContentRequest,
        token: &str,
    ) -> Result<SessionContentResponse, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport("authorized history reads require native transport".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| AgentAppsReadError::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let pending = self.transport.dynamic_host_client()
                .post(format!("{}/v1/external-sessions/content", self.transport.endpoint().base_url()))
                .header("x-agent-apps-token", credential).json(request);
            let response = gateway_http::apply_request_context(pending, self.transport.endpoint()).send().await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            let status = response.status();
            let bytes = response.bytes().await.map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                return Err(AgentAppsReadError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes).map_err(|e| AgentAppsReadError::Decode(e.to_string()))
        }
    }
}
