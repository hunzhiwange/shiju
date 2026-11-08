//! 模型基线测试覆盖多种 JSON 包装结构、内置资源完整性、磁盘与进程缓存优先级及远端刷新。
//!
//! 内置目录审计关注模型限制与模态是否可用；本地 HTTP fixture 只监听回环地址，用于验证
//! 成功写入和各类失败回退，不依赖外部网络服务。

use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::HashSet;
use vw_shared::provider::state::from_models_dev_model;

#[cfg(not(target_arch = "wasm32"))]
#[path = "models_cache_tests.rs"]
mod cache;

fn provider_json(id: &str) -> String {
    format!(
        r#"{{
            "id": "{id}",
            "name": "{id} provider",
            "env": ["{id}_KEY"],
            "models": {{
                "{id}-model": {{
                    "id": "{id}-model",
                    "name": "{id} model",
                    "limit": {{"context": 8, "output": 4}}
                }}
            }}
        }}"#
    )
}

fn provider_map_json(id: &str) -> String {
    format!(r#"{{"{id}": {}}}"#, provider_json(id))
}

fn assert_provider(map: &HashMap<String, Provider>, id: &str) {
    let provider = map.get(id).expect("provider should be parsed");
    assert_eq!(provider.id, id);
    assert_eq!(provider.name, format!("{id} provider"));
    assert!(provider.models.contains_key(&format!("{id}-model")));
}

#[test]
fn url_returns_default_models_dev_url() {
    assert_eq!(models_url(None), "https://models.dev");
    assert_eq!(models_url(Some("https://models.example.test")), "https://models.example.test");
}

#[test]
fn cache_path_uses_global_cache_directory() {
    assert_eq!(cache_path(), global::paths().cache.join("models.json"));
}

#[test]
fn parse_models_text_accepts_direct_provider_map() {
    let parsed = parse_models_text("direct", &provider_map_json("direct")).unwrap();

    assert_provider(&parsed, "direct");
}

#[test]
fn parse_models_text_accepts_providers_wrapper() {
    let text = format!(r#"{{"providers": {}}}"#, provider_map_json("wrapped"));
    let parsed = parse_models_text("wrapped", &text).unwrap();

    assert_provider(&parsed, "wrapped");
}

#[test]
fn parse_models_text_accepts_data_wrapper() {
    let text = format!(r#"{{"data": {}}}"#, provider_map_json("data"));
    let parsed = parse_models_text("data", &text).unwrap();

    assert_provider(&parsed, "data");
}

#[test]
fn parse_models_text_accepts_provider_array_and_filters_empty_ids() {
    let text = format!(r#"[{}, {}]"#, provider_json("array"), provider_json(""));
    let parsed = parse_models_text("array", &text).unwrap();

    assert_provider(&parsed, "array");
    assert!(!parsed.contains_key(""));
}

#[test]
fn parse_models_text_accepts_empty_provider_map_and_array() {
    assert!(parse_models_text("empty_map", "{}").unwrap().is_empty());
    assert!(parse_models_text("empty_array", "[]").unwrap().is_empty());
}

#[test]
fn parse_models_text_rejects_invalid_or_unsupported_shapes() {
    assert!(parse_models_text("invalid", "{").is_none());
    assert!(parse_models_text("other", "42").is_none());
    assert!(parse_models_text("bad_wrapper", r#"{"providers": 42}"#).is_none());
}

#[test]
fn parse_models_text_falls_back_from_bad_providers_wrapper_to_data_wrapper() {
    let text = format!(
        r#"{{
            "providers": 42,
            "data": {}
        }}"#,
        provider_map_json("fallback")
    );

    let parsed = parse_models_text("fallback", &text).unwrap();

    assert_provider(&parsed, "fallback");
}

#[test]
fn parse_models_text_prefers_providers_wrapper_over_data_wrapper() {
    let text = format!(
        r#"{{"providers": {}, "data": {}}}"#,
        provider_map_json("preferred"),
        provider_map_json("fallback")
    );

    let parsed = parse_models_text("wrapper_precedence", &text).unwrap();

    assert_provider(&parsed, "preferred");
    assert!(!parsed.contains_key("fallback"));
}

#[test]
fn parse_models_text_rejects_array_items_with_invalid_provider_shape() {
    let text = r#"[{"id": "bad", "models": []}]"#;

    assert!(parse_models_text("bad_array", text).is_none());
}

#[test]
fn catalog_modelsdev_parse_preserves_canonical_pointer() {
    let text = r#"{"relay":{"id":"relay","models":{"vendor/Flagship:fast":{
        "id":"vendor/Flagship:fast","canonical_model_id":"vendor/Flagship/2026-11"
    }}}}"#;
    let parsed = parse_models_text("canonical_pointer", text).unwrap();
    let model = &parsed["relay"].models["vendor/Flagship:fast"];
    assert_eq!(model.id, "vendor/Flagship:fast");
    assert_eq!(
        serde_json::to_value(model).unwrap()["canonical_model_id"],
        "vendor/Flagship/2026-11"
    );
}

