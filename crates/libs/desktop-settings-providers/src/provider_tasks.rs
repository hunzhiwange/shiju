use desktop_i18n::settings_models::messages::Feedback;
use crate::provider_state::{CustomProviderDraft, ModelCatalogEntry};
use crate::provider_models::*;
use crate::provider_catalog::{build_catalog_from_sources, load_popular_providers_from_config};
use std::collections::HashSet;
use provider_resolution as model_provider;
use provider_model_cache as provider_models;
use auth_file_store as shared_auth;
pub async fn load_gateway_providers(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<std::collections::HashMap<String, model_types::Info>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint)?;
    let response = gateway_model_provider::GatewayModelProviderClient::new(client.transport().clone()).provider_list(None).await?;
    Ok(response.all.into_iter().map(|provider| (provider.id.clone(), provider)).collect())
}
/// 处理 `load_custom_provider_draft` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub async fn load_custom_provider_draft(
    config: desktop_config::ConfigAccess,
    provider_id: String,
) -> Result<CustomProviderDraft, Feedback> {
    let cfg = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
    let raw = cfg.providers.get(&provider_id);
    let settings_providers = model_provider::list_for_settings().await;
    let settings = settings_providers.get(&provider_id);
    let catalog = provider_models::get().await;
    let catalog_model_ids = catalog
        .get(&provider_id)
        .map(|provider| provider.models.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();

    match (raw, settings) {
        (Some(raw), Some(settings)) => {
            merge_custom_provider_draft(&provider_id, raw, settings, &catalog_model_ids)
        }
        (Some(raw), None) => {
            let mut draft = custom_provider_draft_from_value(&provider_id, raw)?;
            for model in &mut draft.models {
                model.from_catalog = catalog_model_ids.contains(&model.model_id);
            }
            Ok(draft)
        }
        (None, Some(settings)) => {
            Ok(draft_from_settings_provider(&provider_id, settings, &catalog_model_ids))
        }
        (None, None) => Err(Feedback::owned(
            "提供商 %s 不在系统配置的 providers 中",
            vec![provider_id.to_string()],
        )),
    }
}

/// 处理 `connect_provider` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub async fn connect_provider(
    endpoint: gateway_endpoint::GatewayEndpoint,
    config: desktop_config::ConfigAccess,
    provider_id: String,
    api_key: String,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    Feedback,
> {
    let path = auth_filepath();
    shared_auth::set_to(
        &path,
        &provider_id,
        &auth_types::Info::Api(auth_types::ApiInfo { key: api_key }),
    )
    .map_err(|e| e.to_string())?;
    model_provider::invalidate_cache().await;
    let local_providers = model_provider::list_for_settings().await;
    if let Some(p) = local_providers.get(&provider_id) {
        let mut provider_cfg = serde_json::Map::new();
        if let Some(api) = p.models.values().find_map(|model| {
            let url = model.api.url.trim();
            (!url.is_empty()).then(|| url.to_string())
        }) {
            provider_cfg.insert("api".to_string(), serde_json::Value::String(api));
        }
        if !p.env.is_empty() {
            provider_cfg.insert(
                "env".to_string(),
                serde_json::Value::Array(
                    p.env.iter().map(|s| serde_json::Value::String(s.clone())).collect(),
                ),
            );
        }
        if !p.name.trim().is_empty() {
            provider_cfg.insert("name".to_string(), serde_json::Value::String(p.name.clone()));
        }

        let mut provider_obj = serde_json::Map::new();
        provider_obj.insert(provider_id.to_string(), serde_json::Value::Object(provider_cfg));
        let mut root = serde_json::Map::new();
        root.insert("providers".to_string(), serde_json::Value::Object(provider_obj));
        let patch = serde_json::Value::Object(root);
        config.patch_full_agent_config_async(patch).await.map_err(Feedback::gateway)?;
    }
    let providers = load_gateway_providers(endpoint.clone()).await.map_err(Feedback::gateway)?;
    let cfg = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
    let (patterns, configured) = load_popular_providers_from_config(&cfg);
    let raw_providers = provider_models::get().await;
    let catalog = build_catalog_from_sources(&raw_providers, &providers);
    Ok((providers, patterns, configured, catalog))
}

/// 处理 `disconnect_provider` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub async fn disconnect_provider(
    endpoint: gateway_endpoint::GatewayEndpoint,
    config: desktop_config::ConfigAccess,
    provider_id: String,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    Feedback,
> {
    let path = auth_filepath();
    shared_auth::remove_from(&path, &provider_id).map_err(|e| e.to_string())?;
    config.remove_global_provider_via_gateway(&provider_id).await.map_err(Feedback::gateway)?;
    model_provider::invalidate_cache().await;
    let providers = load_gateway_providers(endpoint.clone()).await.map_err(Feedback::gateway)?;
    let cfg = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
    let (patterns, configured) = load_popular_providers_from_config(&cfg);
    let raw_providers = provider_models::get().await;
    let catalog = build_catalog_from_sources(&raw_providers, &providers);
    Ok((providers, patterns, configured, catalog))
}

/// 处理 `save_custom_provider` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub async fn save_custom_provider(
    endpoint: gateway_endpoint::GatewayEndpoint,
    config: desktop_config::ConfigAccess,
    draft: CustomProviderDraft,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    Feedback,
> {
    let provider_id = draft.provider_id.trim().to_string();
    let cfg = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
    let existing_provider = cfg.providers.get(&provider_id);
    let (_, patch) = custom_provider_patch_from_draft(&draft, existing_provider)?;

    config.patch_full_agent_config_async(patch).await.map_err(Feedback::gateway)?;
    if !crate::provider_state::is_shiju_official_provider_id(&provider_id)
        && !draft.api_key.trim().is_empty()
    {
        let path = auth_filepath();
        shared_auth::set_to(
            &path,
            &provider_id,
            &auth_types::Info::Api(auth_types::ApiInfo { key: draft.api_key.trim().to_string() }),
        )
        .map_err(|e| e.to_string())?;
    }
    model_provider::invalidate_cache().await;
    let providers = load_gateway_providers(endpoint.clone()).await.map_err(Feedback::gateway)?;
    let cfg = config.load_full_agent_config_async().await.map_err(Feedback::gateway)?;
    let (patterns, configured) = load_popular_providers_from_config(&cfg);
    let raw_providers = provider_models::get().await;
    let catalog = build_catalog_from_sources(&raw_providers, &providers);
    Ok((providers, patterns, configured, catalog))
}
