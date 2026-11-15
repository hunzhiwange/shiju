//! Typed management requests for execution-host egress.
use super::AppRoutingError;
use crate::GatewayClient;
use vw_api_types::app_routing::{RoutingEgressStatus, RoutingEgressUpdate};

impl GatewayClient {
    pub async fn app_routing_egress(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingEgressStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_egress(host, token).await
    }

    pub async fn app_routing_set_egress(
        &self,
        host: &str,
        token: &str,
        update: &RoutingEgressUpdate,
    ) -> Result<RoutingEgressStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_set_egress(host, token, update).await
    }
}
