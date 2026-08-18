use super::*;
use crate::app::state::{CustomProviderDraft, CustomProviderModelDraft, ProviderHeaderDraft};
use serde_json::{Value, json};
use std::collections::HashSet;

#[test]
fn discovered_models_replace_blank_placeholder_and_preserve_existing_names() {
    let mut existing = vec![
        CustomProviderModelDraft::default(),
        CustomProviderModelDraft {
            model_id: "model-b".to_string(),
            display_name: "自定义名称".to_string(),
            ..Default::default()
        },
    ];

    merge_discovered_model_drafts(
        &mut existing,
        vec!["model-c".to_string(), "model-a".to_string(), "model-b".to_string()],
    );

    assert_eq!(
        existing.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        vec!["model-a", "model-b", "model-c"]
    );
    assert_eq!(existing[1].display_name, "自定义名称");
}

#[test]
fn discovered_models_ignore_blank_and_duplicate_ids() {
    let mut existing = vec![CustomProviderModelDraft {
        model_id: "model-a".to_string(),
        display_name: String::new(),
        ..Default::default()
    }];

    merge_discovered_model_drafts(
        &mut existing,
        vec![
            " ".to_string(),
            "model-a".to_string(),
            " model-b ".to_string(),
            "model-b".to_string(),
        ],
    );

    assert_eq!(
        existing.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        vec!["model-a", "model-b"]
    );
}

#[test]
fn custom_provider_draft_rejects_non_object_configs() {
    for raw in [json!(null), json!(false), json!(42), json!("provider"), json!([])] {
        assert_eq!(
            custom_provider_draft_from_value("invalid", &raw).unwrap_err(),
            "提供商 invalid 的配置格式无效"
        );
    }
}

#[test]
fn custom_provider_draft_copies_fields_and_sorts_models() {
    let raw = json!({
        "name": "Custom Provider",
        "api": "https://api.example.test/v1",
        "models": {
            "z-model": {"name": "Zed"},
            "invalid-config": "not an object",
            "a-model": {"name": "Alpha"},
            "unnamed-model": {"name": 42}
        }
    });

    let draft = custom_provider_draft_from_value("custom-provider", &raw).unwrap();

    assert_eq!(draft.provider_id, "custom-provider");
    assert_eq!(draft.display_name, "Custom Provider");
    assert_eq!(draft.base_url, "https://api.example.test/v1");
    assert_eq!(
        draft.models.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        vec!["a-model", "invalid-config", "unnamed-model", "z-model"]
    );
    assert_eq!(
        draft.models.iter().map(|model| model.display_name.as_str()).collect::<Vec<_>>(),
        vec!["Alpha", "", "", "Zed"]
    );
}

#[test]
fn custom_provider_draft_restores_model_capabilities_limits_modalities_and_family() {
    let raw = json!({
        "models": {
            "vision-model": {
                "id": "vendor-vision",
                "name": "Vision",
                "family": "vendor-family",
                "reasoning": true,
                "tool_call": false,
                "temperature": true,
                "attachment": true,
                "interleaved": true,
                "modalities": {
                    "input": ["text", "image"],
                    "output": ["text"]
                },
                "limit": {"context": 131072, "input": 120000, "output": 8192},
                "headers": {"X-Model": "kept"},
                "options": {"reasoningEffort": "high"}
            }
        }
    });

    let draft = custom_provider_draft_from_value("custom", &raw).unwrap();
    let model = &draft.models[0];

    assert_eq!(model.model_id, "vision-model");
    assert_eq!(model.display_name, "Vision");
    assert_eq!(model.family, "vendor-family");
    assert_eq!(model.context_limit, "131072");
    assert_eq!(model.input_limit, "120000");
    assert_eq!(model.output_limit, "8192");
    assert_eq!(model.reasoning, Some(true));
    assert_eq!(model.tool_call, Some(false));
    assert_eq!(model.temperature, Some(true));
    assert_eq!(model.attachment, Some(true));
    assert_eq!(model.interleaved, Some(true));
    assert_eq!(model.input_modalities, Some(vec!["text".to_string(), "image".to_string()]));
    assert_eq!(model.output_modalities, Some(vec!["text".to_string()]));
}

