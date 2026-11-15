use super::*;
use serde_json::json;
use std::collections::HashMap;
use model_types::{ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost, ModelCostCache, ModelLimit, ProviderSource};
fn model(provider_id: &str, id: &str, name: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            protocols: None,
            id: id.to_string(),
            url: "https://example.test".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: name.to_string(),
        family: None,
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: true },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Field { field: "content".to_string() },
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.5, write: 0.75 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 200_000, input: Some(100_000), output: 20_000 },
        status: status.to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn provider(
    id: &str,
    name: &str,
    source: ProviderSource,
    key: Option<&str>,
    models: Vec<Model>,
) -> Info {
    Info {
        id: id.to_string(),
        name: name.to_string(),
        source,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: models.into_iter().map(|model| (model.id.clone(), model)).collect(),
    }
}

#[test]
fn summarize_models_filters_disconnected_providers_and_sorts_providers_and_models() {
    let providers = HashMap::from([
        (
            "disconnected".to_string(),
            provider("hidden", "First", ProviderSource::Custom, Some("  "), Vec::new()),
        ),
        (
            "z-provider".to_string(),
            provider(
                "z-provider",
                "Same",
                ProviderSource::Api,
                None,
                vec![
                    model("z-provider", "ordinary", "Ordinary", "disabled"),
                    model("z-provider", "gpt-5-latest", "GPT-5 Latest", "active"),
                ],
            ),
        ),
        (
            "a-provider".to_string(),
            provider(
                "a-provider",
                "Same",
                ProviderSource::Custom,
                Some("configured"),
                vec![model("a-provider", "custom", "Custom", "stable")],
            ),
        ),
    ]);

    let summaries = summarize_models(providers);

    assert_eq!(
        summaries.iter().map(|provider| provider.id.as_str()).collect::<Vec<_>>(),
        vec!["a-provider", "z-provider"]
    );
    assert_eq!(
        summaries[1].models.iter().map(|model| model.id.as_str()).collect::<Vec<_>>(),
        vec!["gpt-5-latest", "ordinary"]
    );
}

#[test]
fn summarize_models_maps_status_capabilities_limits_and_serialized_detail() {
    let providers = HashMap::from([(
        "provider".to_string(),
        provider(
            "provider",
            "Provider",
            ProviderSource::Config,
            None,
            vec![model("provider", "model", "Model", "active")],
        ),
    )]);

    let summaries = summarize_models(providers);
    let summary = &summaries[0].models[0];

    assert_eq!(summary.name, "Model");
    assert!(summary.enabled);
    assert!(summary.toolcall);
    assert!(summary.attachment);
    assert_eq!(summary.context_limit, 200_000);
    assert_eq!(summary.detail["providerID"], "provider");
    assert_eq!(summary.detail["limit"]["input"], 100_000);
}

#[test]
fn model_status_patch_groups_all_changes_into_one_provider_patch() {
    let changes = vec![
        ModelStatusChange {
            provider_id: "openai".to_string(),
            model_id: "gpt-5".to_string(),
            previous_enabled: true,
            enabled: false,
        },
        ModelStatusChange {
            provider_id: "openai".to_string(),
            model_id: "gpt-4.1".to_string(),
            previous_enabled: false,
            enabled: true,
        },
        ModelStatusChange {
            provider_id: "anthropic".to_string(),
            model_id: "claude-sonnet".to_string(),
            previous_enabled: true,
            enabled: false,
        },
    ];

    assert_eq!(
        model_status_patch(&changes),
        json!({
            "providers": {
                "openai": {
                    "models": {
                        "gpt-5": { "id": "gpt-5", "status": "disabled" },
                        "gpt-4.1": { "id": "gpt-4.1", "status": "active" }
                    }
                },
                "anthropic": {
                    "models": {
                        "claude-sonnet": {
                            "id": "claude-sonnet",
                            "status": "disabled"
                        }
                    }
                }
            }
        })
    );
}

#[test]
fn provider_status_reset_patch_removes_only_explicit_status_fields() {
    let mut config = config_aggregate::Config::default();
    config.providers.insert(
        "openai".to_string(),
        json!({
            "name": "OpenAI",
            "models": {
                "gpt-5": {
                    "status": "active",
                    "name": "GPT-5"
                },
                "gpt-4.1": {
                    "status": "disabled"
                },
                "gpt-4o": {
                    "name": "GPT-4o"
                }
            }
        }),
    );

    assert_eq!(
        provider_model_status_reset_patch(&config, "openai"),
        Some(json!({
            "providers": {
                "openai": {
                    "models": {
                        "gpt-5": { "status": null },
                        "gpt-4.1": { "status": null }
                    }
                }
            }
        }))
    );
    assert!(provider_model_status_reset_patch(&config, "missing").is_none());
}
