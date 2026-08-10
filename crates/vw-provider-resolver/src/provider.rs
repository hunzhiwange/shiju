//! Provider 与模型信息的聚合查询入口。
//!
//! 本模块是 crate 的核心入口，负责把以下来源的信息合并成统一视图：
//! - 本地模型基线元数据
//! - 用户配置中的 provider 与 model 覆盖项
//! - 本地认证状态与环境变量可用性
//! - wasm 环境下网关返回的 provider 列表
//!
//! 最终对外暴露 provider 列表、模型查询、默认模型推导与缓存失效能力。

use crate::cache;
use crate::config;
use crate::models;
use once_cell::sync::Lazy;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::UNIX_EPOCH;

#[cfg(not(target_arch = "wasm32"))]
use crate::auth;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;

pub use vw_shared::provider::state::{
    State, as_string_map, builtin_model_limit, default_model_capabilities, from_models_dev_model,
    from_models_dev_provider, normalize_adapter, resolve_model_capabilities, resolve_model_limit,
};
pub use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelCostOver200k, ModelLimit, ModelNotFoundError, ParsedModelRef,
    ProviderSource, default_adapter,
};
pub use vw_shared::provider::types::{parse_model, sort};

/// 启用过滤后的 provider 状态缓存 key。
const CACHE_KEY_FILTERED: &str = "provider:filtered";
/// 设置页可见的 provider 状态缓存 key。
const CACHE_KEY_SETTINGS: &str = "provider:settings";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ConfigFingerprint {
    path: String,
    exists: bool,
    modified_nanos: u128,
    len: u64,
}

static CONFIG_FINGERPRINT: Lazy<Mutex<Option<ConfigFingerprint>>> = Lazy::new(|| Mutex::new(None));

#[cfg(not(target_arch = "wasm32"))]
/// 抓取当前环境变量快照，供 provider 能力判断使用。
fn snapshot_env() -> HashMap<String, String> {
    std::env::vars_os()
        .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
        .collect()
}

/// 返回面向实际使用场景过滤后的 provider 状态。
///
/// 该状态会过滤掉不可用的模型，更适合正常对话或推理流程使用。
async fn state_filtered() -> Arc<State> {
    invalidate_cache_if_config_changed().await;
    cache::get_or_init(CACHE_KEY_FILTERED, || async { Arc::new(load_state_impl(true).await) }).await
}

/// 返回设置页使用的 provider 状态，不应用启用过滤。
///
/// 设置页通常需要看到更完整的信息，因此这里保留未启用的模型项。
async fn state_for_settings() -> Arc<State> {
    invalidate_cache_if_config_changed().await;
    cache::get_or_init(CACHE_KEY_SETTINGS, || async { Arc::new(load_state_impl(false).await) })
        .await
}

#[cfg(not(target_arch = "wasm32"))]
fn read_config_fingerprint() -> ConfigFingerprint {
    let Some(path) = config::config_path() else {
        return ConfigFingerprint::default();
    };

    let mut fingerprint = ConfigFingerprint {
        path: path.to_string_lossy().to_string(),
        exists: false,
        modified_nanos: 0,
        len: 0,
    };

    let Ok(metadata) = std::fs::metadata(&path) else {
        return fingerprint;
    };

    fingerprint.exists = true;
    fingerprint.len = metadata.len();
    fingerprint.modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    fingerprint
}

#[cfg(target_arch = "wasm32")]
fn read_config_fingerprint() -> ConfigFingerprint {
    ConfigFingerprint::default()
}

async fn invalidate_cache_if_config_changed() {
    let next = read_config_fingerprint();
    let changed = {
        let mut guard = CONFIG_FINGERPRINT.lock().unwrap_or_else(|e| e.into_inner());
        let changed = guard.as_ref().is_some_and(|current| current != &next);
        *guard = Some(next);
        changed
    };

    if changed {
        invalidate_cache().await;
    }
}

/// 列出当前可用 provider。
///
/// # 返回值
///
/// 返回已过滤后的 provider 映射，key 为 provider_id。
pub async fn list() -> HashMap<String, Info> {
    state_filtered().await.providers.clone()
}

/// 列出设置页需要展示的全部 provider。
pub async fn list_for_settings() -> HashMap<String, Info> {
    state_for_settings().await.providers.clone()
}

/// 使 provider 聚合缓存失效。
///
/// 适用于配置变更、认证状态变化或模型元数据刷新之后。
pub async fn invalidate_cache() {
    if let Ok(mut guard) = CONFIG_FINGERPRINT.lock() {
        *guard = None;
    }
    cache::invalidate(CACHE_KEY_FILTERED).await;
    cache::invalidate(CACHE_KEY_SETTINGS).await;
}