#[test]
fn custom_provider_draft_defaults_missing_optional_fields() {
    let draft = custom_provider_draft_from_value("minimal-provider", &json!({})).unwrap();

    assert_eq!(draft.provider_id, "minimal-provider");
    assert!(draft.display_name.is_empty());
    assert!(draft.base_url.is_empty());
    assert_eq!(draft.headers.len(), 1);
    assert!(draft.headers[0].key.is_empty());
    assert!(draft.headers[0].value.is_empty());
    assert_eq!(draft.models.len(), 1);
    assert!(draft.models[0].model_id.is_empty());
    assert!(draft.models[0].display_name.is_empty());
}

#[test]
fn custom_provider_draft_defaults_fields_with_wrong_types() {
    let draft = custom_provider_draft_from_value(
        "typed-provider",
        &json!({"name": true, "api": 7, "models": "not a map"}),
    )
    .unwrap();

    assert!(draft.display_name.is_empty());
    assert!(draft.base_url.is_empty());
    assert_eq!(draft.models.len(), 1);
    assert!(draft.models[0].model_id.is_empty());
}

#[test]
fn custom_provider_draft_uses_placeholder_for_empty_models_map() {
    let draft = custom_provider_draft_from_value("empty-models", &json!({"models": {}})).unwrap();

    assert_eq!(draft.models.len(), 1);
    assert!(draft.models[0].model_id.is_empty());
    assert!(draft.models[0].display_name.is_empty());
}

#[test]
fn extract_shared_headers_defaults_without_models_map() {
    for raw in [json!({}), json!({"models": null}), json!({"models": []})] {
        let headers = extract_shared_headers(raw.as_object().unwrap());

        assert_eq!(headers.len(), 1);
        assert!(headers[0].key.is_empty());
        assert!(headers[0].value.is_empty());
    }
}

#[test]
fn extract_shared_headers_skips_malformed_models_and_headers() {
    let raw = json!({
        "models": {
            "model-a": "not an object",
            "model-b": {"headers": "not an object"},
            "model-c": {"headers": {"Numeric": 42, "Flag": true}}
        }
    });

    let headers = extract_shared_headers(raw.as_object().unwrap());

    assert_eq!(headers.len(), 1);
    assert!(headers[0].key.is_empty());
    assert!(headers[0].value.is_empty());
}

#[test]
fn extract_shared_headers_ignores_non_string_values_and_sorts_keys() {
    let raw = json!({
        "models": {
            "model-a": {
                "headers": {
                    "X-Zeta": "zeta",
                    "Ignored": 17,
                    "Authorization": "Bearer test-value"
                }
            }
        }
    });

    let headers = extract_shared_headers(raw.as_object().unwrap());

    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0].key, "Authorization");
    assert_eq!(headers[0].value, "Bearer test-value");
    assert_eq!(headers[1].key, "X-Zeta");
    assert_eq!(headers[1].value, "zeta");
}

#[test]
fn extract_shared_headers_continues_after_empty_header_map() {
    let raw = json!({
        "models": {
            "model-a": {"headers": {}},
            "model-b": {"headers": {"X-Shared": "shared-value"}}
        }
    });

    let headers = extract_shared_headers(raw.as_object().unwrap());

    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].key, "X-Shared");
    assert_eq!(headers[0].value, "shared-value");
}

#[test]
fn extract_shared_headers_uses_first_non_empty_header_map() {
    let raw = json!({
        "models": {
            "model-a": {"headers": {"X-First": "first-value"}},
            "model-b": {"headers": {"X-Second": "second-value"}}
        }
    });

    let headers = extract_shared_headers(raw.as_object().unwrap());

    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].key, "X-First");
    assert_eq!(headers[0].value, "first-value");
}

