use super::AppRoutingError;
use crate::GatewayAppRoutingClient;
use vw_api_types::app_routing::{RoutingClientHost, RoutingClientHostUpdate};

impl GatewayAppRoutingClient {
    pub async fn app_routing_client_host(&self, host: &str, token: &str) -> Result<RoutingClientHost, AppRoutingError> {
        self.app_routing_request(reqwest::Method::GET, &[host, "client-host"], token, None::<&()>).await
    }

    pub async fn app_routing_set_client_host(&self, host: &str, token: &str, update: &RoutingClientHostUpdate) -> Result<RoutingClientHost, AppRoutingError> {
        self.app_routing_request(reqwest::Method::PUT, &[host, "client-host"], token, Some(update)).await
    }
}
