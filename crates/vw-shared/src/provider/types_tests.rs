//! 提供商与模型公共类型的解析、排序及线格式测试。
//!
//! 覆盖默认适配器、模型标识拆分、缺省字段兼容和能力配置反序列化。

#[test]
fn default_adapter_matches_openai_compatible_wire_value() {
    assert_eq!(super::default_adapter(), "openai-compatible");
}

#[test]
fn api_info_deserialization_fills_default_adapter() {
    let api: super::ApiInfo =
        serde_json::from_str(r#"{"id":"main","url":"https://example.test"}"#).unwrap();

    assert_eq!(api.adapter, "openai-compatible");
}

#[test]
fn parse_model_preserves_model_slashes() {
    let parsed = super::parse_model("provider/family/model");

    assert_eq!(parsed.provider_id, "provider");
    assert_eq!(parsed.model_id, "family/model");
}

#[test]
fn model_not_found_display_includes_suggestions_when_available() {
    let error = super::ModelNotFoundError {
        provider_id: "openai".to_string(),
        model_id: "missing".to_string(),
        suggestions: vec!["gpt-5".to_string(), "gpt-5-mini".to_string()],
    };

    let message = error.to_string();

    assert!(message.contains("openai/missing"));
    assert!(message.contains("gpt-5, gpt-5-mini"));
}

#[test]
fn model_not_found_display_omits_suggestion_text_when_empty() {
    let error = super::ModelNotFoundError {
        provider_id: "openai".to_string(),
        model_id: "missing".to_string(),
        suggestions: Vec::new(),
    };

    assert_eq!(error.to_string(), "未找到模型：openai/missing");
}

#[test]
fn parse_model_uses_empty_model_when_separator_is_absent() {
    let parsed = super::parse_model("provider");

    assert_eq!(parsed.provider_id, "provider");
    assert_eq!(parsed.model_id, "");
}

#[test]
fn parse_model_allows_empty_provider_and_model() {
    let parsed = super::parse_model("");

    assert_eq!(parsed.provider_id, "");
    assert_eq!(parsed.model_id, "");
}

#[test]
fn provider_source_serializes_lowercase_wire_values() {
    let sources = [
        (super::ProviderSource::Env, "\"env\""),
        (super::ProviderSource::Config, "\"config\""),
        (super::ProviderSource::Custom, "\"custom\""),
        (super::ProviderSource::Api, "\"api\""),
    ];

    for (source, expected) in sources {
        let serialized = serde_json::to_string(&source).unwrap();

        assert_eq!(serialized, expected);
    }
}

#[test]
fn info_deserialization_fills_optional_defaults() {
    let info: super::Info = serde_json::from_str(
        r#"{
            "id": "openai",
            "name": "OpenAI",
            "source": "api",
            "env": [],
            "models": {}
        }"#,
    )
    .unwrap();

    assert_eq!(info.id, "openai");
    assert!(matches!(info.source, super::ProviderSource::Api));
    assert!(info.key.is_none());
    assert!(info.options.is_empty());
    assert!(info.models.is_empty());
}

#[test]
fn model_deserialization_fills_default_maps_and_optional_fields() {
    let model: super::Model = serde_json::from_str(
        r#"{
            "id": "gpt-5",
            "providerID": "openai",
            "api": {
                "id": "main",
                "url": "https://example.test"
            },
            "name": "GPT-5",
            "capabilities": {
                "temperature": true,
                "reasoning": true,
                "attachment": false,
                "toolcall": true,
                "input": {
                    "text": true,
                    "audio": false,
                    "image": true,
                    "video": false,
                    "pdf": true
                },
                "output": {
                    "text": true,
                    "audio": false,
                    "image": false,
                    "video": false,
                    "pdf": false
                },
                "interleaved": {
                    "field": "messages"
                }
            },
            "cost": {
                "input": 1.25,
                "output": 10.0,
                "cache": {
                    "read": 0.125,
                    "write": 1.25
                }
            },
            "limit": {
                "context": 400000,
                "output": 128000
            },
            "status": "stable",
            "release_date": "2026-01-01"
        }"#,
    )
    .unwrap();

    assert_eq!(model.provider_id, "openai");
    assert_eq!(model.api.adapter, "openai-compatible");
    assert!(model.family.is_none());
    assert!(model.options.is_empty());
    assert!(model.headers.is_empty());
    assert!(model.variants.is_empty());
    assert!(model.limit.input.is_none());
    assert!(model.cost.experimental_over_200k.is_none());
    assert!(matches!(
        model.capabilities.interleaved,
        super::InterleavedCapability::Field { ref field } if field == "messages"
    ));
}

