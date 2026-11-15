//! 将配置覆盖到模型基线。
use crate::{Info, Model, ApiInfo, ProviderSource, ModelCost, ModelCostCache, default_adapter, as_string_map, resolve_model_capabilities};
use serde_json::Value;
use std::collections::HashMap;

fn unique_provider_api(models: &HashMap<String, Model>) -> Option<(String, String)> {
    let mut api_candidates = models.values().filter_map(|model| {
        let url = model.api.url.trim();
        (!url.is_empty()).then(|| (url.to_string(), model.api.adapter.clone()))
    });
    let first = api_candidates.next()?;
    api_candidates.all(|candidate| candidate == first).then_some(first)
}


pub(crate) fn apply_overrides(database: &mut HashMap<String, Info>, config_providers: &[(String, Value)]) {
    let reasoning_model_ids = vw_shared::provider::state::catalog_reasoning_model_ids(database);

    // 再将用户配置覆盖到基线上，生成最终 provider 视图。
    for (provider_id, provider) in config_providers.iter() {
        let existing = database.get(provider_id).cloned();

        if existing.is_none() {
            tracing::debug!(provider_id, "provider_base_missing");
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

                let mut api =
                    base.as_ref().map(|model| model.api.clone()).unwrap_or_else(|| ApiInfo {
                        id: api_id.clone(),
                        url: inherited_api
                            .as_ref()
                            .map(|(url, _)| url.clone())
                            .unwrap_or_default(),
                        adapter: inherited_api
                            .as_ref()
                            .map(|(_, adapter)| adapter.clone())
                            .unwrap_or_else(default_adapter),
                        protocols: None,
                    });
                if api.id != api_id {
                    api.protocols = None;
                }
                api.id = api_id;

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
                let configured_output =
                    config_limit.and_then(|limit| limit.get("output")).and_then(Value::as_u64);
                let limit = vw_shared::provider::state::resolve_model_limit_with_base(
                    configured_context,
                    configured_input,
                    configured_output,
                    base.as_ref().map(|model| &model.limit),
                );

                let cap = resolve_model_capabilities(
                    model_cfg,
                    base.as_ref().map(|model| &model.capabilities),
                    reasoning_model_ids.contains(&api.id),
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
                    api,
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

        for (id, model) in provider_models.iter_mut() {
            let configured = provider
                .get("models")
                .and_then(Value::as_object)
                .and_then(|models| models.get(id));
            vw_shared::provider::state::apply_api_overrides(
                &mut model.api,
                provider,
                configured,
            );
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
}

#[cfg(test)]
#[path = "overrides_tests.rs"]
mod tests;
