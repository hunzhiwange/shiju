use super::AppRoutingError;
use crate::GatewayClient;
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::RoutingModeRequest,
    vw_api_app_routing::RoutingModeResult,
    vw_api_transport::id::ProjectId,
};

impl GatewayClient {
    pub async fn apply_app_routing_mode(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
        request: &RoutingModeRequest,
    ) -> Result<RoutingModeResult, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .apply_app_routing_mode(host, project, app, token, request).await
    }
}
