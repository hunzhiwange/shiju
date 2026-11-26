//! 远端模型与静态目录的合并。

use std::collections::{BTreeSet, HashMap, HashSet};
use model_types::{ApiInfo, Info, Model, ModelCost, ModelCostCache, ModelLimit, default_adapter};
use serde_json::{Map, Value};
use crate::{DiscoveryError, discover_openai_compatible_models, is_real_bearer};
use crate::http::{DiscoveryLimits, ModelEntry, discover_openai_compatible_entries, model_id};

#[path = "shiju_metadata.rs"]
mod shiju_metadata;

const SHIJU_API_BASE: &str = "https://router.shiju.cloud/v1";

/// 选择配置 API 基址；未配置时仅接受唯一明确的兼容模型部署。
pub fn resolve_api_base(provider: &Info, configured_api: Option<&str>) -> Option<String> {
    if let Some(api) = configured_api {
        let api = api.trim().trim_end_matches('/');
        return (!api.is_empty()).then(|| api.to_string());
    }
    let candidates: BTreeSet<_> = provider
        .models
        .values()
        .filter(|model| {
            matches!(model.api.adapter.as_str(), "openai-compatible" | "openai-responses")
        })
        .map(|model| model.api.url.trim().trim_end_matches('/'))
        .filter(|url| !url.is_empty())
        .collect();
    // Multiple model deployments are not a provider-wide discovery authorization.
    (candidates.len() == 1).then(|| candidates.first().unwrap().to_string())
}

/// 合并远端模型；显式配置状态优先，静态元数据保持不变。
pub fn merge_remote_models(
    provider: &mut Info,
    api_base: &str,
    explicit_model_ids: &HashSet<String>,
    remote_ids: Vec<String>,
    catalog: &HashMap<String, provider_model_catalog::Provider>,
) -> usize {
    merge_remote_model_ids(provider, api_base, remote_ids, catalog, |id| {
        explicit_model_ids.contains(id)
    }).len()
}

fn merge_remote_model_ids(
    provider: &mut Info,
    api_base: &str,
    remote_ids: Vec<String>,
    catalog: &HashMap<String, provider_model_catalog::Provider>,
    is_configured: impl Fn(&str) -> bool,
) -> HashSet<String> {
    let mut changed = HashSet::new();
    for remote_id in remote_ids {
        if let Some(existing_id) = existing_model_id(provider, &remote_id) {
            if is_configured(existing_id) {
                continue;
            }
            let existing_id = existing_id.to_owned();
            if let Some(model) = provider.models.get_mut(&existing_id)
                && model.status != "active"
            {
                model.status = "active".to_string();
                changed.insert(existing_id);
            }
            continue;
        }

        let mut model = safe_remote_model(&provider.id, api_base, remote_id.clone());
        if let Some(raw_provider) = catalog.get(&provider.id)
            && let Some(metadata) = raw_provider.models.values().find(|model| model.id == remote_id)
        {
            let matched = model_state::from_models_dev_model(raw_provider, metadata);
            if matched.api.url.trim_end_matches('/') == api_base.trim_end_matches('/') {
                model = matched;
                model.status = "active".to_string();
            }
        }
        if provider.id == "shiju"
            && let Some(metadata) = provider_model_catalog::shiju_model_metadata(catalog, &remote_id)
        {
            // Only model metadata crosses providers. Routing, credentials, prices,
            // headers and provider-specific request options stay on Shiju.
            let raw_provider = provider_model_catalog::Provider {
                protocols: None,
                npm: None,
                id: provider.id.clone(),
                api: Some(api_base.to_string()),
                adapter: Some(default_adapter()),
                name: provider.name.clone(),
                env: Vec::new(),
                models: HashMap::new(),
            };
            let matched = model_state::from_models_dev_model(&raw_provider, metadata);
            model.name = matched.name;
            model.family = matched.family;
            model.capabilities = matched.capabilities;
            model.limit = matched.limit;
            model.release_date = matched.release_date;
        }
        changed.insert(remote_id.clone());
        provider.models.insert(remote_id, model);
    }
    changed
}