#[test]
fn custom_provider_patch_from_draft_trims_values_and_preserves_existing_model_config() {
    let draft = CustomProviderDraft {
        provider_id: " custom ".to_string(),
        display_name: " Custom Name ".to_string(),
        base_url: " https://api.example.test/v1 ".to_string(),
        api_key: " ignored-by-patch ".to_string(),
        headers: vec![
            ProviderHeaderDraft {
                key: " Authorization ".to_string(),
                value: " Bearer k ".to_string(),
            },
            ProviderHeaderDraft { key: "Blank".to_string(), value: " ".to_string() },
        ],
        models: vec![
            CustomProviderModelDraft {
                model_id: " model-a ".to_string(),
                display_name: " Model A ".to_string(),
                ..Default::default()
            },
            CustomProviderModelDraft {
                model_id: " ".to_string(),
                display_name: " skipped ".to_string(),
                ..Default::default()
            },
        ],
    };
    let existing = json!({
        "name": "Old Name",
        "keep": true,
        "models": {
            "model-a": {
                "status": "disabled",
                "headers": {"Old": "value"}
            }
        }
    });

    let (provider_id, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();

    assert_eq!(provider_id, "custom");
    assert_eq!(
        patch,
        json!({
            "providers": {
                "custom": {
                    "name": "Custom Name",
                    "api": "https://api.example.test/v1",
                    "keep": true,
                    "models": {
                        "model-a": {
                            "name": "Model A",
                            "status": "active",
                            "headers": {"Authorization": "Bearer k"}
                        }
                    }
                }
            }
        })
    );
}

#[test]
fn custom_provider_patch_accepts_empty_model_list() {
    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: "Custom".to_string(),
        base_url: "https://api.example.test/v1".to_string(),
        api_key: String::new(),
        headers: vec![],
        models: vec![CustomProviderModelDraft::default()],
    };

    let (_, patch) = custom_provider_patch_from_draft(&draft, None).unwrap();

    assert_eq!(patch.pointer("/providers/custom/models"), Some(&json!({})));
}

#[test]
fn custom_provider_patch_nulls_removed_and_renamed_models_for_deep_merge() {
    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: "Custom".to_string(),
        base_url: "https://api.example.test/v1".to_string(),
        api_key: String::new(),
        headers: vec![],
        models: vec![
            CustomProviderModelDraft {
                model_id: "model-kept".to_string(),
                display_name: "Kept".to_string(),
                original_model_id: Some("model-kept".to_string()),
                ..Default::default()
            },
            CustomProviderModelDraft {
                model_id: "model-renamed".to_string(),
                display_name: "Renamed".to_string(),
                original_model_id: Some("model-old-name".to_string()),
                ..Default::default()
            },
        ],
    };
    let existing = json!({
        "models": {
            "model-kept": {"name": "Kept"},
            "model-old-name": {"name": "Old"},
            "model-deleted": {"name": "Deleted"}
        }
    });

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();

    assert_eq!(
        patch.pointer("/providers/custom/models").unwrap(),
        &json!({
            "model-kept": {"name": "Kept", "status": "active"},
            "model-renamed": {"name": "Renamed", "status": "active"},
            "model-old-name": null,
            "model-deleted": null
        })
    );
}

#[test]
fn custom_provider_patch_skips_untouched_disabled_catalog_models() {
    let draft = CustomProviderDraft {
        provider_id: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: String::new(),
        headers: vec![],
        models: vec![
            CustomProviderModelDraft {
                model_id: "gpt-4.1".to_string(),
                display_name: "GPT-4.1".to_string(),
                enabled: true,
                from_catalog: true,
                ..Default::default()
            },
            CustomProviderModelDraft {
                model_id: "gpt-legacy".to_string(),
                display_name: "Legacy".to_string(),
                enabled: false,
                from_catalog: true,
                ..Default::default()
            },
            CustomProviderModelDraft {
                model_id: "custom-only".to_string(),
                display_name: "Custom".to_string(),
                enabled: false,
                from_catalog: false,
                ..Default::default()
            },
        ],
    };
    let existing = json!({
        "models": {
            "gpt-4.1": {"status": "active"}
        }
    });

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();
    assert_eq!(
        patch.pointer("/providers/openai/models").unwrap(),
        &json!({
            "gpt-4.1": {"status": "active", "name": "GPT-4.1"},
            "custom-only": {"name": "Custom", "status": "disabled"}
        })
    );
}

