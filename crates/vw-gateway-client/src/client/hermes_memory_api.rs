//! Scoped Hermes memory access. Model configuration remains read-only.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{hermes_memory::*, id::ProjectId};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesMemoryScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
}

/// Do not include response bodies or transport details: they may contain memory text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HermesMemoryError {
    InvalidInput,
    Transport,
    Http(u16),
    Decode,
}
impl std::fmt::Display for HermesMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid Hermes request"),
            Self::Transport => f.write_str("Hermes gateway unavailable"),
            Self::Http(status) => write!(f, "Hermes gateway HTTP {status}"),
            Self::Decode => f.write_str("invalid Hermes gateway response"),
        }
    }
}
impl std::error::Error for HermesMemoryError {}

impl GatewayClient {
    pub async fn hermes_memory(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, token: &str) -> Result<HermesMemoryResponse, HermesMemoryError> {
        self.hermes_memory_request(reqwest::Method::GET, &memory_path(kind), scope, token, None).await
    }

    /// Only an acknowledged write returns an outcome; callers retain dirty drafts on error.
    pub async fn set_hermes_memory(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, content: String, token: &str) -> Result<HermesWriteOutcome, HermesMemoryError> {
        let body = serde_json::to_value(HermesMemoryWriteRequest { content }).map_err(|_| HermesMemoryError::InvalidInput)?;
        self.hermes_memory_request(reqwest::Method::PUT, &memory_path(kind), scope, token, Some(body)).await
    }

    pub async fn hermes_memory_limits(&self, scope: &HermesMemoryScope, token: &str) -> Result<HermesMemoryLimits, HermesMemoryError> {
        self.hermes_memory_request(reqwest::Method::GET, "memory-limits", scope, token, None).await
    }

    /// After success, refresh limits with `hermes_memory_limits` for the same scope.
    pub async fn set_hermes_memory_enabled(&self, scope: &HermesMemoryScope, kind: HermesMemoryKind, enabled: bool, token: &str) -> Result<HermesWriteOutcome, HermesMemoryError> {
        self.hermes_memory_request(reqwest::Method::PUT, &format!("{}/enabled", memory_path(kind)), scope, token, Some(serde_json::json!({"enabled": enabled}))).await
    }

    pub async fn hermes_model(&self, scope: &HermesMemoryScope, token: &str) -> Result<Option<HermesModelSnapshot>, HermesMemoryError> {
        self.hermes_memory_request(reqwest::Method::GET, "model", scope, token, None).await
    }

    async fn hermes_memory_request<T: DeserializeOwned>(&self, method: reqwest::Method, resource: &str, scope: &HermesMemoryScope, token: &str, body: Option<serde_json::Value>) -> Result<T, HermesMemoryError> {
        use HermesMemoryError as E;
        if scope.target_host_id.trim().is_empty() || token.trim().is_empty() {
            return Err(E::InvalidInput);
        }
        let transport = self.native_transport().map_err(|_| E::Transport)?;
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        credential.set_sensitive(true);
        let mut request = transport.request(method, format!("{}/v1/agent-apps/hermes/{resource}", self.endpoint.base_url()))
            .query(scope).header("x-agent-apps-token", credential);
        if let Some(body) = body { request = request.json(&body); }
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await.map_err(|_| E::Transport)?;
        if !response.status().is_success() { return Err(E::Http(response.status().as_u16())); }
        response.json().await.map_err(|_| E::Decode)
    }
}
fn memory_path(kind: HermesMemoryKind) -> String {
    format!("memory/{}", match kind { HermesMemoryKind::Memory => "memory", HermesMemoryKind::User => "user" })
}