fn existing_model_id<'a>(provider: &'a Info, remote_id: &str) -> Option<&'a str> {
    provider.models.get_key_value(remote_id).map(|(id, _)| id.as_str()).or_else(|| {
        provider.models.iter()
            .filter(|(_, model)| model.api.id == remote_id)
            .map(|(id, _)| id.as_str())
            .min()
    })
}

/// 发现并合并单个 provider；无真实凭据或 API 基址时直接跳过。
pub async fn discover_and_merge_provider(
    provider: &mut Info,
    configured_api: Option<&str>,
    configured_models: Option<&Map<String, Value>>,
    bearer: &str,
) -> Result<usize, DiscoveryError> {
    if !is_real_bearer(bearer) {
        return Ok(0);
    }
    if provider.models.values().any(|model| {
        !matches!(model.api.adapter.as_str(), "openai-compatible" | "openai-responses")
    }) {
        // Dedicated authentication and SDK discovery paths must not receive guessed Bearer requests.
        return Ok(0);
    }
    let Some(api_base) = resolve_api_base(provider, configured_api) else {
        return Ok(0);
    };
    if provider.id == "shiju" && api_base == SHIJU_API_BASE {
        let entries =
            discover_openai_compatible_entries(&api_base, Some(bearer), DiscoveryLimits::default())
                .await?;
        let catalog = provider_model_cache::get().await;
        return merge_shiju_models(provider, configured_models, &entries, &catalog);
    }

    let remote_ids = discover_openai_compatible_models(&api_base, Some(bearer)).await?;
    let catalog = provider_model_cache::get().await;
    Ok(merge_remote_model_ids(provider, &api_base, remote_ids, &catalog, |id| {
        configured_models.is_some_and(|models| models.contains_key(id))
    }).len())
}

pub(crate) fn merge_shiju_models(
    provider: &mut Info,
    configured_models: Option<&Map<String, Value>>,
    entries: &[ModelEntry],
    catalog: &HashMap<String, provider_model_catalog::Provider>,
) -> Result<usize, DiscoveryError> {
    let mut unique = HashMap::new();
    for entry in entries {
        let id = model_id(entry.id.as_deref())
            .ok_or_else(|| DiscoveryError::new("invalid_response"))?;
        // Conflicting rows still establish an ID, but supply no metadata.
        unique.entry(id).and_modify(|entry| *entry = None).or_insert(Some(entry));
    }
    let previous_count = provider.models.len();
    provider.models.retain(|_, model| {
        model.api.url.trim_end_matches('/') != SHIJU_API_BASE
            || unique.contains_key(model.api.id.as_str())
    });
    let removed_count = previous_count - provider.models.len();
    let remote_ids = unique.keys().map(|id| (*id).to_owned()).collect();
    let mut changed = merge_remote_model_ids(provider, SHIJU_API_BASE, remote_ids, catalog, |id| {
        configured_models.is_some_and(|models| models.contains_key(id))
    });
    for (remote_id, entry) in unique {
        let Some(entry) = entry else { continue };
        let Some(id) = existing_model_id(provider, remote_id).map(str::to_owned) else { continue };
        let model = provider.models.get_mut(&id).expect("resolved model exists");
        if model.api.id == remote_id && model.api.url.trim_end_matches('/') == SHIJU_API_BASE
            && shiju_metadata::apply(model, entry, configured_models.and_then(|models| models.get(&id)))
        {
            changed.insert(id);
        }
    }
    Ok(removed_count + changed.len())
}

fn safe_remote_model(provider_id: &str, api_base: &str, model_id: String) -> Model {
    Model {
        id: model_id.clone(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            protocols: None,
            id: model_id.clone(),
            url: api_base.trim_end_matches('/').to_string(),
            adapter: default_adapter(),
        },
        name: model_id,
        family: None,
        capabilities: model_state::default_model_capabilities(),
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache: ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 0, input: None, output: 0 },
        status: "active".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: String::new(),
        variants: HashMap::new(),
    }
}
