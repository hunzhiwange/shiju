//! 识矩应用兼容门面。
use super::{GatewayClient, ShijuCredential};
pub use gateway_shiju_app::*;

impl GatewayClient {
    /// Reads the SHIJU application catalog through the local gateway.
    pub async fn shiju_app_catalog(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAppCatalogDto, String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_app_catalog(credential).await
    }

    /// Reads applications installed for the current SHIJU account.
    pub async fn shiju_apps(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAppListDto, String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_apps(credential).await
    }

    /// Reads the personal API key binding for a SHIJU application installation.
    pub async fn shiju_app_api_key_binding(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuAppApiKeyBindingDto, String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_app_api_key_binding(credential, installation_id).await
    }

    /// Reads IAM API keys available to the current SHIJU account.
    pub async fn shiju_app_api_keys(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuIamApiKeyListDto, String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_app_api_keys(credential).await
    }

    /// Saves a personal API key binding for a SHIJU application installation.
    pub async fn shiju_app_save_api_key_binding(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
        request: &ShijuAppApiKeyBindingSaveRequest,
    ) -> Result<(), String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_app_save_api_key_binding(credential, installation_id, request).await
    }

    /// Prepares a gateway-validated SHIJU application launch.
    pub async fn shiju_app_prepare_launch(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuAppLaunchDto, String> {
        gateway_shiju_app::GatewayShijuAppClient::new(self.transport.clone())
            .shiju_app_prepare_launch(credential, installation_id).await
    }
}