#[test]
fn interleaved_capability_accepts_bool_wire_value() {
    let capability: super::InterleavedCapability = serde_json::from_str("true").unwrap();

    assert!(matches!(capability, super::InterleavedCapability::Bool(true)));
}

#[test]
fn model_deserialization_accepts_explicit_over_200k_cost_and_input_limit() {
    let model: super::Model = serde_json::from_str(
        r#"{
            "id": "claude-sonnet-4",
            "providerID": "anthropic",
            "api": {
                "id": "main",
                "url": "https://example.test",
                "adapter": "anthropic"
            },
            "name": "Claude Sonnet 4",
            "family": "claude",
            "capabilities": {
                "temperature": false,
                "reasoning": true,
                "attachment": true,
                "toolcall": true,
                "input": {
                    "text": true,
                    "audio": false,
                    "image": true,
                    "video": false,
                    "pdf": true
                },
                "output": {
                    "text": true,
                    "audio": false,
                    "image": false,
                    "video": false,
                    "pdf": false
                },
                "interleaved": false
            },
            "cost": {
                "input": 3.0,
                "output": 15.0,
                "cache": {
                    "read": 0.3,
                    "write": 3.75
                },
                "experimental_over_200k": {
                    "input": 6.0,
                    "output": 22.5,
                    "cache": {
                        "read": 0.6,
                        "write": 7.5
                    }
                }
            },
            "limit": {
                "context": 1000000,
                "input": 900000,
                "output": 64000
            },
            "status": "stable",
            "options": {
                "reasoning_effort": "high"
            },
            "headers": {
                "x-provider": "anthropic"
            },
            "release_date": "2026-02-01",
            "variants": {
                "fast": {
                    "temperature": 0
                }
            }
        }"#,
    )
    .unwrap();

    let over_200k = model.cost.experimental_over_200k.unwrap();

    assert_eq!(model.family.as_deref(), Some("claude"));
    assert_eq!(model.limit.input, Some(900000));
    assert_eq!(over_200k.cache.write, 7.5);
    assert_eq!(model.options["reasoning_effort"], "high");
    assert_eq!(model.headers["x-provider"], "anthropic");
    assert_eq!(model.variants["fast"]["temperature"], 0);
    assert!(matches!(model.capabilities.interleaved, super::InterleavedCapability::Bool(false)));
}

#[test]
fn sort_prefers_priority_then_latest_then_descending_id() {
    let sorted_ids = super::sort(vec![
        model("zeta"),
        model("alpha-latest"),
        model("gpt-5-mini"),
        model("claude-sonnet-4-latest"),
        model("gpt-5"),
        model("beta"),
        model("gemini-3-pro"),
        model("big-pickle-preview"),
    ])
    .into_iter()
    .map(|model| model.id)
    .collect::<Vec<_>>();

    assert_eq!(
        sorted_ids,
        vec![
            "gpt-5-mini",
            "gpt-5",
            "claude-sonnet-4-latest",
            "big-pickle-preview",
            "gemini-3-pro",
            "alpha-latest",
            "zeta",
            "beta",
        ]
    );
}

fn model(id: &str) -> super::Model {
    super::Model {
        id: id.to_string(),
        provider_id: "provider".to_string(),
        api: super::ApiInfo {
            id: "main".to_string(),
            url: "https://example.test".to_string(),
            adapter: super::default_adapter(),
        },
        name: id.to_string(),
        family: None,
        capabilities: super::Capabilities {
            temperature: true,
            reasoning: false,
            attachment: false,
            toolcall: true,
            input: capability_io(),
            output: capability_io(),
            interleaved: super::InterleavedCapability::Bool(false),
        },
        cost: super::ModelCost {
            input: 1.0,
            output: 2.0,
            cache: super::ModelCostCache { read: 0.1, write: 0.2 },
            experimental_over_200k: None,
        },
        limit: super::ModelLimit { context: 128000, input: None, output: 8192 },
        status: "stable".to_string(),
        options: Default::default(),
        headers: Default::default(),
        release_date: "2026-01-01".to_string(),
        variants: Default::default(),
    }
}

fn capability_io() -> super::CapabilityIO {
    super::CapabilityIO { text: true, audio: false, image: false, video: false, pdf: false }
}
