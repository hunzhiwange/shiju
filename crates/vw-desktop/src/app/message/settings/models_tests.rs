// 验证模型摘要对提供商连接状态、能力与排序的映射。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use serde_json::{Value, json};
use std::collections::HashMap;
use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};

fn app() -> App {
    App::new().0
}

fn model(provider_id: &str, id: &str, name: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            id: id.to_string(),
            url: "https://example.test".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: name.to_string(),
        family: None,
        capabilities: Capabilities {
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

fn provider_summary_with_detail(detail: Value) -> ProviderModelsSummary {
    ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![ModelSummary {
            id: "gpt-5".to_string(),
            name: "GPT-5".to_string(),
            enabled: true,
            toolcall: true,
            attachment: true,
            context_limit: 200_000,
            detail,
        }],
    }
}

fn provider_summary() -> ProviderModelsSummary {
    provider_summary_with_detail(json!({
        "id": "gpt-5",
        "name": "GPT-5",
        "status": "active",
        "release_date": "2026-01-01",
        "api": { "url": "https://api.example.com", "adapter": "openai" },
        "limit": { "context": 200000, "input": 100000, "output": 20000 },
        "capabilities": {
            "toolcall": true,
            "attachment": true,
            "reasoning": true,
            "temperature": true,
            "input": { "text": true, "audio": false, "image": true, "video": false, "pdf": true },
            "output": { "text": true, "audio": false, "image": false, "video": false, "pdf": false },
            "interleaved": { "field": "content" }
        },
        "cost": { "input": 1.0, "output": 2.0, "cache": { "read": 0.5, "write": 0.75 } },
        "headers": { "x-test-header": "configured" },
        "options": { "reasoning_effort": "high" },
        "variants": { "fast": { "tier": "low-latency" } }
    }))
}

fn row_value<'a>(rows: &'a [ModelDetailRow], label: &str) -> &'a str {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("missing detail row: {label}"))
        .value
        .as_str()
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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
fn models_refresh_clears_transient_state_and_success_replaces_providers() {
    let mut app = app();
    app.model_settings.save_error = Some("stale".to_string());
    app.model_settings.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let _ = update(&mut app, SettingsMessage::ModelsRefresh);

    assert!(app.model_settings.loading);
    assert!(app.model_settings.save_error.is_none());
    assert!(app.model_settings.detail_modal.is_none());

    let _ = update(&mut app, SettingsMessage::ModelsRefreshed(Ok(Vec::new())));
    assert!(!app.model_settings.loading);
    assert!(app.model_settings.providers.is_empty());
    assert!(app.model_settings.detail_modal.is_none());
}

#[test]
fn models_refresh_error_stops_loading_and_records_error() {
    let mut app = app();
    app.model_settings.loading = true;
    app.model_settings.providers = vec![provider_summary()];

    let _ = update(&mut app, SettingsMessage::ModelsRefreshed(Err("network".to_string())));

    assert!(!app.model_settings.loading);
    assert_eq!(app.model_settings.save_error.as_deref(), Some("network"));
    assert_eq!(app.model_settings.providers.len(), 1);
    assert!(app.model_settings.detail_modal.is_none());
}

#[test]
fn model_query_changed_preserves_input_verbatim() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::ModelQueryChanged("  GPT-5 / 图像  ".to_string()));

    assert_eq!(app.model_settings.query, "  GPT-5 / 图像  ");
}

#[test]
fn model_toggle_updates_locally_and_starts_save_without_reloading() {
    let mut app = app();
    app.auto_model = false;
    app.model = "another-model".to_string();
    app.model_settings.save_error = Some("stale".to_string());
    app.model_settings.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(!app.model_settings.loading);
    assert!(app.model_settings.saving);
    assert!(app.model_settings.save_error.is_none());
    assert!(app.model_settings.detail_modal.is_none());
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert_eq!(app.model_settings.providers[0].models[0].detail["status"], "disabled");
    assert!(!app.auto_model);
}

