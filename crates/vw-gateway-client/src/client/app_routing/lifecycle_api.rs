//! Typed application routing requests over the existing authenticated gateway transport.
use crate::GatewayClient;
use super::AppRoutingError;
use {
    vw_api_app_routing::RoutingHealth,
    vw_api_app_routing::RoutingStatus,
};

impl GatewayClient {
    pub async fn app_routing_status(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_status(host, token).await
    }

    pub async fn app_routing_health(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingHealth, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_health(host, token).await
    }

    pub async fn app_routing_start(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_start(host, token).await
    }

    pub async fn app_routing_stop(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_stop(host, token).await
    }
}
