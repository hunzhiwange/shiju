use super::GatewayClient;
use gateway_app_failover::{GatewayAppFailoverClient, AgentAppsHighAvailabilityError};
use vw_api_app_failover::*;
use vw_api_transport::id::ProviderId;

impl GatewayClient {
    /// GET the scoped `queue` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_queue(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_queue(scope, token).await
    }

    /// GET the scoped `candidates` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_candidates(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverCandidatesResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_candidates(scope, token).await
    }

    /// POST the scoped `queue/members` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_add_member(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueMemberRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_add_member(scope, body, token).await
    }

    /// DELETE the scoped `queue/members` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_remove_member(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueMemberRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_remove_member(scope, body, token).await
    }

    /// PUT the scoped `queue/order` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_reorder(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueReorderRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_reorder(scope, body, token).await
    }

    /// PUT the scoped `enabled` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_set_enabled(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverQueueEnabledRequest,
        token: Option<&str>,
    ) -> Result<FailoverQueueResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_set_enabled(scope, body, token).await
    }

    /// GET the scoped `health` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_health(
        &self,
        scope: &FailoverQueueQuery,
        provider_id: &ProviderId,
        token: Option<&str>,
    ) -> Result<FailoverHealthResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_health(scope, provider_id, token).await
    }

    /// POST the scoped `health/reset` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_reset_health(
        &self,
        scope: &FailoverQueueQuery,
        provider_id: &ProviderId,
        token: Option<&str>,
    ) -> Result<FailoverHealthResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_reset_health(scope, provider_id, token).await
    }

    /// GET the scoped `route` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_route(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverRouteResponse, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_route(scope, token).await
    }

    /// GET the scoped `policy` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_policy(
        &self,
        scope: &FailoverQueueQuery,
        token: Option<&str>,
    ) -> Result<FailoverPolicy, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_policy(scope, token).await
    }

    /// PUT the scoped `policy` resource. `token` is an optional application grant.
    pub async fn agent_apps_failover_save_policy(
        &self,
        scope: &FailoverQueueQuery,
        body: &FailoverPolicy,
        token: Option<&str>,
    ) -> Result<FailoverPolicy, AgentAppsHighAvailabilityError> {
        GatewayAppFailoverClient::new(self.transport.clone()).agent_apps_failover_save_policy(scope, body, token).await
    }

}
