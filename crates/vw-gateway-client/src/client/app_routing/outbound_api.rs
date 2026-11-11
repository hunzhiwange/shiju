//! Typed management requests for execution-host egress.
use super::AppRoutingError;
use crate::GatewayClient;
use reqwest::Method;
use vw_api_types::app_routing::{RoutingEgressStatus, RoutingEgressUpdate};

impl GatewayClient {
    pub async fn app_routing_egress(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingEgressStatus, AppRoutingError> {
        self.app_routing_request(Method::GET, &[host, "egress"], token, None::<&()>).await
    }

    pub async fn app_routing_set_egress(
        &self,
        host: &str,
        token: &str,
        update: &RoutingEgressUpdate,
    ) -> Result<RoutingEgressStatus, AppRoutingError> {
        self.app_routing_request(Method::PUT, &[host, "egress"], token, Some(update)).await
    }
}