#[test]
fn catalog_modelsdev_parse_keeps_original_provider_ownership_in_wrappers() {
    let providers = json!({
        "relay": {"id": "relay-api", "models": {"Flagship": {
            "id": "Flagship", "canonical_model_id": "vendor/Flagship", "reasoning": false
        }}},
        "vendor": {"id": "vendor-api", "models": {"Flagship": {
            "id": "Flagship", "reasoning": true
        }}}
    });
    for wrapper in ["providers", "data"] {
        let wrapped = json!({(wrapper): providers});
        let parsed = parse_models_text(wrapper, &wrapped.to_string()).unwrap();
        assert_eq!(parsed["relay"].id, "relay-api");
        assert_eq!(parsed["vendor"].id, "vendor-api");
        let relay = serde_json::to_value(&parsed["relay"].models["Flagship"]).unwrap();
        let vendor = serde_json::to_value(&parsed["vendor"].models["Flagship"]).unwrap();
        assert_eq!(relay["canonical_model_id"], "vendor/Flagship");
        assert!(vendor["canonical_model_id"].is_null());
        assert_eq!(relay["reasoning"], false);
        assert_eq!(vendor["reasoning"], true);
    }
}

#[test]
fn catalog_modelsdev_parse_preserves_bundled_pointer_and_local_overlay() {
    let parsed = bundled_models("canonical_bundled");
    let relay = &parsed["deepinfra"].models["stepfun-ai/Step-3.7-Flash"];
    assert_eq!(
        serde_json::to_value(relay).unwrap()["canonical_model_id"],
        "stepfun/step-3.7-flash"
    );
    assert!(parsed["stepfun"].models.contains_key("step-3.7-flash"));
    assert!(!parsed["shiju"].models.is_empty());
}

#[test]
fn bundled_models_loads_included_metadata() {
    let bundled = bundled_models("bundled");

    assert!(!bundled.is_empty());
}

#[test]
fn bundled_models_include_qwen37_plus_multimodal_metadata() {
    let bundled = bundled_models("bundled_qwen37_plus");
    let alibaba = bundled.get("alibaba-cn").expect("Alibaba China provider should be bundled");
    let model = alibaba.models.get("qwen3.7-plus").expect("Qwen3.7 Plus should be bundled");
    let modalities = model.modalities.as_ref().expect("Qwen3.7 Plus should declare modalities");
    let cost = model.cost.as_ref().expect("Qwen3.7 Plus should declare cost");

    assert!(model.attachment);
    assert_eq!(
        modalities.input.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["text", "image", "video"]
    );
    assert_eq!(modalities.output.iter().map(String::as_str).collect::<Vec<_>>(), vec!["text"]);
    assert_eq!(model.limit.context, 1_000_000);
    assert_eq!(model.limit.output, 64_000);
    assert_eq!(cost.input, Some(0.5));
    assert_eq!(cost.output, Some(3.0));
    assert_eq!(cost.cache_read, Some(0.05));
    assert_eq!(cost.cache_write, Some(0.625));
}

