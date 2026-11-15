//! 模型目录加载和状态配置持久化。网关端点由桌面壳注入。
use gateway_endpoint::GatewayEndpoint;
use desktop_i18n::settings_models::messages::Feedback;
use crate::models::{ModelMessage, ModelStatusChange, ModelsLoaded, summarize_models, model_status_patch, provider_model_status_reset_patch};

pub async fn load_models_for_settings(endpoint: GatewayEndpoint) -> ModelsLoaded {
    let client = vw_gateway_client::GatewayClient::new(endpoint).map_err(Feedback::gateway)?;
    let response = gateway_model_provider::GatewayModelProviderClient::new(client.transport().clone())
        .provider_list(None).await.map_err(Feedback::gateway)?;
    Ok(summarize_models(response.all.into_iter().map(|provider| (provider.id.clone(), provider)).collect()))
}
pub async fn save_statuses(endpoint: GatewayEndpoint, changes: Vec<ModelStatusChange>) -> ModelMessage {
    let config = desktop_config::ConfigAccess::new(endpoint, None);
    let result = config.patch_full_agent_config_async(model_status_patch(&changes)).await.map_err(Feedback::gateway);
    ModelMessage::ModelStatusesSaved(changes, result)
}
pub async fn reset_statuses(endpoint: GatewayEndpoint, provider_id: String, changes: Vec<ModelStatusChange>) -> ModelMessage {
    let config = desktop_config::ConfigAccess::new(endpoint.clone(), None);
    let result = async {
        let full_config = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
        if let Some(patch) = provider_model_status_reset_patch(&full_config, &provider_id) {
            config.patch_full_agent_config_async(patch).await.map_err(Feedback::gateway)?;
        }
        load_models_for_settings(endpoint).await
    }.await;
    ModelMessage::ProviderModelsResetCompleted(changes, result)
}