#[test]
fn disabling_selected_model_switches_back_to_auto_model() {
    let mut app = app();
    app.auto_model = false;
    app.model = "openai/gpt-5".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(app.auto_model);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn enabling_model_updates_locally_and_schedules_save_without_forcing_auto_model() {
    let mut app = app();
    app.auto_model = false;
    app.model = "gpt-5".to_string();
    let mut provider = provider_summary();
    provider.models[0].enabled = false;
    provider.models[0].detail["status"] = json!("disabled");
    app.model_settings.providers = vec![provider];

    let task = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), true),
    );

    assert!(!app.model_settings.loading);
    assert!(app.model_settings.saving);
    assert!(app.model_settings.providers[0].models[0].enabled);
    assert_eq!(app.model_settings.providers[0].models[0].detail["status"], "active");
    assert!(!app.auto_model);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
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
    let mut config = vw_config_types::config::Config::default();
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

#[test]
fn provider_set_all_only_changes_models_in_selected_group() {
    let mut app = app();
    let mut anthropic = provider_summary();
    anthropic.id = "anthropic".to_string();
    anthropic.name = "Anthropic".to_string();
    anthropic.models[0].id = "claude-sonnet".to_string();
    anthropic.models[0].name = "Claude Sonnet".to_string();
    app.model_settings.providers = vec![provider_summary(), anthropic];

    let task = update(&mut app, SettingsMessage::ProviderModelsSetAll("openai".to_string(), false));

    assert!(app.model_settings.saving);
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert!(app.model_settings.providers[1].models[0].enabled);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn provider_reset_optimistically_disables_only_selected_group() {
    let mut app = app();
    let mut anthropic = provider_summary();
    anthropic.id = "anthropic".to_string();
    anthropic.models[0].id = "claude-sonnet".to_string();
    app.model_settings.providers = vec![provider_summary(), anthropic];

    let task = update(&mut app, SettingsMessage::ProviderModelsReset("openai".to_string()));

    assert!(app.model_settings.saving);
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert!(app.model_settings.providers[1].models[0].enabled);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn provider_model_groups_start_collapsed_and_toggle_independently() {
    let mut app = app();
    assert!(app.model_settings.expanded_provider_ids.is_empty());

    let _ = update(&mut app, SettingsMessage::ProviderModelsExpansionToggled("openai".to_string()));
    assert!(app.model_settings.expanded_provider_ids.contains("openai"));

    let _ = update(&mut app, SettingsMessage::ProviderModelsExpansionToggled("openai".to_string()));
    assert!(!app.model_settings.expanded_provider_ids.contains("openai"));
}

#[test]
fn models_set_all_updates_every_connected_model_with_one_save() {
    let mut app = app();
    let mut provider = provider_summary();
    provider.models.push(ModelSummary {
        id: "gpt-4.1".to_string(),
        name: "GPT-4.1".to_string(),
        enabled: false,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: json!({ "id": "gpt-4.1", "status": "disabled" }),
    });
    app.model_settings.providers = vec![provider];

    let task = update(&mut app, SettingsMessage::ModelsSetAll(false));

    assert!(app.model_settings.saving);
    assert!(app.model_settings.providers[0].models.iter().all(|model| !model.enabled));
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn model_status_save_error_rolls_back_local_changes() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary()];
    let change = ModelStatusChange {
        provider_id: "openai".to_string(),
        model_id: "gpt-5".to_string(),
        previous_enabled: true,
        enabled: false,
    };

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle(
            change.provider_id.clone(),
            change.model_id.clone(),
            change.enabled,
        ),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelStatusesSaved(vec![change], Err("save failed".to_string())),
    );

    assert!(!app.model_settings.saving);
    assert!(app.model_settings.providers[0].models[0].enabled);
    assert_eq!(app.model_settings.providers[0].models[0].detail["status"], "active");
    assert_eq!(app.model_settings.save_error.as_deref(), Some("save failed"));
}

#[test]
fn model_status_save_success_keeps_optimistic_state() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary()];
    let change = ModelStatusChange {
        provider_id: "openai".to_string(),
        model_id: "gpt-5".to_string(),
        previous_enabled: true,
        enabled: false,
    };

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle(
            change.provider_id.clone(),
            change.model_id.clone(),
            change.enabled,
        ),
    );
    let _ = update(&mut app, SettingsMessage::ModelStatusesSaved(vec![change], Ok(())));

    assert!(!app.model_settings.saving);
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert!(app.model_settings.save_error.is_none());
}

#[test]
fn model_detail_open_reports_missing_provider_and_missing_model() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary()];

    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("missing".to_string(), "gpt-5".to_string()),
    );
    assert_eq!(app.model_settings.save_error.as_deref(), Some("未找到提供商"));
    assert!(app.model_settings.detail_modal.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "missing".to_string()),
    );
    assert_eq!(app.model_settings.save_error.as_deref(), Some("未找到模型"));
    assert!(app.model_settings.detail_modal.is_none());
}

