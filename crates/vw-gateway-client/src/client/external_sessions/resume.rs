use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_types::external_session::{SessionResumeResponse, SessionResumeRequest};

impl GatewayClient {
    /// Resume on the authorized execution host; completes after the native process exits.
    pub async fn external_session_resume(
        &self,
        request: &SessionResumeRequest,
        token: &str,
    ) -> Result<SessionResumeResponse, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history resume requires native transport".into(),
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
                .post(format!("{}/v1/external-sessions/resume", self.endpoint.base_url()))
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
    /// Preview a credential-free command without launching a terminal.
    pub async fn external_session_resume_command(
        &self,
        request: &SessionResumeRequest,
        token: &str,
    ) -> Result<vw_api_types::external_session::SessionResumeCommandResponse, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport(
                "authorized history resume requires native transport".into(),
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
                .post(format!("{}/v1/external-sessions/resume/command", self.endpoint.base_url()))
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
