//! 处理模型提供商设置子模块的目录、认证和连接状态。

use crate::app::config;
use crate::app::config::server_config_unreachable_error;
use crate::app::provider::provider as model_provider;
use crate::app::provider::provider_models;
use crate::app::state::{CustomProviderDraft, ModelCatalogEntry};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use vw_shared::auth::{self as shared_auth};

use super::helpers::{build_catalog_from_sources, load_popular_providers_from_config};
use super::super::util::load_gateway_providers;

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
            ..Default::default()
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
            ..Default::default()
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
                .map(|(model_id, value)| custom_model_draft_from_value(model_id, value))
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

fn custom_model_draft_from_value(
    model_id: &str,
    value: &Value,
) -> crate::app::state::CustomProviderModelDraft {
    let Some(model) = value.as_object() else {
        return crate::app::state::CustomProviderModelDraft {
            model_id: model_id.to_string(),
            original_model_id: Some(model_id.to_string()),
            ..Default::default()
        };
    };

    let limit = model.get("limit").and_then(Value::as_object);
    let modalities = model.get("modalities").and_then(Value::as_object);
    crate::app::state::CustomProviderModelDraft {
        model_id: model_id.to_string(),
        display_name: model.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        family: model.get("family").and_then(Value::as_str).unwrap_or_default().to_string(),
        context_limit: limit
            .and_then(|limit| limit.get("context"))
            .and_then(Value::as_u64)
            .map(|value| value.to_string())
            .unwrap_or_default(),
        input_limit: limit
            .and_then(|limit| limit.get("input"))
            .and_then(Value::as_u64)
            .map(|value| value.to_string())
            .unwrap_or_default(),
        output_limit: limit
            .and_then(|limit| limit.get("output"))
            .and_then(Value::as_u64)
            .map(|value| value.to_string())
            .unwrap_or_default(),
        reasoning: model.get("reasoning").and_then(Value::as_bool),
        tool_call: model.get("tool_call").and_then(Value::as_bool),
        temperature: model.get("temperature").and_then(Value::as_bool),
        attachment: model.get("attachment").and_then(Value::as_bool),
        interleaved: model.get("interleaved").and_then(Value::as_bool),
        input_modalities: extract_modalities(modalities, "input"),
        output_modalities: extract_modalities(modalities, "output"),
        original_model_id: Some(model_id.to_string()),
    }
}

fn extract_modalities(modalities: Option<&Map<String, Value>>, side: &str) -> Option<Vec<String>> {
    modalities
        .and_then(|modalities| modalities.get(side))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
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
        config::patch_full_agent_config_async(patch)
            .await
            .map_err(server_config_unreachable_error)?;
    }
    let providers = load_gateway_providers().await?;
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
    let providers = load_gateway_providers().await?;
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
    let (_, patch) = custom_provider_patch_from_draft(&draft, existing_provider)?;

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
    let providers = load_gateway_providers().await?;
    let cfg =
        config::load_full_agent_config_async().await.map_err(server_config_unreachable_error)?;
    let (patterns, configured) = load_popular_providers_from_config(&cfg);
    let raw_providers = provider_models::get().await;
    let catalog = build_catalog_from_sources(&raw_providers, &providers);
    Ok((providers, patterns, configured, catalog))
}

pub(crate) fn custom_provider_patch_from_draft(
    draft: &CustomProviderDraft,
    existing_provider: Option<&Value>,
) -> Result<(String, Value), String> {
    let provider_id = draft.provider_id.trim().to_string();
    let base_url = draft.base_url.trim().to_string();
    validate_model_drafts(&draft.models)?;
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
        let existing_model = existing_models
            .get(model_id)
            .or_else(|| m.original_model_id.as_deref().and_then(|id| existing_models.get(id)));
        let mut model_cfg = existing_model.and_then(Value::as_object).cloned().unwrap_or_default();
        if !m.display_name.trim().is_empty() {
            model_cfg.insert(
                "name".to_string(),
                serde_json::Value::String(m.display_name.trim().to_string()),
            );
        } else {
            model_cfg.remove("name");
        }
        if !m.family.trim().is_empty() {
            model_cfg.insert(
                "family".to_string(),
                serde_json::Value::String(m.family.trim().to_string()),
            );
        } else {
            model_cfg.remove("family");
        }
        for (key, value) in [
            ("reasoning", m.reasoning),
            ("tool_call", m.tool_call),
            ("temperature", m.temperature),
            ("attachment", m.attachment),
            ("interleaved", m.interleaved),
        ] {
            if let Some(value) = value {
                model_cfg.insert(key.to_string(), serde_json::Value::Bool(value));
            }
        }
        let mut modalities = model_cfg
            .get("modalities")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(input) = &m.input_modalities {
            modalities.insert(
                "input".to_string(),
                Value::Array(input.iter().map(|item| Value::String(item.clone())).collect()),
            );
        }
        if let Some(output) = &m.output_modalities {
            modalities.insert(
                "output".to_string(),
                Value::Array(output.iter().map(|item| Value::String(item.clone())).collect()),
            );
        }
        if modalities.is_empty() {
            model_cfg.remove("modalities");
        } else {
            model_cfg.insert("modalities".to_string(), Value::Object(modalities));
        }
        let mut limit = model_cfg.get("limit").and_then(Value::as_object).cloned().unwrap_or_default();
        update_limit_field(&mut limit, "context", &m.context_limit, false)?;
        update_limit_field(&mut limit, "input", &m.input_limit, false)?;
        update_limit_field(&mut limit, "output", &m.output_limit, true)?;
        if limit.is_empty() {
            model_cfg.remove("limit");
        } else {
            model_cfg.insert("limit".to_string(), Value::Object(limit));
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
    Ok((provider_id, serde_json::Value::Object(root)))
}

pub(crate) fn validate_model_drafts(
    models: &[crate::app::state::CustomProviderModelDraft],
) -> Result<(), String> {
    let mut ids = HashSet::new();
    for model in models {
        let model_id = model.model_id.trim();
        if model_id.is_empty() {
            continue;
        }
        if !ids.insert(model_id.to_string()) {
            return Err(format!("模型 ID 重复：{model_id}"));
        }
        let context = parse_limit_field(model_id, "上下文", &model.context_limit, false)?;
        let input = parse_limit_field(model_id, "输入", &model.input_limit, false)?;
        let output = parse_limit_field(model_id, "输出", &model.output_limit, true)?;
        if let (Some(context), Some(input)) = (context, input) {
            if input > context {
                return Err(format!("模型 {model_id} 的输入限制不能超过上下文限制"));
            }
        }
        if let (Some(context), Some(output)) = (context, output) {
            if output > context {
                return Err(format!("模型 {model_id} 的输出限制不能超过上下文限制"));
            }
        }
    }
    Ok(())
}

fn parse_limit_field(
    model_id: &str,
    label: &str,
    value: &str,
    allow_zero: bool,
) -> Result<Option<u64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("模型 {model_id} 的{label}限制必须是{}", if allow_zero { "非负整数" } else { "正整数" }))?;
    if !allow_zero && parsed == 0 {
        return Err(format!("模型 {model_id} 的{label}限制必须是正整数"));
    }
    Ok(Some(parsed))
}

fn update_limit_field(
    limit: &mut Map<String, Value>,
    key: &str,
    value: &str,
    allow_zero: bool,
) -> Result<(), String> {
    let parsed = parse_limit_field("当前模型", key, value, allow_zero)?;
    if let Some(value) = parsed {
        limit.insert(key.to_string(), Value::Number(value.into()));
    } else {
        limit.remove(key);
    }
    Ok(())
}
