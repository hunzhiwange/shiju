//! Authorized native Codex compression settings and stored-byte queries.
use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_types::external_session::*;

impl GatewayClient {
    pub async fn external_session_compression(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<SessionCompressionState, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history maintenance requires native transport".into(),
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
                .post(format!(
                    "{}/v1/external-sessions/compression/query",
                    self.endpoint.base_url()
                ))
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
    pub async fn external_session_compression_set(
        &self,
        request: &SessionCompressionUpdate,
        token: &str,
    ) -> Result<SessionCompressionState, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history maintenance requires native transport".into(),
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
                .post(format!("{}/v1/external-sessions/compression", self.endpoint.base_url()))
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
