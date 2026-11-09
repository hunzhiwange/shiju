//! Read usage resources from the authorized execution host.

use super::GatewayClient;
use crate::http::apply_request_context;
use reqwest::header::{CACHE_CONTROL, HeaderValue};
use serde::{Deserialize, de::DeserializeOwned};
use vw_api_types::{
    agent_apps_auth::AuthError,
    apps_usage::{UsageSnapshotQuery, UsageSnapshotResponse},
};

/// Read failure with the HTTP status and the typed gateway error when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsUsageReadError {
    InvalidPrincipalToken,
    Transport,
    Http { status: u16, error: Option<AuthError> },
    Decode,
}

impl std::fmt::Display for AppsUsageReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrincipalToken => f.write_str("invalid application principal token"),
            Self::Transport => f.write_str("usage read request could not be completed"),
            Self::Http { status, error: Some(error) } => {
                write!(f, "usage read failed with HTTP {status} ({:?})", error.code)
            }
            Self::Http { status, error: None } => write!(f, "usage read failed with HTTP {status}"),
            Self::Decode => f.write_str("invalid usage read response"),
        }
    }
}

impl std::error::Error for AppsUsageReadError {}

/// Template metadata for the selected application. Native templates have no script source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AppsUsageTemplate {
    /// Stable identifier assigned by the gateway.
    pub template: String,
    /// Unexpanded preset source.
    pub source: Option<String>,
}

/// Saved query settings returned by the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppsUsageSettings {
    pub template: String,
    pub enabled: bool,
    /// Zero disables automatic refresh.
    pub refresh_interval_minutes: u16,
    #[serde(default)]
    pub display: AppsUsageDisplay,
    #[serde(default)]
    pub credentials: AppsUsageCredentialRefs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppsUsageDisplay {
    #[default]
    Left,
    Used,
}

/// Opaque references to credentials managed by the execution host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppsUsageCredentialRefs {
    pub api_key_ref: Option<String>,
    pub access_token_ref: Option<String>,
    pub signing: Option<AppsUsageSigningCredentialRefs>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppsUsageSigningCredentialRefs {
    pub access_key_id_ref: String,
    pub secret_access_key_ref: String,
}

impl GatewayClient {
    /// Read the current usage capabilities and cached snapshot for this selection.
    pub async fn apps_usage_snapshot(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<UsageSnapshotResponse, AppsUsageReadError> {
        self.apps_usage_read("/v1/apps/usage/snapshot", selection, principal_token).await
    }

    /// Read template metadata. Leave the query kind unset.
    pub async fn apps_usage_templates(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<Vec<AppsUsageTemplate>, AppsUsageReadError> {
        self.apps_usage_read("/v1/apps/usage/templates", selection, principal_token).await
    }

    /// Read saved settings, or `None` if this selection has no settings. Leave the query kind unset.
    pub async fn apps_usage_settings(
        &self,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<Option<AppsUsageSettings>, AppsUsageReadError> {
        self.apps_usage_read("/v1/apps/usage/settings", selection, principal_token).await
    }

    async fn apps_usage_read<T: DeserializeOwned>(
        &self,
        path: &str,
        selection: &UsageSnapshotQuery,
        principal_token: &str,
    ) -> Result<T, AppsUsageReadError> {
        let mut token = HeaderValue::from_str(principal_token)
            .map_err(|_| AppsUsageReadError::InvalidPrincipalToken)?;
        token.set_sensitive(true);
        let request = self
            .dynamic_host_client
            .get(format!("{}{path}", self.endpoint.base_url()))
            .query(selection)
            .header(CACHE_CONTROL, "no-cache, no-store")
            .header("x-agent-apps-token", token);

        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| AppsUsageReadError::Transport)?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppsUsageReadError::Http {
                status: status.as_u16(),
                error: response.json().await.ok(),
            });
        }
        response.json().await.map_err(|error| {
            if error.is_decode() {
                AppsUsageReadError::Decode
            } else {
                AppsUsageReadError::Transport
            }
        })
    }
}