#[test]
fn model_detail_open_maps_complete_detail_into_human_readable_rows() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary()];

    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let modal = app.model_settings.detail_modal.as_ref().expect("detail modal");
    assert_eq!(modal.provider_id, "openai");
    assert_eq!(modal.provider_name, "OpenAI");
    assert_eq!(modal.model_id, "gpt-5");
    assert_eq!(modal.model_name, "GPT-5");
    assert_eq!(row_value(&modal.rows, "状态 (status)"), "active");
    assert_eq!(row_value(&modal.rows, "API Base URL (api.url)"), "https://api.example.com");
    assert_eq!(row_value(&modal.rows, "输入限制 (limit.input)"), "100000");
    assert_eq!(row_value(&modal.rows, "支持推理 (capabilities.reasoning)"), "支持");
    assert_eq!(row_value(&modal.rows, "输入模态 (capabilities.input.*)"), "image, pdf, text");
    assert_eq!(row_value(&modal.rows, "输出模态 (capabilities.output.*)"), "text");
    assert_eq!(row_value(&modal.rows, "混合输入 (capabilities.interleaved)"), "field: content");
    assert_eq!(row_value(&modal.rows, "缓存写 (cost.cache.write)"), "0.75");
    assert_eq!(row_value(&modal.rows, "请求头数量 (headers)"), "1");
    assert_eq!(row_value(&modal.rows, "自定义选项数量 (options)"), "1");
    assert_eq!(row_value(&modal.rows, "变体数量 (variants)"), "1");
    assert!(modal.raw_json.contains("\"gpt-5\""));
    assert!(!modal.show_raw);
}

#[test]
fn model_detail_open_uses_defaults_for_missing_or_wrong_typed_values() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary_with_detail(json!({
        "status": 7,
        "api": "invalid",
        "limit": { "context": "large", "output": null },
        "capabilities": {
            "toolcall": "yes",
            "input": ["text"],
            "output": {},
            "interleaved": { "unexpected": true }
        },
        "cost": null,
        "headers": [],
        "options": null,
        "variants": "none"
    }))];

    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let rows = &app.model_settings.detail_modal.as_ref().expect("detail modal").rows;
    assert_eq!(row_value(rows, "状态 (status)"), "");
    assert_eq!(row_value(rows, "API Base URL (api.url)"), "");
    assert_eq!(row_value(rows, "上下文限制 (limit.context)"), "0");
    assert_eq!(row_value(rows, "输入限制 (limit.input)"), "-");
    assert_eq!(row_value(rows, "支持工具调用 (capabilities.toolcall)"), "不支持");
    assert_eq!(row_value(rows, "输入模态 (capabilities.input.*)"), "无");
    assert_eq!(row_value(rows, "输出模态 (capabilities.output.*)"), "无");
    assert_eq!(row_value(rows, "输入价格 (cost.input)"), "0");
    assert_eq!(row_value(rows, "请求头数量 (headers)"), "0");
    assert!(!rows.iter().any(|row| row.label == "混合输入 (capabilities.interleaved)"));
}

#[test]
fn model_detail_open_renders_boolean_interleaved_capability() {
    let mut app = app();
    app.model_settings.providers = vec![provider_summary_with_detail(json!({
        "capabilities": { "interleaved": false }
    }))];

    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let rows = &app.model_settings.detail_modal.as_ref().expect("detail modal").rows;
    assert_eq!(row_value(rows, "混合输入 (capabilities.interleaved)"), "false");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn model_detail_toggle_is_safe_without_modal_and_reversible_with_modal() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::ModelDetailToggleRaw);
    assert!(app.model_settings.detail_modal.is_none());

    app.model_settings.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::ModelDetailToggleRaw);
    assert!(app.model_settings.detail_modal.as_ref().expect("detail modal").show_raw);

    let _ = update(&mut app, SettingsMessage::ModelDetailToggleRaw);
    assert!(!app.model_settings.detail_modal.as_ref().expect("detail modal").show_raw);

    let _ = update(&mut app, SettingsMessage::ModelDetailClose);
    assert!(app.model_settings.detail_modal.is_none());
}
