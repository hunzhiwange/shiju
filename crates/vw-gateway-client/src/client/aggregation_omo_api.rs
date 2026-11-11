//! OMO and Slim keep separate resources and response types. Writes are never replayed.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::agent_apps::aggregation::{
    OmoApplyRequest, OmoDisableRequest, OmoSlimState, OmoState,
};
use vw_api_types::id::ProjectId;

/// Preserve gateway status and structured errors instead of returning an empty catalog.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationOmoError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AggregationOmoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, .. } => write!(f, "OMO HTTP {status}"),
        }
    }
}
impl std::error::Error for AggregationOmoError {}

impl GatewayClient {
    /// Read the selected variant, including its current provider and opaque revision.
    pub async fn aggregation_omo_state(
        &self,
        host: &str,
        project: &ProjectId,
    ) -> Result<OmoState, AggregationOmoError> {
        self.aggregation_omo_request::<(), _>(
            reqwest::Method::GET,
            "omo",
            Some((host, project)),
            None,
        )
        .await
    }
    /// Apply a saved provider at the supplied revision; conflicts remain explicit.
    pub async fn aggregation_omo_apply(
        &self,
        request: &OmoApplyRequest,
    ) -> Result<OmoState, AggregationOmoError> {
        self.aggregation_omo_request(reqwest::Method::POST, "omo/apply", None, Some(request)).await
    }
    /// Disable only this variant at the supplied revision.
    pub async fn aggregation_omo_disable(
        &self,
        request: &OmoDisableRequest,
    ) -> Result<OmoState, AggregationOmoError> {
        self.aggregation_omo_request(reqwest::Method::POST, "omo/disable", None, Some(request))
            .await
    }
    /// Read the selected variant, including its current provider and opaque revision.
    pub async fn aggregation_omo_slim_state(
        &self,
        host: &str,
        project: &ProjectId,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        self.aggregation_omo_request::<(), _>(
            reqwest::Method::GET,
            "omo-slim",
            Some((host, project)),
            None,
        )
        .await
    }
    /// Apply a saved provider at the supplied revision; conflicts remain explicit.
    pub async fn aggregation_omo_slim_apply(
        &self,
        request: &OmoApplyRequest,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        self.aggregation_omo_request(reqwest::Method::POST, "omo-slim/apply", None, Some(request))
            .await
    }
    /// Disable only this variant at the supplied revision.
    pub async fn aggregation_omo_slim_disable(
        &self,
        request: &OmoDisableRequest,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        self.aggregation_omo_request(reqwest::Method::POST, "omo-slim/disable", None, Some(request))
            .await
    }
    async fn aggregation_omo_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        scope: Option<(&str, &ProjectId)>,
        body: Option<&B>,
    ) -> Result<T, AggregationOmoError> {
        let mut request = self.dynamic_host_client.request(
            method,
            format!("{}/v1/agent-apps/opencode/{resource}", self.endpoint.base_url()),
        );
        if let Some((host, project)) = scope {
            request = request.query(&[("targetHostId", host), ("projectId", project.as_ref())]);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| AggregationOmoError::Transport("OMO request failed".into()))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| AggregationOmoError::Transport("OMO response read failed".into()))?;
        if status != reqwest::StatusCode::OK {
            return Err(AggregationOmoError::Http {
                status: status.as_u16(),
                body: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
            });
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| AggregationOmoError::Decode("Invalid OMO response".into()))
    }
}