#[test]
fn merge_custom_provider_draft_combines_config_and_settings_models() {
    use crate::app::provider::provider::{
        ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
        ModelCostCache, ModelLimit, ProviderSource,
    };
    use std::collections::HashMap;

    fn sample_model(id: &str, name: &str, status: &str, context: u64) -> Model {
        Model {
            id: id.to_string(),
            provider_id: "openai".to_string(),
            api: ApiInfo {
                id: id.to_string(),
                url: "https://api.openai.com/v1".to_string(),
                adapter: "openai".to_string(),
            },
            name: name.to_string(),
            family: Some("gpt".to_string()),
            capabilities: Capabilities {
                temperature: true,
                reasoning: false,
                attachment: false,
                toolcall: true,
                input: CapabilityIO {
                    text: true,
                    audio: false,
                    image: id == "catalog-model",
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
            limit: ModelLimit { context, input: Some(28_672), output: 4096 },
            status: status.to_string(),
            options: Default::default(),
            headers: Default::default(),
            release_date: String::new(),
            variants: HashMap::new(),
        }
    }

    let raw = json!({
        "name": "Custom OpenAI",
        "api": "https://example.test/v1",
        "models": {
            "custom-model": {"name": "Custom Model", "status": "active"},
            "catalog-model": {
                "status": "disabled",
                "limit": {"context": 333333333, "input": 28672}
            }
        }
    });
    let settings = Info {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        source: ProviderSource::Config,
        env: Vec::new(),
        key: None,
        options: Default::default(),
        models: HashMap::from([
            (
                "catalog-model".to_string(),
                sample_model("catalog-model", "Catalog Model", "disabled", 128_000),
            ),
            (
                "extra-catalog".to_string(),
                sample_model("extra-catalog", "Extra Catalog", "active", 32_000),
            ),
        ]),
    };
    let catalog_ids = HashSet::from(["catalog-model".to_string(), "extra-catalog".to_string()]);

    let draft = merge_custom_provider_draft("openai", &raw, &settings, &catalog_ids).unwrap();
    let ids = draft.models.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["catalog-model", "custom-model", "extra-catalog"]);
    assert!(!draft.models[0].enabled);
    assert!(draft.models[0].from_catalog);
    assert_eq!(draft.models[0].display_name, "Catalog Model");
    assert_eq!(draft.models[0].context_limit, "333333333");
    assert!(draft.models[0].input_limit.is_empty());
    let inherited = draft.models[0]
        .inherited_capabilities
        .as_ref()
        .expect("catalog model capabilities should be available to the editor");
    assert!(inherited.input_modalities.iter().any(|modality| modality == "image"));
    assert!(draft.models[1].enabled);
    assert!(!draft.models[1].from_catalog);
    assert!(draft.models[2].enabled);
    assert!(draft.models[2].from_catalog);
}

#[test]
fn custom_provider_patch_from_draft_removes_empty_names_and_headers() {
    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: " ".to_string(),
        base_url: " ".to_string(),
        api_key: String::new(),
        headers: vec![ProviderHeaderDraft { key: " ".to_string(), value: "value".to_string() }],
        models: vec![CustomProviderModelDraft {
            model_id: "model-a".to_string(),
            display_name: " ".to_string(),
            ..Default::default()
        }],
    };
    let existing = json!({
        "name": "Old Name",
        "models": {
            "model-a": {
                "name": "Old Model",
                "headers": {"Old": "value"},
                "status": "active"
            }
        }
    });

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();

    assert_eq!(
        patch,
        json!({
            "providers": {
                "custom": {
                    "api": "",
                    "models": {
                        "model-a": {
                            "status": "active"
                        }
                    }
                }
            }
        })
    );
}

