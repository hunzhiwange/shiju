use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_environment::paths::*;
use vw_api_app_identity::{AppId};


impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_paths(
        &self,
        app: AppId,
        request_body: &ProviderPathsRequest,
        token: &str,
    ) -> Result<ProviderPaths, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_paths(app, request_body, token).await
    }

    pub async fn app_provider_directory(
        &self,
        app: AppId,
        request: &ProviderDirectoryQuery,
        token: &str,
    ) -> Result<ProviderDirectoryStatus, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_directory(app, request, token).await
    }

    pub async fn app_provider_directory_save(
        &self,
        app: AppId,
        request: &ProviderDirectoryUpdate,
        token: &str,
    ) -> Result<ProviderDirectoryStatus, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_directory_save(app, request, token).await
    }
}