#[test]
fn bundled_models_include_shiju_static_catalog() {
    let bundled = bundled_models("bundled_shiju");
    let shiju = bundled.get("shiju").expect("bundled models should include shiju");
    let expected = HashSet::from([
        "cosyvoice-v2",
        "deepseek-v4-flash",
        "deepseek-v4-pro-0813",
        "deepseek-v4.1-flash",
        "deepseek-v4.1-flash-1017",
        "fun-asr",
        "glm-5.3",
        "gte-rerank-v2",
        "happyhorse-1.1-t2v",
        "kimi-k3",
        "kimi/kimi-k2.8-preview",
        "paraformer-v2",
        "qwen-audio-3.1-asr-flash-streaming",
        "qwen-audio-3.1-tts-next",
        "qwen-image-2.0",
        "qwen-image-2.0-pro",
        "qwen-image-3.0",
        "qwen-image-3.0-pro",
        "qwen-mt-plus",
        "qwen-vl-ocr",
        "qwen3-asr-flash",
        "qwen3-rerank",
        "qwen3-vl-plus",
        "qwen3.7-max",
        "qwen3.7-plus",
        "qwen3.8-2.4t-a95b",
        "qwen3.8-27b",
        "qwen3.8-flash",
        "qwen3.8-max",
        "qwen3.8-omni-flash",
        "text-embedding-v3",
        "text-embedding-v4",
        "voice-enrollment",
        "wan2.2-animate-mix",
        "wan2.2-s2v",
        "wan2.2-s2v-detect",
        "wan2.7-image",
        "wan2.7-image-pro",
        "wan3.0-video",
        "wan3.0-video-prime",
    ]);
    let actual = shiju.models.keys().map(String::as_str).collect::<HashSet<_>>();

    assert_eq!(shiju.id, "shiju");
    assert_eq!(shiju.name, "识矩");
    assert_eq!(shiju.env, vec!["SHIJU_API_KEY"]);
    assert_eq!(shiju.api.as_deref(), Some("https://router.shiju.cloud/v1"));
    assert_eq!(shiju.adapter.as_deref(), Some("openai-compatible"));
    assert_eq!(actual, expected);
    assert_eq!(shiju.models.len(), 40);
    assert!(shiju.models.contains_key("qwen3.8-max"));
    assert!(shiju.models.contains_key("deepseek-v4.1-flash"));

    for model_id in ["qwen-vl-ocr", "qwen3-vl-plus", "qwen-image-3.0", "kimi-k3"] {
        let input = &shiju.models[model_id]
            .modalities
            .as_ref()
            .unwrap_or_else(|| panic!("{model_id} should declare modalities"))
            .input;
        assert!(input.iter().any(|modality| modality == "image"));
    }

    for model_id in ["qwen3.8-flash", "qwen3.8-max"] {
        let input = &shiju.models[model_id]
            .modalities
            .as_ref()
            .unwrap_or_else(|| panic!("{model_id} should declare modalities"))
            .input;
        assert!(input.iter().any(|modality| modality == "image"));
        assert!(input.iter().any(|modality| modality == "video"));
    }

    let image_generation = shiju.models["qwen-image-2.0"]
        .modalities
        .as_ref()
        .expect("Qwen Image 2.0 should declare modalities");
    assert_eq!(image_generation.input, ["text"]);
    assert_eq!(image_generation.output, ["image"]);
    assert!(bundled.contains_key("openai"));
}

#[test]
fn bundled_shiju_models_have_usable_limits_and_modalities() {
    let shiju = bundled_models("bundled_shiju_audit")
        .remove("shiju")
        .expect("bundled models should include shiju");

    for (model_id, model) in shiju.models {
        let output_is_text = model
            .modalities
            .as_ref()
            .is_none_or(|modalities| modalities.output.iter().any(|item| item == "text"));
        if output_is_text {
            assert!(
                model.limit.context > 0
                    || documented_shiju_unknown_limit("shiju", &model_id, "context"),
                "{model_id} should have a text context limit"
            );
            assert!(
                model.limit.output > 0
                    || documented_shiju_unknown_limit("shiju", &model_id, "output"),
                "{model_id} should have a text output limit"
            );
        }
    }
}

