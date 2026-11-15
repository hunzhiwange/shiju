use crate::*;
use crate::test_support::*;
use serde_json::{Value, json};
use crate::native::load_state_impl;
use provider_model_cache as models;

#[tokio::test]
async fn cross_layer_unknown_model_uses_shared_default_limit() {
    let env = ProviderTestEnv::new().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../vw-shared/assets/provider/cross_layer_unknown_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();
    let providers = json!({
        "models": {model_id: {"name": fixture["model_name"], "status": "active"}}
    });
    let mut provider = providers;
    provider["api"] = fixture["base_url"].clone();
    let mut provider_map = serde_json::Map::new();
    provider_map.insert(provider_id.to_string(), provider);
    env.write_config(config_with_default(Value::Null, Value::Object(provider_map))).await;

    let model = list_for_settings().await[provider_id].models[model_id].clone();
    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
}

#[tokio::test]
async fn cross_layer_explicit_model_preserves_limits_and_capabilities_after_reload() {
    let env = ProviderTestEnv::new().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../vw-shared/assets/provider/cross_layer_explicit_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();
    let model = json!({
        "name": fixture["model_name"],
        "family": fixture["family"],
        "reasoning": fixture["reasoning"],
        "tool_call": fixture["tool_call"],
        "temperature": fixture["temperature"],
        "attachment": fixture["attachment"],
        "interleaved": fixture["interleaved"],
        "modalities": {
            "input": fixture["input_modalities"],
            "output": fixture["output_modalities"]
        },
        "limit": {
            "context": fixture["context_limit"],
            "input": fixture["input_limit"],
            "output": fixture["output_limit"]
        },
        "status": "active"
    });
    let mut provider = json!({"api": fixture["base_url"], "models": {}});
    provider["models"][model_id] = model;
    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), provider);
    env.write_config(config_with_default(Value::Null, Value::Object(providers))).await;

    let model = list_for_settings().await[provider_id].models[model_id].clone();
    assert_eq!(model.limit.context, fixture["context_limit"].as_u64().unwrap());
    assert_eq!(model.limit.input, Some(fixture["input_limit"].as_u64().unwrap()));
    assert_eq!(model.limit.output, fixture["output_limit"].as_u64().unwrap());
    assert_eq!(model.family.as_deref(), fixture["family"].as_str());
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.temperature);
    assert!(model.capabilities.attachment);
    assert!(model.capabilities.input.text && model.capabilities.input.image);
    assert!(model.capabilities.output.text);
}

#[tokio::test]
async fn custom_config_provider_uses_safe_defaults() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-only": {
                "api": "https://config.example/custom",
                "models": {
                    "custom-model": {
                        "status": "active",
                        "limit": {"context": 7000, "output": 700}
                    }
                }
            }
        }),
    ))
    .await;

    let model =
        get_model("custom-only", "custom-model").await.expect("custom model should resolve");

    assert_eq!(model.api.id, "custom-model");
    assert_eq!(model.api.adapter, default_adapter());
    assert_eq!(model.name, "custom-model");
    assert_eq!(model.provider_id, "custom-only");
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert_eq!(model.cost.output, 0.0);
}

#[tokio::test]
async fn custom_model_without_limits_uses_shared_defaults_and_explicit_limits_win() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-limit-provider": {
                "models": {
                    "fallback-model": {"status": "active"},
                    "explicit-model": {
                        "status": "active",
                        "limit": {"context": 12_345, "input": 10_000, "output": 2_048}
                    }
                }
            }
        }),
    ))
    .await;

    let fallback = get_model("custom-limit-provider", "fallback-model")
        .await
        .expect("fallback model should resolve");
    assert_eq!(fallback.limit.context, vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT);
    assert_eq!(fallback.limit.input, Some(vw_shared::provider::state::DEFAULT_MODEL_INPUT_LIMIT));
    assert_eq!(fallback.limit.output, vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT);

    let explicit = get_model("custom-limit-provider", "explicit-model")
        .await
        .expect("explicit model should resolve");
    assert_eq!(explicit.limit.context, 12_345);
    assert_eq!(explicit.limit.input, Some(10_000));
    assert_eq!(explicit.limit.output, 2_048);
}

#[tokio::test]
async fn custom_model_explicit_capabilities_override_baseline_fields() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-capability-provider": {
                "models": {
                    "custom-model": {
                        "status": "active",
                        "family": "custom-family",
                        "reasoning": true,
                        "tool_call": false,
                        "temperature": true,
                        "attachment": true,
                        "modalities": {
                            "input": ["text", "image"],
                            "output": ["text"]
                        },
                        "limit": {"context": 65536, "input": 60000, "output": 4096}
                    }
                }
            }
        }),
    ))
    .await;

    let model = get_model("custom-capability-provider", "custom-model")
        .await
        .expect("custom model should resolve");

    assert_eq!(model.family.as_deref(), Some("custom-family"));
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.temperature);
    assert!(model.capabilities.attachment);
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.input.image);
    assert!(model.capabilities.output.text);
    assert!(!model.capabilities.output.image);
    assert_eq!(model.limit.context, 65536);
    assert_eq!(model.limit.input, Some(60000));
    assert_eq!(model.limit.output, 4096);
}

