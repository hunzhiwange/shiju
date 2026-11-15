//! Typed reads and replacements of execution-host routing settings.
use super::AppRoutingError;
use crate::GatewayClient;
use {
    vw_api_app_identity::AppId,
    vw_api_types::app_routing::AppRoutingConfig,
    vw_api_types::app_routing::RoutingConfig,
    vw_api_transport::id::ProjectId,
};

impl GatewayClient {
    pub async fn app_routing_config(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingConfig, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_config(host, token).await
    }

    pub async fn app_routing_set_config(
        &self,
        host: &str,
        token: &str,
        config: &RoutingConfig,
    ) -> Result<RoutingConfig, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_set_config(host, token, config).await
    }

    pub async fn app_routing_app_config(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<AppRoutingConfig, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_app_config(host, project, app, token).await
    }

    pub async fn app_routing_set_app_config(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
        config: &AppRoutingConfig,
    ) -> Result<AppRoutingConfig, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_set_app_config(host, project, app, token, config).await
    }
}
