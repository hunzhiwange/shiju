//! 原生模型目录、环境、认证与发现的顺序组装。
use crate::{State, Info, ProviderSource, from_models_dev_provider};
use provider_config_reader as config;
use provider_model_cache as models;
use provider_auth_reader as auth;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// 抓取当前环境变量快照，供 provider 能力判断使用。
fn snapshot_env() -> HashMap<String, String> {
    std::env::vars_os()
        .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
        .collect()
}


/// 加载并合并内置模型数据、用户配置覆盖与运行时启用状态。
///
/// 非 wasm 环境下的大致流程如下：
/// 1. 读取模型基线元数据
/// 2. 将所有模型初始标记为 `disabled`
/// 3. 合并配置文件中的 provider 覆盖项
/// 4. 根据认证信息、环境变量与配置推导启用状态
/// 5. 视调用场景决定是否过滤不可用模型
pub(crate) async fn load_state_impl(apply_enabled_filter: bool) -> State {
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

    crate::overrides::apply_overrides(&mut database, &config_providers);

    let env_vars = snapshot_env();
    for provider in database.values_mut() {
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
            .filter(|key| provider_model_discovery::is_real_bearer(key))
            .map(str::to_string)
            .or_else(|| {
                provider
                    .env
                    .iter()
                    .filter_map(|name| env_vars.get(name))
                    .find(|key| provider_model_discovery::is_real_bearer(key))
                    .cloned()
            });
        let Some(bearer) = bearer else {
            continue;
        };

        if let Err(error) = provider_model_discovery::discover_and_merge_provider(
            provider,
            configured_api.as_deref(),
            &explicit_model_ids,
            &bearer,
        )
        .await
        {
            tracing::debug!(
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

        if apply_enabled_filter && provider.models.is_empty() {
            continue;
        }

        providers.insert(provider_id.clone(), provider);
    }

    State { providers }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
