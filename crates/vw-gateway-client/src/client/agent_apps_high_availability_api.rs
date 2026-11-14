//! Scope-bound failover management. Writes are sent once, including on ambiguous failures.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::{
    agent_apps_high_availability::*,
    error::ApiErrorBody,
    id::ProviderId,
};

/// Structured gateway failures remain available without displaying response payloads or credentials.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppsHighAvailabilityError {
    Transport,
    InvalidCredential,
    NativeTransportRequired,
    Decode,
    Http { status: u16, body: Option<ApiErrorBody> },
}

impl std::fmt::Display for AgentAppsHighAvailabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("failover request transport failed"),
            Self::InvalidCredential => f.write_str("invalid application credential"),
            Self::NativeTransportRequired => f.write_str("failover requests require native transport"),
            Self::Decode => f.write_str("invalid failover response"),
            Self::Http { status, .. } => write!(f, "failover request HTTP {status}"),
        }
    }
}
impl std::error::Error for AgentAppsHighAvailabilityError {}

impl GatewayClient {
    /// GET the scoped `queue` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_queue(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::GET, "queue", scope,
            None, None, token,
        ).await
    }

    /// GET the scoped `candidates` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_candidates(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverCandidatesResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::GET, "candidates", scope,
            None, None, token,
        ).await
    }

    /// POST the scoped `queue/members` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_add_member(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueMemberRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request(
            reqwest::Method::POST, "queue/members", scope,
            None, Some(body), token,
        ).await
    }

    /// DELETE the scoped `queue/members` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_remove_member(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueMemberRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request(
            reqwest::Method::DELETE, "queue/members", scope,
            None, Some(body), token,
        ).await
    }

    /// PUT the scoped `queue/order` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_reorder(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueReorderRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request(
            reqwest::Method::PUT, "queue/order", scope,
            None, Some(body), token,
        ).await
    }

    /// PUT the scoped `enabled` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_set_enabled(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueEnabledRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request(
            reqwest::Method::PUT, "enabled", scope,
            None, Some(body), token,
        ).await
    }

    /// GET the scoped `health` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_health(
        &self,
        scope: &FailoverQueueQuery,
        provider_id: &ProviderId,
        token: Option<&str>,
    ) -> Result<FailoverHealthResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::GET, "health", scope,
            Some(provider_id), None, token,
        ).await
    }

    /// POST the scoped `health/reset` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_reset_health(
        &self,
        scope: &FailoverQueueQuery,
        provider_id: &ProviderId,
        token: Option<&str>,
    ) -> Result<FailoverHealthResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::POST, "health/reset", scope,
            Some(provider_id), None, token,
        ).await
    }

    /// GET the scoped `route` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_route(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverRouteResponse, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::GET, "route", scope,
            None, None, token,
        ).await
    }

    /// GET the scoped `policy` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_policy(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverPolicy, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request::<(), _>(
            reqwest::Method::GET, "policy", scope,
            None, None, token,
        ).await
    }

    /// PUT the scoped `policy` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_save_policy(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverPolicy,
        token: Option<&str>,
    ) -> Result<FailoverPolicy, AgentAppsHighAvailabilityError> {
        self.agent_apps_failover_request(
            reqwest::Method::PUT, "policy", scope,
            None, Some(body), token,
        ).await
    }

    async fn agent_apps_failover_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        scope: &FailoverQueueQuery,
        provider_id: Option<&ProviderId>,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<T, AgentAppsHighAvailabilityError> {
        use AgentAppsHighAvailabilityError as Error;
        // The general JSON helpers erase structured HTTP statuses. Reuse the native
        // transport and request context here to retain conflicts and permission errors.
        let transport = self.native_transport().map_err(|_| Error::NativeTransportRequired)?;
        let mut request = transport.request(
            method,
            format!("{}/v1/agent-apps/high-availability/{resource}", self.endpoint.base_url()),
        ).query(scope);
        if let Some(provider_id) = provider_id {
            request = request.query(&[("providerId", provider_id.as_ref())]);
        }
        if let Some(token) = token {
            let mut value = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::InvalidCredential)?;
            value.set_sensitive(true);
            request = request.header("x-agent-apps-token", value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| Error::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| Error::Transport)?;
        if !status.is_success() {
            // Discard unstructured proxy pages; never include raw response text in errors.
            return Err(Error::Http {
                status: status.as_u16(),
                body: serde_json::from_slice(&bytes).ok(),
            });
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Decode)
    }
}
