//! 远端模型与静态目录的合并。

use std::collections::{BTreeSet, HashMap, HashSet};
use model_types::{ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost, ModelCostCache, ModelLimit, default_adapter};
use crate::{DiscoveryError, discover_openai_compatible_models, is_real_bearer};

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
    let mut changed = 0;
    for remote_id in remote_ids {
        let existing_id = if provider.models.contains_key(&remote_id) {
            Some(remote_id.clone())
        } else {
            provider
                .models
                .iter()
                .filter(|(_, model)| model.api.id == remote_id)
                .map(|(id, _)| id.clone())
                .min()
        };

        if let Some(existing_id) = existing_id {
            if explicit_model_ids.contains(&existing_id) {
                continue;
            }
            if let Some(model) = provider.models.get_mut(&existing_id)
                && model.status != "active"
            {
                model.status = "active".to_string();
                changed += 1;
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
        provider.models.insert(remote_id, model);
        changed += 1;
    }
    changed
}

/// 发现并合并单个 provider；无真实凭据或 API 基址时直接跳过。
pub async fn discover_and_merge_provider(
    provider: &mut Info,
    configured_api: Option<&str>,
    explicit_model_ids: &HashSet<String>,
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
    let remote_ids = discover_openai_compatible_models(&api_base, bearer).await?;
    let catalog = provider_model_cache::get().await;
    Ok(merge_remote_models(provider, &api_base, explicit_model_ids, remote_ids, &catalog))
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
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: false,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(false),
        },
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
