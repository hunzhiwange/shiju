//! Native authenticated history settings and scoped backup queries.
use crate::{AgentAppsReadError, GatewayExternalSessionClient};
use vw_api_external_session::*;

impl GatewayExternalSessionClient {
    pub async fn external_history_options(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<HistoryOptions, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history maintenance requires native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pending = self.transport.dynamic_host_client()
                .post(format!(
                    "{}/v1/external-sessions/history-options/query",
                    self.transport.endpoint().base_url()
                ))

                .json(request);
            let response = gateway_http::apply_request_context(pending, self.transport.endpoint())
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
    pub async fn external_history_options_set(
        &self,
        request: &HistoryOptionsUpdate,
        token: &str,
    ) -> Result<HistoryOptions, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history maintenance requires native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pending = self.transport.dynamic_host_client()
                .post(format!("{}/v1/external-sessions/history-options", self.transport.endpoint().base_url()))

                .json(request);
            let response = gateway_http::apply_request_context(pending, self.transport.endpoint())
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
    pub async fn external_history_backups(
        &self,
        request: &HistoryMaintenanceTarget,
        token: &str,
    ) -> Result<HistoryBackups, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history maintenance requires native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pending = self.transport.dynamic_host_client()
                .post(format!(
                    "{}/v1/external-sessions/history-backups/query",
                    self.transport.endpoint().base_url()
                ))

                .json(request);
            let response = gateway_http::apply_request_context(pending, self.transport.endpoint())
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
