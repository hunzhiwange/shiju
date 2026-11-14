use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_types::external_session::{SessionImageRequest, SessionImageResponse};

impl GatewayClient {
    /// Read authorized bitmap bytes on the selected host, without local filesystem fallback.
    pub async fn external_session_image(
        &self,
        request: &SessionImageRequest,
        token: &str,
    ) -> Result<SessionImageResponse, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (request, token);
            return Err(AgentAppsReadError::Transport("authorized history reads require native transport".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| AgentAppsReadError::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let pending = self.dynamic_host_client
                .post(format!("{}/v1/external-sessions/images", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential).json(request);
            let mut response = crate::http::apply_request_context(pending, &self.endpoint).send().await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
            let status = response.status();
            let media_type = if status.is_success() {
                match response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                    Some(mime @ ("image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp")) => mime.to_owned(),
                    _ => return Err(AgentAppsReadError::Decode("unsupported image media type".into())),
                }
            } else { String::new() };
            let limit = if status.is_success() { MAX_IMAGE_BYTES } else { 64 * 1024 };
            if response.content_length().is_some_and(|len| len > limit as u64) {
                return Err(AgentAppsReadError::Decode("image response exceeds size limit".into()));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))? {
                if chunk.len() > limit.saturating_sub(bytes.len()) {
                    return Err(AgentAppsReadError::Decode("image response exceeds size limit".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                return Err(AgentAppsReadError::Http { status: status.as_u16(), body });
            }
            Ok(SessionImageResponse { bytes, media_type })
        }
    }
}
