//! Scoped aggregation requests. Membership rules remain gateway-owned.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::agent_apps::{AppId, aggregation::*};

/// Preserve gateway status and structured errors without replaying writes.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationApiError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AggregationApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, .. } => write!(f, "aggregation HTTP {status}"),
        }
    }
}
impl std::error::Error for AggregationApiError {}

impl GatewayClient {
    /// Read the exact host/project snapshot using the endpoint's bearer identity.
    pub async fn aggregation_state(
        &self,
        app: AppId,
        scope: &AggregationQuery,
    ) -> Result<AggregationState, AggregationApiError> {
        let url = self.aggregation_url(app, &["state"])?;
        let request =
            self.native_transport().map_err(AggregationApiError::Transport)?.get(url).query(scope);
        self.aggregation_response(request).await
    }
    /// Preview intent without publishing or recalculating membership locally.
    pub async fn aggregation_preview(
        &self,
        app: AppId,
        body: &AggregationPreviewRequest,
    ) -> Result<AggregationPreview, AggregationApiError> {
        self.aggregation_write(reqwest::Method::POST, app, &["preview"], body).await
    }
    /// Set a member target once, at the caller's observed revision.
    pub async fn aggregation_member_set(
        &self,
        app: AppId,
        provider_id: &str,
        body: &AggregationMemberRequest,
    ) -> Result<AggregationState, AggregationApiError> {
        self.aggregation_write(reqwest::Method::PUT, app, &["members", provider_id], body).await
    }
    /// Select the default once. The gateway owns eligibility and automatic membership.
    pub async fn aggregation_default_set(
        &self,
        app: AppId,
        body: &AggregationDefaultRequest,
    ) -> Result<AggregationState, AggregationApiError> {
        self.aggregation_write(reqwest::Method::PUT, app, &["default"], body).await
    }
    fn aggregation_url(
        &self,
        app: AppId,
        resource: &[&str],
    ) -> Result<reqwest::Url, AggregationApiError> {
        let app =
            serde_json::to_value(app).map_err(|e| AggregationApiError::Transport(e.to_string()))?;
        let mut url = reqwest::Url::parse(&self.endpoint.base_url())
            .map_err(|e| AggregationApiError::Transport(e.to_string()))?;
        url.path_segments_mut()
            .map_err(|_| AggregationApiError::Transport("invalid gateway URL".into()))?
            .pop_if_empty()
            .extend(["v1", "agent-apps", app.as_str().expect("AppId string"), "aggregation"])
            .extend(resource.iter().copied());
        Ok(url)
    }
    pub(super) async fn aggregation_write<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        app: AppId,
        resource: &[&str],
        body: &B,
    ) -> Result<T, AggregationApiError> {
        let url = self.aggregation_url(app, resource)?;
        self.aggregation_response(
            self.native_transport()
                .map_err(AggregationApiError::Transport)?
                .request(method, url)
                .json(body),
        )
        .await
    }
    async fn aggregation_response<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, AggregationApiError> {
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|e| AggregationApiError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes =
            response.bytes().await.map_err(|e| AggregationApiError::Transport(e.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
            });
            return Err(AggregationApiError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|e| AggregationApiError::Decode(e.to_string()))
    }
}