#[test]
fn shiju_matched_models_keep_api_catalog_capabilities_and_limits() {
    let api: HashMap<String, Provider> = serde_json::from_str(BUNDLED_MODELS_TEXT).unwrap();
    let overlay: HashMap<String, Provider> =
        serde_json::from_str(BUNDLED_SHIJU_MODELS_TEXT).unwrap();
    let mut matched = 0;
    for (id, model) in &overlay["shiju"].models {
        let Some(source) = shiju_model_metadata(&api, id) else { continue };
        matched += 1;
        let actual = serde_json::to_value(model).unwrap();
        let expected = serde_json::to_value(source).unwrap();
        for field in [
            "name",
            "family",
            "attachment",
            "reasoning",
            "reasoning_options",
            "tool_call",
            "temperature",
            "modalities",
            "interleaved",
            "limit",
            "release_date",
        ] {
            assert_eq!(actual[field], expected[field], "shiju/{id}: {field}");
        }
        assert!(model.provider.is_none(), "{id}: do not inherit an upstream adapter");
        assert!(model.headers.is_none(), "{id}: do not inherit upstream authentication");
    }
    assert_eq!(matched, 22);
    assert!(shiju_model_metadata(&api, "kimi/kimi-k2.8-preview").is_none());
    assert!(shiju_model_metadata(&api, "qwen-image-3.0-pro").is_none());
    assert_eq!(overlay["shiju"].models["qwen3.8-27b"].limit.context, 1_000_000);
}

