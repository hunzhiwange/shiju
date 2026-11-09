//! Managed account mutations on the Gateway management routes.

use reqwest::{Method, RequestBuilder, StatusCode};
use serde::Serialize;
use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};

use super::agent_apps_auth_flow::send_auth_request;
use super::{AgentAppsAuthCredentials, AgentAppsAuthError, GatewayClient};

/// Selects the accounts to sign out locally in the given authorization scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "selection", rename_all = "snake_case")]
pub enum AgentAppsAuthLogout<'a> {
    Account { account_id: &'a str },
    Default,
    All,
}

#[derive(Serialize)]
struct BindingRequest<'a> {
    account_id: Option<&'a str>,
}

impl GatewayClient {
    /// Bind a supplier to an account, or remove the binding with `None`.
    pub async fn agent_apps_auth_bind(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        provider_id: &str,
        account_id: Option<&str>,
    ) -> Result<(), AgentAppsAuthError> {
        if service == AuthService::KimiOauth {
            // Kimi supplier keys can include spaces and Unicode. Keep the key in one URL segment.
            if provider_id.trim().is_empty()
                || provider_id.len() > 128
                || provider_id.chars().any(char::is_control)
                || matches!(provider_id, "." | "..")
                || provider_id.contains(['/', '\\'])
            {
                return Err(AgentAppsAuthError::Request(AuthErrorCode::InvalidInput));
            }
        } else {
            validate_identity(provider_id)?;
        }
        if let Some(id) = account_id {
            validate_identity(id)?;
        }
        let request = self.auth_account_request(
            Method::PUT,
            &["bindings", provider_id],
            host_id,
            service,
            credentials,
        )?;
        send_auth_request(request.json(&BindingRequest { account_id }), StatusCode::NO_CONTENT)
            .await?;
        Ok(())
    }

    /// Select a default account in the given authorization scope.
    pub async fn agent_apps_auth_set_default(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        validate_identity(account_id)?;
        let request = self.auth_account_request(
            Method::PUT,
            &["accounts", account_id, "default"],
            host_id,
            service,
            credentials,
        )?;
        send_auth_request(request, StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    /// Delete an account. The host refuses deletion while a supplier still references it.
    pub async fn agent_apps_auth_delete(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        validate_identity(account_id)?;
        let request = self.auth_account_request(
            Method::DELETE,
            &["accounts", account_id],
            host_id,
            service,
            credentials,
        )?;
        send_auth_request(request, StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    /// Sign out locally while retaining account records and supplier bindings.
    pub async fn agent_apps_auth_logout(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        selection: AgentAppsAuthLogout<'_>,
    ) -> Result<(), AgentAppsAuthError> {
        if let AgentAppsAuthLogout::Account { account_id } = selection {
            validate_identity(account_id)?;
        }
        let request =
            self.auth_account_request(Method::POST, &["logout"], host_id, service, credentials)?;
        send_auth_request(request.json(&selection), StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    /// Request upstream revocation. `RemoteRevocationUnconfirmed` means local logout
    /// committed but the upstream service did not confirm revocation.
    pub async fn agent_apps_auth_revoke(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        validate_identity(account_id)?;
        let request = self.auth_account_request(
            Method::POST,
            &["accounts", account_id, "revoke"],
            host_id,
            service,
            credentials,
        )?;
        send_auth_request(request, StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    fn auth_account_request(
        &self,
        method: Method,
        path: &[&str],
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<RequestBuilder, AgentAppsAuthError> {
        validate_identity(host_id)?;
        self.auth_management_request(method, path, host_id, service, credentials)
    }
}

fn validate_identity(id: &str) -> Result<(), AgentAppsAuthError> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AgentAppsAuthError::Request(AuthErrorCode::InvalidInput));
    }
    Ok(())
}
