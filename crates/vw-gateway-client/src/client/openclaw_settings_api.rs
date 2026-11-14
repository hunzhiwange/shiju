//! Fixed OpenClaw settings. Provider snapshots and model choices retain their existing SDK owners.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{id::ProjectId, openclaw_settings::*};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawSettingsScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
}

/// Do not include response bodies or transport details: they may contain configuration secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenClawSettingsError {
    InvalidInput,
    Transport,
    Http(u16),
    Decode,
}
impl std::fmt::Display for OpenClawSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid OpenClaw request"),
            Self::Transport => f.write_str("OpenClaw gateway unavailable"),
            Self::Http(status) => write!(f, "OpenClaw gateway HTTP {status}"),
            Self::Decode => f.write_str("invalid OpenClaw gateway response"),
        }
    }
}
impl std::error::Error for OpenClawSettingsError {}

impl GatewayClient {
    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_model(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawDefaultModel>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "model", scope, token, None).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_model(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawDefaultModel,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        let body = serde_json::to_value(value).map_err(|_| OpenClawSettingsError::InvalidInput)?;
        self.openclaw_settings_request(reqwest::Method::PUT, "model", scope, token, Some(body))
            .await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_catalog(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawModelCatalog>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "catalog", scope, token, None).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_catalog(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawModelCatalog,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        let body = serde_json::to_value(value).map_err(|_| OpenClawSettingsError::InvalidInput)?;
        self.openclaw_settings_request(reqwest::Method::PUT, "catalog", scope, token, Some(body))
            .await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_defaults(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawAgentsDefaults>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "defaults", scope, token, None).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_defaults(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawAgentsDefaults,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        let body = serde_json::to_value(value).map_err(|_| OpenClawSettingsError::InvalidInput)?;
        self.openclaw_settings_request(reqwest::Method::PUT, "defaults", scope, token, Some(body))
            .await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_env(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawEnvConfig>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "env", scope, token, None).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_env(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawEnvConfig,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        let body = serde_json::to_value(value).map_err(|_| OpenClawSettingsError::InvalidInput)?;
        self.openclaw_settings_request(reqwest::Method::PUT, "env", scope, token, Some(body)).await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_tools(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawToolsConfig>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "tools", scope, token, None).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_tools(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawToolsConfig,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        let body = serde_json::to_value(value).map_err(|_| OpenClawSettingsError::InvalidInput)?;
        self.openclaw_settings_request(reqwest::Method::PUT, "tools", scope, token, Some(body))
            .await
    }

    pub async fn openclaw_settings_health(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Vec<OpenClawHealthWarning>, OpenClawSettingsError> {
        self.openclaw_settings_request(reqwest::Method::GET, "health", scope, token, None).await
    }

    async fn openclaw_settings_request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        scope: &OpenClawSettingsScope,
        token: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, OpenClawSettingsError> {
        use OpenClawSettingsError as E;
        if scope.target_host_id.trim().is_empty() || token.trim().is_empty() {
            return Err(E::InvalidInput);
        }
        let transport = self.native_transport().map_err(|_| E::Transport)?;
        let mut credential =
            reqwest::header::HeaderValue::from_str(token).map_err(|_| E::InvalidInput)?;
        credential.set_sensitive(true);
        let mut request = transport
            .request(
                method,
                format!("{}/v1/agent-apps/openclaw/settings/{resource}", self.endpoint.base_url()),
            )
            .query(scope)
            .header("x-agent-apps-token", credential);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| E::Transport)?;
        if !response.status().is_success() {
            return Err(E::Http(response.status().as_u16()));
        }
        response.json().await.map_err(|_| E::Decode)
    }
}
