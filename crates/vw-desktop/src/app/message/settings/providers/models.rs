//! 注入桌面端点与原有本地配置路径，委派提供商领域任务。
pub use desktop_settings_providers::provider_models::*;
use crate::app::state::{CustomProviderDraft, ModelCatalogEntry};
use crate::app::i18n::settings_models::messages::Feedback;
type Loaded = Result<(std::collections::HashMap<String, model_types::Info>, Vec<String>, bool, Vec<ModelCatalogEntry>), Feedback>;
fn access() -> desktop_config::ConfigAccess {
    #[cfg(not(target_arch = "wasm32"))]
    let path = desktop_config::config_gateway::project_config_override_path();
    #[cfg(target_arch = "wasm32")]
    let path = None;
    desktop_config::ConfigAccess::new(crate::app::config::gateway_client_endpoint(), path)
}
pub async fn load_custom_provider_draft(provider_id: String) -> Result<CustomProviderDraft, Feedback> {
    desktop_settings_providers::provider_tasks::load_custom_provider_draft(access(), provider_id).await
}
pub async fn connect_provider(provider_id: String, api_key: String) -> Loaded {
    desktop_settings_providers::provider_tasks::connect_provider(crate::app::config::gateway_client_endpoint(), access(), provider_id, api_key).await
}
pub async fn disconnect_provider(provider_id: String) -> Loaded {
    desktop_settings_providers::provider_tasks::disconnect_provider(crate::app::config::gateway_client_endpoint(), access(), provider_id).await
}
pub async fn save_custom_provider(draft: CustomProviderDraft) -> Loaded {
    desktop_settings_providers::provider_tasks::save_custom_provider(crate::app::config::gateway_client_endpoint(), access(), draft).await
}