fn audit_provider_models(label: &str, providers: &HashMap<String, Provider>) -> Result<(), String> {
    for (provider_id, provider) in providers {
        for (model_id, model) in &provider.models {
            // Image/video generation APIs need not publish a text-token window.
            let media_generation = model.modalities.as_ref().is_some_and(|modalities| {
                !modalities.output.is_empty()
                    && modalities
                        .output
                        .iter()
                        .all(|modality| matches!(modality.as_str(), "image" | "video"))
            });
            if model.limit.context == 0
                && !media_generation
                && !documented_shiju_unknown_limit(provider_id, model_id, "context")
            {
                return Err(format!("{label}/{provider_id}/{model_id}: missing context limit"));
            }
            let Some(modalities) = model.modalities.as_ref() else {
                return Err(format!("{label}/{provider_id}/{model_id}: missing modalities"));
            };
            if modalities.input.is_empty() {
                return Err(format!("{label}/{provider_id}/{model_id}: missing input modalities"));
            }
            if (model.tool_call == Some(true) || model.reasoning == Some(true))
                && modalities.output.iter().any(|item| item == "text")
                && !modalities.input.iter().any(|item| item == "text")
            {
                return Err(format!(
                    "{label}/{provider_id}/{model_id}: text model missing text input"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn bundled_model_assets_have_no_missing_limits_or_malformed_modalities() {
    for (label, text) in [
        ("api", include_str!("../assets/model/api.json")),
        ("shiju", include_str!("../assets/model/shiju.json")),
    ] {
        let value: Value = serde_json::from_str(text).expect("model asset should be valid JSON");
        let providers = if label == "shiju" {
            value
                .get("shiju")
                .map(|provider| {
                    serde_json::from_value::<HashMap<String, Provider>>(json!({
                        "shiju": provider
                    }))
                })
                .expect("shiju asset should contain its provider")
                .expect("shiju provider should parse")
        } else {
            serde_json::from_value::<HashMap<String, Provider>>(value)
                .expect("api providers should parse")
        };

        audit_provider_models(label, &providers).unwrap_or_else(|error| panic!("{error}"));
    }
}

#[test]
fn model_asset_audit_rejects_missing_limits_bad_modalities_and_textless_text_models() {
    let missing_limit = json!({
        "fixture": {
            "models": {
                "broken": {
                    "id": "broken",
                    "name": "Broken",
                    "modalities": {"input": ["text"], "output": ["text"]}
                }
            }
        }
    });
    let parsed: HashMap<String, Provider> = serde_json::from_value(missing_limit).unwrap();
    let error = audit_provider_models("missing-limit", &parsed).unwrap_err();
    assert!(error.contains("missing context limit"));

    let malformed_modalities = json!({
        "fixture": {
            "models": {
                "broken": {
                    "id": "broken",
                    "name": "Broken",
                    "limit": {"context": 1024},
                    "modalities": {"input": "text", "output": ["text"]}
                }
            }
        }
    });
    assert!(serde_json::from_value::<HashMap<String, Provider>>(malformed_modalities).is_err());

    let textless_text_model = json!({
        "fixture": {
            "models": {
                "broken": {
                    "id": "broken",
                    "name": "Broken",
                    "reasoning": true,
                    "limit": {"context": 1024},
                    "modalities": {"input": ["image"], "output": ["text"]}
                }
            }
        }
    });
    let provider = serde_json::from_value::<HashMap<String, Provider>>(textless_text_model)
        .unwrap()
        .remove("fixture")
        .unwrap();
    let model = &provider.models["broken"];
    assert!(model.modalities.as_ref().unwrap().input.iter().all(|item| item != "text"));
    assert!(
        model.reasoning == Some(true)
            && model.modalities.as_ref().unwrap().output.iter().any(|item| item == "text")
    );
    let error = audit_provider_models(
        "textless-text",
        &HashMap::from([(String::from("fixture"), provider)]),
    )
    .unwrap_err();
    assert!(error.contains("text model missing text input"));
}

#[test]
fn bundled_api_models_have_typed_limits_and_non_text_outputs_are_not_text_models() {
    let bundled = bundled_models("bundled_api_audit");

    for (provider_id, provider) in bundled {
        if provider.models.is_empty() {
            continue;
        }

        for (model_id, model) in &provider.models {
            let media_generation = model.modalities.as_ref().is_some_and(|modalities| {
                !modalities.output.is_empty()
                    && modalities
                        .output
                        .iter()
                        .all(|modality| matches!(modality.as_str(), "image" | "video"))
            });
            assert!(
                model.limit.context > 0
                    || media_generation
                    || documented_shiju_unknown_limit(&provider_id, model_id, "context"),
                "{provider_id}/{model_id} should have a token context limit unless it generates images/video"
            );
            let output_is_text = model
                .modalities
                .as_ref()
                .is_none_or(|modalities| modalities.output.iter().any(|item| item == "text"));
            if output_is_text {
                assert!(
                    model.limit.output > 0
                        || documented_shiju_unknown_limit(&provider_id, model_id, "output"),
                    "{provider_id}/{model_id} should have a text output limit"
                );
            }

            if let Some(modalities) = model.modalities.as_ref()
                && modalities
                    .output
                    .iter()
                    .any(|item| matches!(item.as_str(), "image" | "audio" | "video"))
                && !modalities.output.iter().any(|item| item == "text")
            {
                let converted = from_models_dev_model(&provider, model);
                assert!(
                    !converted.capabilities.output.text,
                    "{provider_id}/{model_id} must keep media-only output non-text"
                );
            }
        }
    }
}

#[test]
fn init_is_idempotent() {
    init();
    init();
}

// Only documented model-specific exceptions may omit a token limit.
fn documented_shiju_unknown_limit(provider: &str, model: &str, field: &str) -> bool {
    if provider != "shiju" {
        return false;
    }
    let evidence: Value =
        serde_json::from_str(include_str!("../assets/model/shiju_official_sources.json")).unwrap();
    evidence["models"][model]["limit"][field].as_u64() == Some(0)
}

#[test]
fn shiju_official_models_preserve_documented_limits_and_modalities() {
    let evidence: Value =
        serde_json::from_str(include_str!("../assets/model/shiju_official_sources.json")).unwrap();
    let catalog = bundled_models("official_sources");
    let provider = &catalog["shiju"];
    assert_eq!(evidence["models"].as_object().unwrap().len(), 17);
    for (id, source) in evidence["models"].as_object().unwrap() {
        let model = &provider.models[id];
        assert_eq!(model.limit.context, source["limit"]["context"].as_u64().unwrap(), "{id}");
        assert_eq!(model.limit.output, source["limit"]["output"].as_u64().unwrap(), "{id}");
        assert_eq!(model.limit.input, source["limit"]["input"].as_u64(), "{id}");
        assert_eq!(serde_json::to_value(&model.modalities).unwrap(), source["modalities"], "{id}");
        assert!(!source["sources"].as_array().unwrap().is_empty());
        let runtime = from_models_dev_model(provider, model);
        assert_eq!(runtime.limit.context, model.limit.context, "{id}");
        if id == "kimi/kimi-k2.8-preview" {
            assert_eq!(runtime.limit.output, 4096);
        } else {
            assert_eq!(runtime.limit.output, model.limit.output, "{id}");
            assert_eq!(runtime.limit.input, model.limit.input, "{id}");
            assert!(!runtime.capabilities.toolcall);
        }
    }
}
