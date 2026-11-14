use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_types::external_session::{DeleteSessionOutcome, DeleteSessionRequest};

impl GatewayClient {
    /// Delete an explicitly selected session with independent deletion authority.
    pub async fn external_session_delete(
        &self,
        request: &DeleteSessionRequest,
        token: &str,
    ) -> Result<DeleteSessionOutcome, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history deletion requires native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsReadError::Transport("invalid application credential".into())
            })?;
            credential.set_sensitive(true);
            let pending = self
                .dynamic_host_client
                .post(format!("{}/v1/external-sessions/delete", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .json(request);
            let response = crate::http::apply_request_context(pending, &self.endpoint)
                .send()
                .await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            let status = response.status();
            let bytes =
                response.bytes().await.map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                return Err(AgentAppsReadError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes).map_err(|e| AgentAppsReadError::Decode(e.to_string()))
        }
    }
    /// Delete selected sessions and preserve their ordered individual outcomes.
    pub async fn external_sessions_delete(
        &self,
        request: &[DeleteSessionRequest],
        token: &str,
    ) -> Result<Vec<DeleteSessionOutcome>, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history deletion requires native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsReadError::Transport("invalid application credential".into())
            })?;
            credential.set_sensitive(true);
            let pending = self
                .dynamic_host_client
                .post(format!("{}/v1/external-sessions/delete-batch", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .json(request);
            let response = crate::http::apply_request_context(pending, &self.endpoint)
                .send()
                .await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            let status = response.status();
            let bytes =
                response.bytes().await.map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                return Err(AgentAppsReadError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes).map_err(|e| AgentAppsReadError::Decode(e.to_string()))
        }
    }
}