#[test]
fn custom_provider_patch_writes_explicit_model_metadata_and_preserves_unedited_fields() {
    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: "Custom".to_string(),
        base_url: "https://api.example.test/v1".to_string(),
        api_key: String::new(),
        headers: vec![ProviderHeaderDraft { key: "X-Keep".to_string(), value: "yes".to_string() }],
        models: vec![CustomProviderModelDraft {
            model_id: "model-a".to_string(),
            display_name: "Model A".to_string(),
            family: "family-a".to_string(),
            context_limit: "65536".to_string(),
            input_limit: "60000".to_string(),
            output_limit: "4096".to_string(),
            reasoning: Some(true),
            tool_call: Some(false),
            temperature: Some(true),
            attachment: Some(true),
            interleaved: Some(true),
            input_modalities: Some(vec!["text".to_string(), "image".to_string()]),
            output_modalities: Some(vec!["text".to_string()]),
            ..Default::default()
        }],
    };
    let existing = json!({
        "models": {
            "model-a": {
                "id": "vendor-model-a",
                "status": "active",
                "headers": {"X-Keep": "yes"},
                "options": {"foo": "bar"}
            }
        }
    });

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();
    assert_eq!(
        patch,
        json!({
            "providers": {
                "custom": {
                    "name": "Custom",
                    "api": "https://api.example.test/v1",
                    "models": {
                        "model-a": {
                            "id": "vendor-model-a",
                            "name": "Model A",
                            "family": "family-a",
                            "reasoning": true,
                            "tool_call": false,
                            "temperature": true,
                            "attachment": true,
                            "interleaved": true,
                            "modalities": {"input": ["text", "image"], "output": ["text"]},
                            "limit": {"context": 65536, "input": 60000, "output": 4096},
                            "status": "active",
                            "headers": {"X-Keep": "yes"},
                            "options": {"foo": "bar"}
                        }
                    }
                }
            }
        })
    );
}

#[test]
fn cross_layer_fixture_patch_and_usage_window_share_the_default_limit() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../vw-shared/assets/provider/cross_layer_unknown_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let draft = CustomProviderDraft {
        provider_id: fixture["provider_id"].as_str().unwrap().to_string(),
        display_name: fixture["provider_name"].as_str().unwrap().to_string(),
        base_url: fixture["base_url"].as_str().unwrap().to_string(),
        api_key: String::new(),
        headers: Vec::new(),
        models: vec![CustomProviderModelDraft {
            model_id: fixture["model_id"].as_str().unwrap().to_string(),
            display_name: fixture["model_name"].as_str().unwrap().to_string(),
            ..Default::default()
        }],
    };

    let (_, patch) = custom_provider_patch_from_draft(&draft, None).unwrap();
    let model = &patch["providers"][fixture["provider_id"].as_str().unwrap()]["models"]
        [fixture["model_id"].as_str().unwrap()];
    assert!(model.get("limit").is_none());

    let resolved = vw_shared::provider::state::resolve_model_limit(None, None, None);
    assert_eq!(vw_shared::provider::state::effective_input_window(&resolved), 200_000);
}

