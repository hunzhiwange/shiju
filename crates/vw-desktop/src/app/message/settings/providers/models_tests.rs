use super::*;
use crate::app::state::{CustomProviderModelDraft, ProviderHeaderDraft};
use serde_json::json;

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
            },
            CustomProviderModelDraft {
                model_id: " ".to_string(),
                display_name: " skipped ".to_string(),
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

    let (provider_id, patch) = custom_provider_patch_from_draft(&draft, Some(&existing));

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
                            "status": "disabled",
                            "headers": {"Authorization": "Bearer k"}
                        }
                    }
                }
            }
        })
    );
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

    let (_, patch) = custom_provider_patch_from_draft(&draft, Some(&existing));

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