/// 按 provider_id 获取 provider 信息。
///
/// # 参数
///
/// * `provider_id` - provider 的稳定标识
pub async fn get_provider(provider_id: &str) -> Option<Info> {
    state_filtered().await.providers.get(provider_id).cloned()
}

/// 按 provider_id 与 model_id 获取模型信息，并提供有限的模糊建议。
///
/// 当 provider_id 或 model_id 不存在时，会返回带建议项的 `ModelNotFoundError`，
/// 以改善命令行与 UI 场景下的可诊断性。
pub async fn get_model(provider_id: &str, model_id: &str) -> Result<Model, ModelNotFoundError> {
    let state = state_filtered().await;

    let Some(provider) = state.providers.get(provider_id) else {
        let suggestions =
            suggest(provider_id, state.providers.keys().map(|s| s.as_str()).collect());
        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    let Some(model) = provider.models.get(model_id).cloned() else {
        let alias_matches =
            provider.models.values().filter(|m| m.api.id == model_id).cloned().collect::<Vec<_>>();

        if alias_matches.len() == 1 {
            return Ok(alias_matches[0].clone());
        }

        let suggestions = if alias_matches.len() > 1 {
            let mut ids = alias_matches.into_iter().map(|m| m.id).collect::<Vec<_>>();
            ids.sort();
            ids.truncate(3);
            ids
        } else {
            suggest(model_id, provider.models.keys().map(|s| s.as_str()).collect())
        };

        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    Ok(model)
}

/// 返回默认模型；若配置未指定，则选择排序后的首个 provider 与模型。
///
/// 默认选择逻辑保持确定性：先按 provider_id 排序，再按模型排序规则选择首项。
pub async fn default_model() -> Result<ParsedModelRef, String> {
    if let Some(m) = config::read_default_model().await.as_deref() {
        return Ok(parse_model(m));
    }

    let providers = list().await;
    let mut candidates = providers.values().collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));

    let Some(p) = candidates.first() else {
        return Err("没有可用的 provider".to_string());
    };

    let mut model_list = p.models.values().cloned().collect::<Vec<_>>();
    model_list = sort(model_list);

    let Some(m) = model_list.first() else {
        return Err("没有可用的模型".to_string());
    };

    Ok(ParsedModelRef { provider_id: p.id.clone(), model_id: m.id.clone() })
}

/// 对 provider 或模型 id 给出最多三个相似候选。
///
/// 当前使用 Jaro-Winkler 相似度，阈值为 `0.70`。
fn suggest(query: &str, candidates: Vec<&str>) -> Vec<String> {
    let mut scored = candidates
        .into_iter()
        .map(|c| (c, strsim::jaro_winkler(query, c)))
        .filter(|(_, s)| *s > 0.70)
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(3).map(|(c, _)| c.to_string()).collect()
}

#[cfg(target_arch = "wasm32")]
/// 通过网关加载 provider 状态。
///
/// wasm 环境不直接访问本地配置与认证文件，因此改由网关提供聚合结果。
async fn load_state_via_gateway(apply_enabled_filter: bool) -> State {
    let Ok(client) = config::gateway_client() else {
        return State { providers: HashMap::new() };
    };
    let Ok(response) = client.provider_list(None).await else {
        return State { providers: HashMap::new() };
    };

    let providers = response
        .all
        .into_iter()
        .filter_map(|mut provider| {
            if apply_enabled_filter {
                provider.models.retain(|_, model| model.status == "active");
            }
            if provider.models.is_empty() {
                return None;
            }
            Some((provider.id.clone(), provider))
        })
        .collect();

    State { providers }
}

fn unique_provider_api(models: &HashMap<String, Model>) -> Option<(String, String)> {
    let mut api_candidates = models.values().filter_map(|model| {
        let url = model.api.url.trim();
        (!url.is_empty()).then(|| (url.to_string(), model.api.adapter.clone()))
    });
    let first = api_candidates.next()?;
    api_candidates.all(|candidate| candidate == first).then_some(first)
}

