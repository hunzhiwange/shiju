// 验证提供商摘要的排序、连接判定与展示信息生成。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use std::collections::HashMap;
use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};

fn model(provider_id: &str, model_id: &str, name: &str) -> Model {
    Model {
        id: model_id.to_string(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            id: model_id.to_string(),
            url: "https://example.test".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: name.to_string(),
        family: None,
        capabilities: Capabilities {
            temperature: true,
            reasoning: false,
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
        limit: ModelLimit { context: 8_000, input: None, output: 1_000 },
        status: "stable".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: String::new(),
        variants: HashMap::new(),
    }
}

fn provider(id: &str, name: &str, source: ProviderSource, models: Vec<Model>) -> Info {
    Info {
        id: id.to_string(),
        name: name.to_string(),
        source,
        env: Vec::new(),
        key: None,
        options: HashMap::new(),
        models: models.into_iter().map(|model| (model.id.clone(), model)).collect(),
    }
}

fn raw_model(id: &str, name: &str) -> raw_model_provider::Model {
    raw_model_provider::Model {
        id: id.to_string(),
        name: name.to_string(),
        ..serde_json::from_value(serde_json::json!({})).expect("empty raw model fixture is valid")
    }
}

fn raw_provider(
    id: &str,
    name: &str,
    models: Vec<raw_model_provider::Model>,
) -> raw_model_provider::Provider {
    raw_model_provider::Provider {
        api: None,
        name: name.to_string(),
        env: Vec::new(),
        id: id.to_string(),
        adapter: None,
        models: models.into_iter().map(|model| (model.id.clone(), model)).collect(),
    }
}

fn catalog_values(catalog: &[ModelCatalogEntry]) -> Vec<(&str, &str, &str, &str)> {
    catalog
        .iter()
        .map(|entry| {
            (
                entry.provider_id.as_str(),
                entry.provider_name.as_str(),
                entry.model_id.as_str(),
                entry.model_name.as_str(),
            )
        })
        .collect()
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn provider_id_validation_accepts_supported_characters_and_rejects_invalid_values() {
    for valid in ["a", "provider-2", "my_provider"] {
        assert!(is_valid_provider_id(valid), "expected valid provider id: {valid}");
    }

    for invalid in ["", "OpenAI", "UPPER", "bad id", "bad.id", "提供商"] {
        assert!(!is_valid_provider_id(invalid), "expected invalid provider id: {invalid}");
    }
}

#[test]
fn summarize_providers_maps_every_source_and_sorts_by_name_then_id() {
    let providers = HashMap::from([
        (
            "map-key-custom".to_string(),
            provider("custom", "Zulu", ProviderSource::Custom, Vec::new()),
        ),
        (
            "map-key-config".to_string(),
            provider("config", "Same", ProviderSource::Config, Vec::new()),
        ),
        ("map-key-env".to_string(), provider("env", "Same", ProviderSource::Env, Vec::new())),
        ("map-key-api".to_string(), provider("api", "Alpha", ProviderSource::Api, Vec::new())),
    ]);

    let summaries = summarize_providers(providers);
    let actual = summaries
        .iter()
        .map(|summary| {
            (
                summary.id.as_str(),
                summary.name.as_str(),
                summary.source_label.as_str(),
                summary.connected,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            ("api", "Alpha", "API 密钥", true),
            ("config", "Same", "配置", false),
            ("env", "Same", "环境变量", true),
            ("custom", "Zulu", "内置", false),
        ]
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn summarize_providers_marks_keyed_custom_provider_as_connected() {
    let mut custom = provider("custom", "Custom", ProviderSource::Custom, Vec::new());
    custom.key = Some("configured-secret".to_string());

    let summaries = summarize_providers(HashMap::from([(custom.id.clone(), custom)]));

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].connected);
}

#[test]
fn load_popular_providers_from_config_returns_unconfigured_empty_fallback() {
    let config = vw_config_types::config::Config {
        popular_providers: vec!["openai".to_string()],
        ..Default::default()
    };

    assert_eq!(load_popular_providers_from_config(&config), (Vec::new(), false));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn provider_info_catalog_filters_blank_ids_and_sorts_all_tie_breakers() {
    let providers = HashMap::from([
        (
            "z-map-key".to_string(),
            provider(
                "z-provider",
                "Same Provider",
                ProviderSource::Api,
                vec![model("z-provider", "z-model", "Same Model")],
            ),
        ),
        (
            "a-map-key".to_string(),
            provider(
                "a-provider",
                "Same Provider",
                ProviderSource::Api,
                vec![
                    model("a-provider", "z-model", "Same Model"),
                    model("a-provider", "a-model", "Same Model"),
                    model("a-provider", "middle", "Alpha Model"),
                    model("a-provider", "   ", "Ignored Model"),
                ],
            ),
        ),
        (
            "blank-map-key".to_string(),
            provider(
                " ",
                "First Provider",
                ProviderSource::Api,
                vec![model(" ", "visible-model", "Ignored Provider")],
            ),
        ),
    ]);

    let catalog = build_catalog_from_provider_infos(&providers);

    assert_eq!(
        catalog_values(&catalog),
        vec![
            ("a-provider", "Same Provider", "middle", "Alpha Model"),
            ("a-provider", "Same Provider", "a-model", "Same Model"),
            ("a-provider", "Same Provider", "z-model", "Same Model"),
            ("z-provider", "Same Provider", "z-model", "Same Model"),
        ]
    );
}

#[test]
fn source_catalog_filters_blank_ids_and_provider_infos_override_matching_raw_entries() {
    let raw_providers = HashMap::from([
        (
            "raw-map-key".to_string(),
            raw_provider(
                "shared",
                "Raw Provider",
                vec![
                    raw_model("duplicate", "Raw Duplicate"),
                    raw_model("raw-only", "Raw Only"),
                    raw_model(" ", "Ignored Raw Model"),
                ],
            ),
        ),
        (
            "blank-map-key".to_string(),
            raw_provider(" ", "Ignored Raw Provider", vec![raw_model("model", "Ignored")]),
        ),
    ]);
    let providers = HashMap::from([(
        "info-map-key".to_string(),
        provider(
            "shared",
            "Info Provider",
            ProviderSource::Api,
            vec![
                model("shared", "duplicate", "Info Duplicate"),
                model("shared", "info-only", "Info Only"),
            ],
        ),
    )]);

    let catalog = build_catalog_from_sources(&raw_providers, &providers);

    assert_eq!(
        catalog_values(&catalog),
        vec![
            ("shared", "Info Provider", "duplicate", "Info Duplicate"),
            ("shared", "Info Provider", "info-only", "Info Only"),
            ("shared", "Raw Provider", "raw-only", "Raw Only"),
        ]
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn empty_provider_sources_produce_an_empty_catalog() {
    assert!(build_catalog_from_provider_infos(&HashMap::new()).is_empty());
    assert!(build_catalog_from_sources(&HashMap::new(), &HashMap::new()).is_empty());
}
