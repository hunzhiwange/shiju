//! Managed authorization flows on the Gateway management routes.

use std::fmt;

use reqwest::header::{CACHE_CONTROL, HeaderValue};
use reqwest::{Method, RequestBuilder, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use vw_api_types::agent_apps_auth::{
    AuthError, AuthErrorCode, AuthFlow, AuthFlowId, AuthPasteRequest, AuthService,
    AuthStartRequest, AuthStartResponse,
};

use super::{AgentAppsAuthCredentials, GatewayClient};
use crate::http::apply_request_context;

/// An authorization request failure. HTTP failures retain the gateway's stable code and correlation ID.
#[derive(Clone, PartialEq, Eq)]
pub enum AgentAppsAuthError {
    Request(AuthErrorCode),
    InvalidPrincipalToken,
    Transport,
    Http { status: u16, code: Option<AuthErrorCode>, correlation_id: Option<String> },
    Decode,
}

impl fmt::Debug for AgentAppsAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(code) => f.debug_tuple("Request").field(code).finish(),
            Self::InvalidPrincipalToken => f.write_str("InvalidPrincipalToken"),
            Self::Transport => f.write_str("Transport"),
            Self::Http { status, code, .. } => f
                .debug_struct("Http")
                .field("status", status)
                .field("code", code)
                .finish_non_exhaustive(),
            Self::Decode => f.write_str("Decode"),
        }
    }
}

impl fmt::Display for AgentAppsAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(code) => write!(f, "invalid authorization request ({code:?})"),
            Self::InvalidPrincipalToken => f.write_str("invalid authorization principal token"),
            Self::Transport => f.write_str("authorization request could not be completed"),
            Self::Http { status, code: Some(code), .. } => {
                write!(f, "authorization request failed with HTTP {status} ({code:?})")
            }
            Self::Http { status, code: None, .. } => {
                write!(f, "authorization request failed with HTTP {status}")
            }
            Self::Decode => f.write_str("invalid authorization response"),
        }
    }
}

impl std::error::Error for AgentAppsAuthError {}

impl GatewayClient {
    /// Start a flow for the selected service and execution host.
    pub async fn agent_apps_auth_start(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        input: &AuthStartRequest,
    ) -> Result<AuthStartResponse, AgentAppsAuthError> {
        input.validate_for(service).map_err(AgentAppsAuthError::Request)?;
        let request =
            self.auth_management_request(Method::POST, &["flows"], host_id, service, credentials)?;
        decode(send_auth_request(request.json(input), StatusCode::CREATED).await?).await
    }

    /// Read one flow snapshot. Use the returned `retry_after_seconds` to schedule the next poll.
    pub async fn agent_apps_auth_poll(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
    ) -> Result<AuthFlow, AgentAppsAuthError> {
        let id = flow_id_segment(flow_id)?;
        let request = self.auth_management_request(
            Method::GET,
            &["flows", id],
            host_id,
            service,
            credentials,
        )?;
        decode(send_auth_request(request, StatusCode::OK).await?).await
    }

    /// Request cancellation and wait for confirmation from the execution host.
    pub async fn agent_apps_auth_cancel(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
    ) -> Result<(), AgentAppsAuthError> {
        let id = flow_id_segment(flow_id)?;
        let request = self.auth_management_request(
            Method::DELETE,
            &["flows", id],
            host_id,
            service,
            credentials,
        )?;
        send_auth_request(request, StatusCode::NO_CONTENT).await?;
        Ok(())
    }

    /// Submit sensitive input to the original Claude flow, then poll for the authorization result.
    pub async fn agent_apps_auth_paste(
        &self,
        host_id: &str,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
        input: &AuthPasteRequest,
    ) -> Result<(), AgentAppsAuthError> {
        let id = flow_id_segment(flow_id)?;
        let request = self.auth_management_request(
            Method::POST,
            &["flows", id, "paste"],
            host_id,
            AuthService::ClaudeOfficial,
            credentials,
        )?;
        send_auth_request(request.json(input), StatusCode::ACCEPTED).await?;
        Ok(())
    }

    pub(super) fn auth_management_request(
        &self,
        method: Method,
        path: &[&str],
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<RequestBuilder, AgentAppsAuthError> {
        let mut principal = HeaderValue::from_str(credentials.principal_token)
            .map_err(|_| AgentAppsAuthError::InvalidPrincipalToken)?;
        principal.set_sensitive(true);

        // Management requests use Basic authentication and the endpoint's tenant scope.
        let mut context = self.endpoint.clone();
        context.auth = None;
        let mut url = Url::parse(&format!("{}/v1/agent-apps/auth", context.base_url()))
            .map_err(|_| AgentAppsAuthError::Transport)?;
        url.path_segments_mut().map_err(|_| AgentAppsAuthError::Transport)?.extend(path);
        let request = self
            .dynamic_host_client
            .request(method, url)
            .query(&ScopeQuery { host_id, service })
            .header(CACHE_CONTROL, "no-cache, no-store")
            .header("x-agent-apps-token", principal)
            .basic_auth(credentials.username, Some(credentials.password));
        Ok(apply_request_context(request, &context))
    }
}

#[derive(serde::Serialize)]
struct ScopeQuery<'a> {
    host_id: &'a str,
    service: AuthService,
}

fn flow_id_segment(flow_id: &AuthFlowId) -> Result<&str, AgentAppsAuthError> {
    let id = &flow_id.0;
    // Host-issued IDs are UUIDs. Keep the opaque ID confined to a single path segment.
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') {
        return Err(AgentAppsAuthError::Request(AuthErrorCode::InvalidInput));
    }
    Ok(id)
}

pub(super) async fn send_auth_request(
    request: RequestBuilder,
    expected: StatusCode,
) -> Result<Response, AgentAppsAuthError> {
    // The generic HTTP helpers log URLs and error bodies. Project only the structured error fields.
    let response = request.send().await.map_err(|_| AgentAppsAuthError::Transport)?;
    let status = response.status();
    if status == expected {
        return Ok(response);
    }
    let error = response.json::<AuthError>().await.ok();
    Err(AgentAppsAuthError::Http {
        status: status.as_u16(),
        code: error.as_ref().map(|error| error.code),
        correlation_id: error.map(|error| error.correlation_id),
    })
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, AgentAppsAuthError> {
    response.json().await.map_err(|error| {
        if error.is_decode() { AgentAppsAuthError::Decode } else { AgentAppsAuthError::Transport }
    })
}