#[tokio::test]
async fn new_model_inherits_unique_builtin_provider_api() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "deepseek": {
                "models": {
                    "deepseek-future": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model =
        get_model("deepseek", "deepseek-future").await.expect("configured model should resolve");

    assert_eq!(model.api.url, "https://api.deepseek.com");
    assert_eq!(model.api.adapter, default_adapter());
}

#[tokio::test]
async fn provider_id_and_model_id_defaults_are_used_when_config_names_are_absent() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "unnamed-provider": {
                "models": {
                    "unnamed-model": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let provider = get_provider("unnamed-provider").await.expect("provider should resolve");
    let model = get_model("unnamed-provider", "unnamed-model").await.expect("model should resolve");

    assert_eq!(provider.name, "unnamed-provider");
    assert_eq!(model.name, "unnamed-model");
    assert_eq!(model.api.id, "unnamed-model");
    assert_eq!(model.api.url, "");
}

#[tokio::test]
async fn custom_provider_reasoning_inherits_catalog_for_shiju_and_token_plan() {
    let env = ProviderTestEnv::new().await;
    env.write_config(json!({"providers": {
        "shiju": {"models": {"deepseek-v4.1-flash": {"status": "active"}}},
        "token-plan": {"api": "https://example.test/v1", "models": {
            "deepseek-v4-flash-0731": {"status": "active"},
            "qwen3.7-plus": {"status": "active"},
            "alias": {"id": "glm-5.2", "status": "active"},
            "deepseek-v4-pro": {"status": "active", "reasoning": false},
            "wan2.7-image": {"status": "active"},
            "unknown-model": {"status": "active"}
        }}
    }}))
    .await;
    let state = load_state_impl(false).await;
    assert!(state.providers["shiju"].models["deepseek-v4.1-flash"].capabilities.reasoning);
    for (id, expected) in [
        ("deepseek-v4.1-flash", vec!["low", "high", "max"]),
        ("deepseek-v4-pro-0813", vec!["high", "max"]),
        ("glm-5.3", vec!["low", "high", "max"]),
        ("qwen3.8-max", vec!["low", "medium", "xhigh"]),
        ("qwen3.7-plus", vec![]),
        ("kimi/kimi-k2.8-preview", vec!["low", "high", "max"]),
    ] {
        let model = &state.providers["shiju"].models[id];
        assert_eq!(
            vw_shared::provider::reasoning::reasoning_levels(
                id,
                "openai-compatible",
                "shiju",
                "",
                model.capabilities.reasoning_options.as_deref(),
            ),
            expected,
            "{id}"
        );
    }

    let models = &state.providers["token-plan"].models;
    for id in ["deepseek-v4-flash-0731", "qwen3.7-plus", "alias"] {
        assert!(models[id].capabilities.reasoning, "{id}");
        assert_eq!(models[id].api.url, "https://example.test/v1");
    }
    for id in ["deepseek-v4-pro", "wan2.7-image", "unknown-model"] {
        assert!(!models[id].capabilities.reasoning, "{id}");
    }
}

#[test]
fn model_protocol_010_model_endpoint_precedes_provider_default() {
    let raw: models::Provider = serde_json::from_value(json!({
        "id":"fixture", "api":"https://provider.example/v1", "npm":"@ai-sdk/openai",
        "models":{"m":{"id":"m", "provider":{"api":"https://model.example/v1", "npm":"@ai-sdk/anthropic"}}}
    })).unwrap();
    let model = from_models_dev_model(&raw, &raw.models["m"]);
    assert_eq!(model.api.url, "https://model.example/v1");
    assert_eq!(model.api.adapter, "anthropic");
}

#[tokio::test]
async fn model_protocol_010_config_preserves_or_replaces_catalog_deployment() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({"deepseek": {
            "models": {"deepseek-v4-flash":{"status":"active"}}
        }}),
    ))
    .await;
    let original = list_for_settings().await;
    assert!(
        original["deepseek"].models["deepseek-v4-flash"]
            .api
            .protocols
            .as_ref()
            .is_some_and(|p| !p.is_empty())
    );
    env.write_config(config_with_default(Value::Null, json!({"deepseek": {
        "api":"https://private.example/v1",
        "models": {"deepseek-v4-flash":{"status":"active", "provider":{"api":"https://model.example/v1", "protocols":[]}}}
    }}))).await;
    let configured = list_for_settings().await;
    let model = &configured["deepseek"].models["deepseek-v4-flash"];
    assert_eq!(model.api.url, "https://model.example/v1");
    assert_eq!(model.api.protocols, Some(vec![]));
    for (id, model) in &configured["deepseek"].models {
        if id != "deepseek-v4-flash" {
            assert_eq!(model.api.url, "https://private.example/v1");
            assert_eq!(model.api.protocols, None);
        }
    }
}
