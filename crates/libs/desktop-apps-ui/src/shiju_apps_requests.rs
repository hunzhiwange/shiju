use gateway_shiju_app::*;
use gateway_shiju_account::ShijuCredential;
pub(super) async fn gateway_shiju_app_catalog_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
) -> Result<ShijuAppCatalogDto, String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_catalog(credential).await
}

pub(super) async fn gateway_shiju_apps_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
) -> Result<ShijuAppListDto, String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_apps(credential).await
}

pub(super) async fn gateway_shiju_app_api_key_binding_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppApiKeyBindingDto, String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_api_key_binding(credential, installation_id).await
}

pub(super) async fn gateway_shiju_app_api_keys_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
) -> Result<ShijuIamApiKeyListDto, String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_api_keys(credential).await
}

pub(super) async fn gateway_shiju_app_save_api_key_binding_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
    installation_id: &str,
    request: &ShijuAppApiKeyBindingSaveRequest,
) -> Result<(), String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_save_api_key_binding(credential, installation_id, request).await
}

pub(super) async fn gateway_shiju_app_prepare_launch_async(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppLaunchDto, String> {
    let client = client?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_prepare_launch(credential, installation_id).await
}
