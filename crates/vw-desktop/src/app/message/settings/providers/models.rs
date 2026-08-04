//! 处理模型提供商设置子模块的目录、认证和连接状态。

use crate::app::config;
use crate::app::config::server_config_unreachable_error;
use crate::app::provider::provider as model_provider;
use crate::app::provider::provider_models;
use crate::app::state::{CustomProviderDraft, ModelCatalogEntry};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use vw_shared::auth::{self as shared_auth};

use super::helpers::{build_catalog_from_sources, load_popular_providers_from_config};

fn auth_filepath() -> std::path::PathBuf {
    let base = directories::BaseDirs::new();
    let home = base
        .as_ref()
        .map(|b| std::path::PathBuf::from(b.home_dir()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let data = base
        .as_ref()
        .map(|b| std::path::PathBuf::from(b.data_dir()).join(vw_config_types::paths::APP_DIR_NAME))
        .unwrap_or_else(|| {
            home.join(".local").join("share").join(vw_config_types::paths::APP_DIR_NAME)
        });
    shared_auth::store::resolve_filepath(&home, &data)
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

/// 处理 `current_provider_api_key` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `None` 表示输入为空或当前状态不需要生成后续值。
pub fn current_provider_api_key(provider_id: &str) -> Option<String> {
    match shared_auth::store::get_from(&auth_filepath(), provider_id) {
        Some(shared_auth::Info::Api(info)) => Some(info.key),
        Some(shared_auth::Info::Wellknown(info)) => Some(info.key),
        _ => None,
    }
}

pub(crate) fn merge_discovered_model_drafts(
    existing: &mut Vec<crate::app::state::CustomProviderModelDraft>,
    discovered: Vec<String>,
) {
    let mut merged = BTreeMap::new();

    for model in existing.drain(..) {
        let model_id = model.model_id.trim().to_string();
        if model_id.is_empty() {
            continue;
        }
        merged.entry(model_id.clone()).or_insert(crate::app::state::CustomProviderModelDraft {
            model_id,
            display_name: model.display_name,
        });
    }

    for model_id in discovered {
        let model_id = model_id.trim().to_string();
        if model_id.is_empty() {
            continue;
        }
        merged.entry(model_id.clone()).or_insert(crate::app::state::CustomProviderModelDraft {
            model_id,
            display_name: String::new(),
        });
    }

    *existing = merged.into_values().collect();
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn discover_custom_provider_models(
    draft: CustomProviderDraft,
) -> Result<Vec<String>, String> {
    use vw_provider_resolver::remote_models::{
        discover_openai_compatible_models, is_real_bearer, resolve_api_base,
    };

    let providers = model_provider::list_for_settings().await;
    let provider = providers.get(draft.provider_id.trim());
    let bearer = if is_real_bearer(&draft.api_key) {
        Some(draft.api_key.trim().to_string())
    } else {
        provider
            .and_then(|provider| provider.key.as_deref())
            .filter(|key| is_real_bearer(key))
            .map(str::to_string)
    }
    .ok_or_else(|| "请先填写或保存真实 API Key".to_string())?;

    let configured_api = draft.base_url.trim();
    let api_base = if configured_api.is_empty() {
        provider.and_then(|provider| resolve_api_base(provider, None))
    } else {
        Some(configured_api.trim_end_matches('/').to_string())
    }
    .ok_or_else(|| "基础 URL 不能为空".to_string())?;

    let models = discover_openai_compatible_models(&api_base, &bearer)
        .await
        .map_err(|error| discovery_error_message(error.category()))?;
    if models.is_empty() {
        return Err("未发现可用模型".to_string());
    }
    Ok(models)
}

#[cfg(not(target_arch = "wasm32"))]
fn discovery_error_message(category: &str) -> String {
    let detail = match category {
        "timeout" => "请求超时",
        "request" => "网络请求失败",
        "http_status" => "服务返回错误状态",
        "redirect" => "服务返回重定向",
        "response_too_large" => "响应过大",
        "invalid_response" => "返回格式无效",
        "client" => "请求初始化失败",
        _ => "未知错误",
    };
    format!("自动发现模型失败：{detail}")
}

#[cfg(target_arch = "wasm32")]
pub async fn discover_custom_provider_models(
    _draft: CustomProviderDraft,
) -> Result<Vec<String>, String> {
    Err("当前平台暂不支持自动发现模型".to_string())
}

/// 处理 `load_custom_provider_draft` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub async fn load_custom_provider_draft(
    provider_id: String,
) -> Result<CustomProviderDraft, String> {
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
    let raw = cfg
        .providers
        .get(&provider_id)
        .ok_or_else(|| format!("提供商 {} 不在系统配置的 providers 中", provider_id))?;
    custom_provider_draft_from_value(&provider_id, raw)
}

fn custom_provider_draft_from_value(
    provider_id: &str,
    raw: &Value,
) -> Result<CustomProviderDraft, String> {
    let Some(obj) = raw.as_object() else {
        return Err(format!("提供商 {} 的配置格式无效", provider_id));
    };

    let headers = extract_shared_headers(obj);
    let models = obj
        .get("models")
        .and_then(Value::as_object)
        .map(|models| {
            let mut items = models
                .iter()
                .map(|(model_id, value)| crate::app::state::CustomProviderModelDraft {
                    model_id: model_id.clone(),
                    display_name: value
                        .as_object()
                        .and_then(|model| model.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect::<Vec<_>>();
            items.sort_by(|a, b| a.model_id.cmp(&b.model_id));
            items
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec![crate::app::state::CustomProviderModelDraft::default()]);

    Ok(CustomProviderDraft {
        provider_id: provider_id.to_string(),
        display_name: obj.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        base_url: obj.get("api").and_then(Value::as_str).unwrap_or_default().to_string(),
        api_key: current_provider_api_key(provider_id).unwrap_or_default(),
        headers,
        models,
    })
}

fn extract_shared_headers(obj: &Map<String, Value>) -> Vec<crate::app::state::ProviderHeaderDraft> {
    let Some(models) = obj.get("models").and_then(Value::as_object) else {
        return vec![crate::app::state::ProviderHeaderDraft::default()];
    };

    for model in models.values() {
        let Some(headers) =
            model.as_object().and_then(|model| model.get("headers")).and_then(Value::as_object)
        else {
            continue;
        };

        let mut items = headers
            .iter()
            .filter_map(|(key, value)| {
                value.as_str().map(|value| crate::app::state::ProviderHeaderDraft {
                    key: key.clone(),
                    value: value.to_string(),
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        if !items.is_empty() {
            return items;
        }
    }

    vec![crate::app::state::ProviderHeaderDraft::default()]
}

/// 处理 `connect_provider` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub async fn connect_provider(
    provider_id: String,
    api_key: String,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    String,
> {
    let path = auth_filepath();
    shared_auth::store::set_to(
        &path,
        &provider_id,
        &shared_auth::Info::Api(shared_auth::ApiInfo { key: api_key }),
    )
    .map_err(|e| e.to_string())?;
    model_provider::invalidate_cache().await;
    let providers = model_provider::list_for_settings().await;
    if let Some(p) = providers.get(&provider_id) {
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
        config::patch_full_agent_config_async(patch)
            .await
            .map_err(server_config_unreachable_error)?;
    }
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
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
    provider_id: String,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    String,
> {
    let path = auth_filepath();
    shared_auth::store::remove_from(&path, &provider_id).map_err(|e| e.to_string())?;
    config::remove_global_provider_via_gateway(&provider_id)
        .await
        .map_err(server_config_unreachable_error)?;
    model_provider::invalidate_cache().await;
    let providers = model_provider::list_for_settings().await;
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
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
    draft: CustomProviderDraft,
) -> Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    String,
> {
    let provider_id = draft.provider_id.trim().to_string();
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
    let existing_provider = cfg.providers.get(&provider_id);
    let (_, patch) = custom_provider_patch_from_draft(&draft, existing_provider);

    config::patch_full_agent_config_async(patch).await.map_err(server_config_unreachable_error)?;
    if !draft.api_key.trim().is_empty() {
        let path = auth_filepath();
        shared_auth::store::set_to(
            &path,
            &provider_id,
            &shared_auth::Info::Api(shared_auth::ApiInfo { key: draft.api_key.trim().to_string() }),
        )
        .map_err(|e| e.to_string())?;
    }
    model_provider::invalidate_cache().await;
    let providers = model_provider::list_for_settings().await;
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
    let (patterns, configured) = load_popular_providers_from_config(&cfg);
    let raw_providers = provider_models::get().await;
    let catalog = build_catalog_from_sources(&raw_providers, &providers);
    Ok((providers, patterns, configured, catalog))
}

fn custom_provider_patch_from_draft(
    draft: &CustomProviderDraft,
    existing_provider: Option<&Value>,
) -> (String, Value) {
    let provider_id = draft.provider_id.trim().to_string();
    let base_url = draft.base_url.trim().to_string();
    let mut shared_headers = serde_json::Map::new();
    for h in &draft.headers {
        let k = h.key.trim();
        let v = h.value.trim();
        if k.is_empty() || v.is_empty() {
            continue;
        }
        shared_headers.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }

    let existing_models = existing_provider
        .and_then(Value::as_object)
        .and_then(|provider| provider.get("models"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut models_obj = serde_json::Map::new();
    for m in &draft.models {
        let model_id = m.model_id.trim();
        if model_id.is_empty() {
            continue;
        }
        let mut model_cfg =
            existing_models.get(model_id).and_then(Value::as_object).cloned().unwrap_or_default();
        if !m.display_name.trim().is_empty() {
            model_cfg.insert(
                "name".to_string(),
                serde_json::Value::String(m.display_name.trim().to_string()),
            );
        } else {
            model_cfg.remove("name");
        }
        if !shared_headers.is_empty() {
            model_cfg
                .insert("headers".to_string(), serde_json::Value::Object(shared_headers.clone()));
        } else {
            model_cfg.remove("headers");
        }
        models_obj.insert(model_id.to_string(), serde_json::Value::Object(model_cfg));
    }

    let mut provider_cfg =
        existing_provider.and_then(Value::as_object).cloned().unwrap_or_default();
    if !draft.display_name.trim().is_empty() {
        provider_cfg.insert(
            "name".to_string(),
            serde_json::Value::String(draft.display_name.trim().to_string()),
        );
    } else {
        provider_cfg.remove("name");
    }
    provider_cfg.insert("api".to_string(), serde_json::Value::String(base_url));
    provider_cfg.insert("models".to_string(), serde_json::Value::Object(models_obj));

    let mut provider_obj = serde_json::Map::new();
    provider_obj.insert(provider_id.clone(), serde_json::Value::Object(provider_cfg));

    let mut root = serde_json::Map::new();
    root.insert("providers".to_string(), serde_json::Value::Object(provider_obj));
    (provider_id, serde_json::Value::Object(root))
}
