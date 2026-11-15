use super::AppRoutingError;
use crate::GatewayClient;
use vw_api_app_routing::{RoutingClientHost, RoutingClientHostUpdate};

impl GatewayClient {
    pub async fn app_routing_client_host(&self, host: &str, token: &str) -> Result<RoutingClientHost, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_client_host(host, token).await
    }

    pub async fn app_routing_set_client_host(&self, host: &str, token: &str, update: &RoutingClientHostUpdate) -> Result<RoutingClientHost, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_set_client_host(host, token, update).await
    }
}
