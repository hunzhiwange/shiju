//! Managed account queries on the Gateway management routes.

use super::GatewayClient;
use crate::http::apply_request_context;
use reqwest::header::{CACHE_CONTROL, HeaderValue};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use vw_api_types::agent_apps_auth::{AuthAccount, AuthError, AuthService, AuthStatus};

/// Basic management credentials and the independent trusted principal token.
pub struct AgentAppsAuthCredentials<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub principal_token: &'a str,
}

/// A supplier's account reference in the selected authorization scope.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAppsAuthBinding {
    pub provider_id: String,
    pub service: AuthService,
    pub account_id: String,
}

/// Query failure with the structured authorization error when the gateway supplies one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAppsAuthReadError {
    InvalidPrincipalToken,
    Transport,
    Http { status: u16, error: Option<AuthError> },
    Decode,
}

impl std::fmt::Display for AgentAppsAuthReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrincipalToken => f.write_str("invalid authorization principal token"),
            Self::Transport => f.write_str("authorization request could not be completed"),
            Self::Http { status, error: Some(error) } => {
                write!(f, "authorization request failed with HTTP {status} ({:?})", error.code)
            }
            Self::Http { status, error: None } => {
                write!(f, "authorization request failed with HTTP {status}")
            }
            Self::Decode => f.write_str("invalid authorization response"),
        }
    }
}

impl std::error::Error for AgentAppsAuthReadError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountsResponse {
    accounts: Vec<AuthAccount>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingsResponse {
    bindings: Vec<AgentAppsAuthBinding>,
}

impl GatewayClient {
    /// List redacted accounts for the given host and service in the endpoint's tenant scope.
    pub async fn agent_apps_auth_accounts(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<Vec<AuthAccount>, AgentAppsAuthReadError> {
        let response: AccountsResponse =
            self.agent_apps_auth_read("accounts", host_id, service, None, credentials).await?;
        Ok(response.accounts)
    }

    /// Read the service's default account and capabilities in the current scope.
    pub async fn agent_apps_auth_status(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<AuthStatus, AgentAppsAuthReadError> {
        self.agent_apps_auth_read("status", host_id, service, None, credentials).await
    }

    /// List visible supplier bindings, optionally filtering by a scoped account ID.
    pub async fn agent_apps_auth_bindings(
        &self,
        host_id: &str,
        service: AuthService,
        account_id: Option<&str>,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<Vec<AgentAppsAuthBinding>, AgentAppsAuthReadError> {
        let response: BindingsResponse = self
            .agent_apps_auth_read("bindings", host_id, service, account_id, credentials)
            .await?;
        Ok(response.bindings)
    }

    async fn agent_apps_auth_read<T: DeserializeOwned>(
        &self,
        resource: &str,
        host_id: &str,
        service: AuthService,
        account_id: Option<&str>,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<T, AgentAppsAuthReadError> {
        let mut principal = HeaderValue::from_str(credentials.principal_token)
            .map_err(|_| AgentAppsAuthReadError::InvalidPrincipalToken)?;
        principal.set_sensitive(true);

        // The management route uses Basic authentication on the endpoint's tenant scope.
        let mut context = self.endpoint.clone();
        context.auth = None;
        let request = self
            .dynamic_host_client
            .get(format!("{}/v1/agent-apps/auth/{resource}", context.base_url()))
            .query(&ScopeQuery { host_id, service, account_id })
            .header(CACHE_CONTROL, "no-cache, no-store")
            .header("x-agent-apps-token", principal)
            .basic_auth(credentials.username, Some(credentials.password));
        let response = apply_request_context(request, &context)
            .send()
            .await
            .map_err(|_| AgentAppsAuthReadError::Transport)?;
        let status = response.status();
        if !status.is_success() {
            return Err(AgentAppsAuthReadError::Http {
                status: status.as_u16(),
                error: response.json().await.ok(),
            });
        }
        response.json().await.map_err(|error| {
            if error.is_decode() {
                AgentAppsAuthReadError::Decode
            } else {
                AgentAppsAuthReadError::Transport
            }
        })
    }
}

#[derive(serde::Serialize)]
struct ScopeQuery<'a> {
    host_id: &'a str,
    service: AuthService,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}
