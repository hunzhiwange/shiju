//! Typed application routing requests over the existing authenticated gateway transport.
use crate::GatewayClient;
use vw_api_types::{
    app_routing::{RoutingHealth, RoutingStatus},
    error::ApiErrorBody,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AppRoutingError {
    Transport,
    InvalidRequest,
    Decode,
    Http { status: u16, body: Option<ApiErrorBody> },
}

impl std::fmt::Display for AppRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("application routing transport failed"),
            Self::InvalidRequest => f.write_str("invalid application routing request"),
            Self::Decode => f.write_str("malformed application routing response"),
            Self::Http { status, .. } => write!(f, "application routing HTTP {status}"),
        }
    }
}

impl std::error::Error for AppRoutingError {}

impl GatewayClient {
    pub async fn app_routing_status(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        self.app_routing_request(reqwest::Method::GET, host, "status", token).await
    }

    pub async fn app_routing_health(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingHealth, AppRoutingError> {
        self.app_routing_request(reqwest::Method::GET, host, "health", token).await
    }

    pub async fn app_routing_start(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        self.app_routing_request(reqwest::Method::POST, host, "start", token).await
    }

    pub async fn app_routing_stop(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        self.app_routing_request(reqwest::Method::POST, host, "stop", token).await
    }

    async fn app_routing_request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        host: &str,
        action: &str,
        token: &str,
    ) -> Result<T, AppRoutingError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, host, action, token);
            Err(AppRoutingError::Transport)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if matches!(host, "" | "." | "..") {
                return Err(AppRoutingError::InvalidRequest);
            }
            let mut url = reqwest::Url::parse(&self.endpoint.base_url())
                .map_err(|_| AppRoutingError::InvalidRequest)?;
            url.path_segments_mut()
                .map_err(|_| AppRoutingError::InvalidRequest)?
                .pop_if_empty()
                .extend(["v1", "app-routing", "hosts", host, action]);
            let mut token = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| AppRoutingError::InvalidRequest)?;
            token.set_sensitive(true);
            let write = method == reqwest::Method::POST;
            let mut request =
                self.dynamic_host_client.request(method, url).header("x-agent-apps-token", token);
            if write {
                request = request.json(&vw_api_types::app_routing::RoutingLifecycleRequest {});
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| AppRoutingError::Transport)?;
            let status = response.status();
            if !status.is_success() {
                let body = response.json::<ApiErrorBody>().await.ok();
                return Err(AppRoutingError::Http { status: status.as_u16(), body });
            }
            response.json().await.map_err(|error| {
                if error.is_decode() { AppRoutingError::Decode } else { AppRoutingError::Transport }
            })
        }
    }
}