#[test]
fn cross_layer_explicit_fixture_round_trips_through_settings_patch() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../vw-shared/assets/provider/cross_layer_explicit_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let draft = CustomProviderDraft {
        provider_id: fixture["provider_id"].as_str().unwrap().to_string(),
        display_name: fixture["provider_name"].as_str().unwrap().to_string(),
        base_url: fixture["base_url"].as_str().unwrap().to_string(),
        api_key: String::new(),
        headers: Vec::new(),
        models: vec![CustomProviderModelDraft {
            model_id: fixture["model_id"].as_str().unwrap().to_string(),
            display_name: fixture["model_name"].as_str().unwrap().to_string(),
            family: fixture["family"].as_str().unwrap().to_string(),
            context_limit: fixture["context_limit"].as_u64().unwrap().to_string(),
            input_limit: fixture["input_limit"].as_u64().unwrap().to_string(),
            output_limit: fixture["output_limit"].as_u64().unwrap().to_string(),
            reasoning: fixture["reasoning"].as_bool(),
            tool_call: fixture["tool_call"].as_bool(),
            temperature: fixture["temperature"].as_bool(),
            attachment: fixture["attachment"].as_bool(),
            interleaved: fixture["interleaved"].as_bool(),
            input_modalities: Some(
                fixture["input_modalities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_str().unwrap().to_string())
                    .collect(),
            ),
            output_modalities: Some(
                fixture["output_modalities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_str().unwrap().to_string())
                    .collect(),
            ),
            ..Default::default()
        }],
    };

    let (_, patch) = custom_provider_patch_from_draft(&draft, None).unwrap();
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let restored =
        custom_provider_draft_from_value(provider_id, &patch["providers"][provider_id]).unwrap();
    let model = &restored.models[0];
    assert_eq!(model.model_id, fixture["model_id"].as_str().unwrap());
    assert_eq!(model.family, fixture["family"].as_str().unwrap());
    assert_eq!(model.context_limit, fixture["context_limit"].as_u64().unwrap().to_string());
    assert_eq!(model.input_limit, fixture["input_limit"].as_u64().unwrap().to_string());
    assert_eq!(model.output_limit, fixture["output_limit"].as_u64().unwrap().to_string());
    assert_eq!(model.reasoning, fixture["reasoning"].as_bool());
    assert_eq!(model.tool_call, fixture["tool_call"].as_bool());
    assert_eq!(model.temperature, fixture["temperature"].as_bool());
    assert_eq!(model.attachment, fixture["attachment"].as_bool());
    assert_eq!(model.interleaved, fixture["interleaved"].as_bool());
    assert_eq!(
        model.input_modalities.as_deref(),
        Some(["text".to_string(), "image".to_string()].as_slice())
    );
    assert_eq!(model.output_modalities.as_deref(), Some(["text".to_string()].as_slice()));
}

#[test]
fn custom_provider_patch_rejects_invalid_numbers_and_relationships() {
    let mut model = CustomProviderModelDraft {
        model_id: "model".to_string(),
        context_limit: "not-a-number".to_string(),
        ..Default::default()
    };
    let draft = |model| CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: String::new(),
        base_url: "https://example.test".to_string(),
        api_key: String::new(),
        headers: vec![ProviderHeaderDraft::default()],
        models: vec![model],
    };

    assert_eq!(
        custom_provider_patch_from_draft(&draft(model.clone()), None).unwrap_err(),
        "模型 model 的上下文限制必须是正整数"
    );

    model.context_limit = "100".to_string();
    model.input_limit = "101".to_string();
    assert!(custom_provider_patch_from_draft(&draft(model.clone()), None).is_ok());

    model.input_limit.clear();
    model.output_limit = "101".to_string();
    assert_eq!(
        custom_provider_patch_from_draft(&draft(model), None).unwrap_err(),
        "模型 model 的输出限制不能超过上下文限制"
    );
}

#[test]
fn custom_provider_patch_removes_empty_overrides_so_resolver_can_use_defaults() {
    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: String::new(),
        base_url: "https://example.test".to_string(),
        api_key: String::new(),
        headers: vec![ProviderHeaderDraft::default()],
        models: vec![CustomProviderModelDraft {
            model_id: "model".to_string(),
            family: "old".to_string(),
            context_limit: String::new(),
            input_limit: String::new(),
            output_limit: String::new(),
            ..Default::default()
        }],
    };
    let existing = json!({
        "models": {
            "model": {
                "limit": {"context": 1234, "input": 1000, "output": 200},
                "reasoning": true,
                "family": "old"
            }
        }
    });

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing)).unwrap();
    let model = &patch["providers"]["custom"]["models"]["model"];
    assert!(model.get("limit").is_none());
    assert_eq!(model["reasoning"], true);
    assert_eq!(model["family"], "old");
}
