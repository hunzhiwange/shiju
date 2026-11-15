use super::AppRoutingError;
use crate::GatewayClient;
use {
    vw_api_app_identity::AppId,
    vw_api_types::app_routing::RoutingRequestsStatus,
    vw_api_transport::id::ProjectId,
};
impl GatewayClient {
    pub async fn app_routing_requests(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<RoutingRequestsStatus, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_requests(host, project, app, token).await
    }
}