/// 加载并合并内置模型数据、用户配置覆盖与运行时启用状态。
///
/// 非 wasm 环境下的大致流程如下：
/// 1. 读取模型基线元数据
/// 2. 将所有模型初始标记为 `disabled`
/// 3. 合并配置文件中的 provider 覆盖项
/// 4. 根据认证信息、环境变量与配置推导启用状态
/// 5. 视调用场景决定是否过滤不可用模型
async fn load_state_impl(apply_enabled_filter: bool) -> State {
    #[cfg(target_arch = "wasm32")]
    {
        load_state_via_gateway(apply_enabled_filter).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        models::init();
        let _cfg = config::get().await;
        let models_dev = models::get().await;

        // 先基于模型基线元数据构建 provider 集合。
        let mut database = models_dev
            .into_iter()
            .map(|(k, v)| (k, from_models_dev_provider(v)))
            .collect::<HashMap<_, _>>();

        for provider in database.values_mut() {
            for model in provider.models.values_mut() {
                model.status = "disabled".to_string();
            }
        }

        let mut providers: HashMap<String, Info> = HashMap::new();
        let config_providers: Vec<(String, Value)> = config::load_provider_overrides().await;

        // 再将用户配置覆盖到基线上，生成最终 provider 视图。
        for (provider_id, provider) in config_providers.iter() {
            let existing = database.get(provider_id).cloned();

            if existing.is_none() {
                tracing::info!(provider_id, "provider_base_missing");
            }

            let name = provider
                .get("name")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
                .or_else(|| existing.as_ref().map(|p| p.name.clone()))
                .unwrap_or_else(|| provider_id.to_string());

            let env_arr = provider
                .get("env")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
                })
                .or_else(|| existing.as_ref().map(|p| p.env.clone()))
                .unwrap_or_default();

            let mut options = existing.as_ref().map(|p| p.options.clone()).unwrap_or_default();
            if let Some(o) = provider.get("options").and_then(Value::as_object) {
                for (k, v) in o {
                    options.insert(k.clone(), v.clone());
                }
            }

            let mut provider_models =
                existing.as_ref().map(|p| p.models.clone()).unwrap_or_default();
            let inherited_api = unique_provider_api(&provider_models);

            if let Some(models_cfg) = provider.get("models").and_then(Value::as_object) {
                for (model_id, model_cfg) in models_cfg {
                    let base = provider_models.get(model_id).cloned().or_else(|| {
                        model_cfg
                            .get("id")
                            .and_then(Value::as_str)
                            .and_then(|id| provider_models.get(id).cloned())
                    });

                    let api_id = model_cfg
                        .get("id")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| base.as_ref().map(|m| m.api.id.clone()))
                        .unwrap_or_else(|| model_id.to_string());

                    let api_url = provider
                        .get("api")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| base.as_ref().map(|m| m.api.url.clone()))
                        .or_else(|| inherited_api.as_ref().map(|(url, _)| url.clone()))
                        .unwrap_or_default();

                    let api_adapter = model_cfg
                        .get("provider")
                        .and_then(|p| p.get("adapter"))
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| {
                            provider.get("adapter").and_then(Value::as_str).map(|s| s.to_string())
                        })
                        .or_else(|| base.as_ref().map(|m| m.api.adapter.clone()))
                        .or_else(|| inherited_api.as_ref().map(|(_, adapter)| adapter.clone()))
                        .unwrap_or_else(default_adapter);
                    let api_adapter = normalize_adapter(&api_adapter);

                    let status = model_cfg
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "disabled".to_string());

                    let name = model_cfg
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| base.as_ref().map(|m| m.name.clone()))
                        .unwrap_or_else(|| model_id.to_string());

                    let family = model_cfg
                        .get("family")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| base.as_ref().and_then(|m| m.family.clone()));

                    let release_date = model_cfg
                        .get("release_date")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                        .or_else(|| base.as_ref().map(|m| m.release_date.clone()))
                        .unwrap_or_default();

                    let headers = model_cfg
                        .get("headers")
                        .map(as_string_map)
                        .or_else(|| base.as_ref().map(|m| m.headers.clone()))
                        .unwrap_or_default();

                    let mut options_model =
                        base.as_ref().map(|m| m.options.clone()).unwrap_or_default();
                    if let Some(o) = model_cfg.get("options").and_then(Value::as_object) {
                        for (k, v) in o {
                            options_model.insert(k.clone(), v.clone());
                        }
                    }

                    let config_limit = model_cfg.get("limit");
                    let configured_context = config_limit
                        .and_then(|limit| limit.get("context"))
                        .and_then(Value::as_u64)
                        .filter(|value| *value > 0);
                    let configured_input = config_limit
                        .and_then(|limit| limit.get("input"))
                        .and_then(Value::as_u64)
                        .filter(|value| *value > 0);
                    let configured_output = config_limit
                        .and_then(|limit| limit.get("output"))
                        .and_then(Value::as_u64);
                    let limit = vw_shared::provider::state::resolve_model_limit_with_base(
                        configured_context,
                        configured_input,
                        configured_output,
                        base.as_ref().map(|model| &model.limit),
                    );

                    let cap = resolve_model_capabilities(
                        model_cfg,
                        base.as_ref().map(|model| &model.capabilities),
                    );

                    let cost = base.as_ref().map(|m| m.cost.clone()).unwrap_or(ModelCost {
                        input: 0.0,
                        output: 0.0,
                        cache: ModelCostCache { read: 0.0, write: 0.0 },
                        experimental_over_200k: None,
                    });

                    let model = Model {
                        id: model_id.to_string(),
                        provider_id: provider_id.to_string(),
                        api: ApiInfo { id: api_id, url: api_url, adapter: api_adapter },
                        name,
                        family,
                        capabilities: cap,
                        cost,
                        limit,
                        status,
                        options: options_model,
                        headers,
                        release_date,
                        variants: HashMap::new(),
                    };
                    provider_models.insert(model_id.to_string(), model);
                }
            }

            database.insert(
                provider_id.to_string(),
                Info {
                    id: provider_id.to_string(),
                    name,
                    source: ProviderSource::Config,
                    env: env_arr,
                    key: None,
                    options,
                    models: provider_models,
                },
            );
        }

        let env_vars = snapshot_env();
        for (_provider_id, provider) in database.iter_mut() {
            let key = provider
                .env
                .iter()
                .filter_map(|k| env_vars.get(k).cloned())
                .find(|v| !v.is_empty());
            if let Some(k) = key {
                provider.source = ProviderSource::Env;
                if provider.env.len() == 1 {
                    provider.key = Some(k);
                }
            }
        }

        for (provider_id, info) in auth::all() {
            let (next_source, next_key) = match info {
                auth::Info::Api(api) => (ProviderSource::Api, Some(api.key)),
                auth::Info::Oauth(_) => {
                    (ProviderSource::Api, Some(vw_shared::auth::OAUTH_DUMMY_KEY.to_string()))
                }
                auth::Info::Wellknown(_) => {
                    (ProviderSource::Api, Some(vw_shared::auth::OAUTH_DUMMY_KEY.to_string()))
                }
            };

            if let Some(p) = database.get_mut(&provider_id) {
                p.source = next_source;
                if p.key.as_deref().unwrap_or_default().trim().is_empty() {
                    p.key = next_key;
                }
            }
        }

        for (provider_id, provider) in database.iter_mut() {
            let configured = config_providers
                .iter()
                .find(|(configured_id, _)| configured_id == provider_id)
                .map(|(_, value)| value);
            let configured_api = configured
                .and_then(|value| value.get("api"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let explicit_model_ids = configured
                .and_then(|value| value.get("models"))
                .and_then(Value::as_object)
                .map(|models| models.keys().cloned().collect::<HashSet<_>>())
                .unwrap_or_default();
            let bearer = provider
                .key
                .as_deref()
                .filter(|key| crate::remote_models::is_real_bearer(key))
                .map(str::to_string)
                .or_else(|| {
                    provider
                        .env
                        .iter()
                        .filter_map(|name| env_vars.get(name))
                        .find(|key| crate::remote_models::is_real_bearer(key))
                        .cloned()
                });
            let Some(bearer) = bearer else {
                continue;
            };

            if let Err(error) = crate::remote_models::discover_and_merge_provider(
                provider,
                configured_api.as_deref(),
                &explicit_model_ids,
                &bearer,
            )
            .await
            {
                tracing::warn!(
                    provider_id,
                    category = error.category(),
                    "remote_model_discovery_failed"
                );
            }
        }

        for (provider_id, provider) in database.into_iter() {
            let mut provider = provider;

            if apply_enabled_filter {
                provider.models.retain(|_, m| m.status == "active");
            }

            if provider.models.is_empty() {
                continue;
            }

            providers.insert(provider_id.clone(), provider);
        }

        State { providers }
    }
}

pub async fn get_small_model(provider_id: &str) -> Option<Model> {
    let state = state_filtered().await;
    let provider = state.providers.get(provider_id)?;

    let mut priority = vec![
        "claude-haiku-4-5",
        "claude-haiku-4.5",
        "3-5-haiku",
        "3.5-haiku",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gpt-5-nano",
    ];

    if provider_id.starts_with("vibewindow") {
        priority = vec!["gpt-5-nano"];
    }

    if provider_id.starts_with("github-copilot") {
        priority.insert(0, "gpt-5-mini");
        priority.insert(1, "claude-haiku-4.5");
    }

    for item in priority {
        for (id, model) in &provider.models {
            if id.contains(item) {
                return Some(model.clone());
            }
        }
    }

    None
}

pub fn init() {
    models::init();
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
