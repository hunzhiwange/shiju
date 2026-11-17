use super::*;
use crate::state::{ModelDetailRow, ModelSummary};
use serde_json::{Value, json};
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

fn row_value(rows: &[ModelDetailRow], label: &str) -> String {
    rows.iter()
        .find(|row| row.label.display(config_ui::DesktopLocale::SimplifiedChinese) == label)
        .unwrap_or_else(|| panic!("missing detail row: {label}"))
        .value.display(config_ui::DesktopLocale::SimplifiedChinese)
}

#[test]
fn models_refresh_clears_transient_state_and_success_replaces_providers() {
    let mut app = crate::state::ModelSettingsState::default();
    app.save_error = Some(("stale".to_string()).into());
    app.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let _ = update(&mut app, ModelMessage::ModelsRefresh);

    assert!(app.loading);
    assert!(app.save_error.is_none());
    assert!(app.detail_modal.is_none());

    let _ = update(&mut app, ModelMessage::ModelsRefreshed(Ok(Vec::new())));
    assert!(!app.loading);
    assert!(app.providers.is_empty());
    assert!(app.detail_modal.is_none());
}

#[test]
fn models_refresh_error_stops_loading_and_records_error() {
    let mut app = crate::state::ModelSettingsState::default();
    app.loading = true;
    app.providers = vec![provider_summary()];

    let _ = update(&mut app, ModelMessage::ModelsRefreshed(Err("network".to_string().into())));

    assert!(!app.loading);
    assert_eq!(app.save_error.as_deref(), Some("network"));
    assert_eq!(app.providers.len(), 1);
    assert!(app.detail_modal.is_none());
}

#[test]
fn model_query_changed_preserves_input_verbatim() {
    let mut app = crate::state::ModelSettingsState::default();

    let _ = update(&mut app, ModelMessage::ModelQueryChanged("  GPT-5 / 图像  ".to_string()));

    assert_eq!(app.query, "  GPT-5 / 图像  ");
}

#[test]
fn provider_model_groups_start_collapsed_and_toggle_independently() {
    let mut app = crate::state::ModelSettingsState::default();
    assert!(app.expanded_provider_ids.is_empty());

    let _ = update(&mut app, ModelMessage::ProviderModelsExpansionToggled("openai".to_string()));
    assert!(app.expanded_provider_ids.contains("openai"));

    let _ = update(&mut app, ModelMessage::ProviderModelsExpansionToggled("openai".to_string()));
    assert!(!app.expanded_provider_ids.contains("openai"));
}

#[test]
fn model_detail_open_reports_missing_provider_and_missing_model() {
    let mut app = crate::state::ModelSettingsState::default();
    app.providers = vec![provider_summary()];

    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("missing".to_string(), "gpt-5".to_string()),
    );
    assert_eq!(app.save_error.as_deref(), Some("未找到提供商"));
    assert!(app.detail_modal.is_none());

    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("openai".to_string(), "missing".to_string()),
    );
    assert_eq!(app.save_error.as_deref(), Some("未找到模型"));
    assert!(app.detail_modal.is_none());
}

#[test]
fn model_detail_open_maps_complete_detail_into_human_readable_rows() {
    let mut app = crate::state::ModelSettingsState::default();
    app.providers = vec![provider_summary()];

    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let modal = app.detail_modal.as_ref().expect("detail modal");
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
    let mut app = crate::state::ModelSettingsState::default();
    app.providers = vec![provider_summary_with_detail(json!({
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
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let rows = &app.detail_modal.as_ref().expect("detail modal").rows;
    assert_eq!(row_value(rows, "状态 (status)"), "");
    assert_eq!(row_value(rows, "API Base URL (api.url)"), "");
    assert_eq!(row_value(rows, "上下文限制 (limit.context)"), "0");
    assert_eq!(row_value(rows, "输入限制 (limit.input)"), "-");
    assert_eq!(row_value(rows, "支持工具调用 (capabilities.toolcall)"), "不支持");
    assert_eq!(row_value(rows, "输入模态 (capabilities.input.*)"), "无");
    assert_eq!(row_value(rows, "输出模态 (capabilities.output.*)"), "无");
    assert_eq!(row_value(rows, "输入价格 (cost.input)"), "0");
    assert_eq!(row_value(rows, "请求头数量 (headers)"), "0");
    assert!(!rows.iter().any(|row| row.label.display(config_ui::DesktopLocale::SimplifiedChinese) == "混合输入 (capabilities.interleaved)"));
}

#[test]
fn model_detail_open_renders_boolean_interleaved_capability() {
    let mut app = crate::state::ModelSettingsState::default();
    app.providers = vec![provider_summary_with_detail(json!({
        "capabilities": { "interleaved": false }
    }))];

    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let rows = &app.detail_modal.as_ref().expect("detail modal").rows;
    assert_eq!(row_value(rows, "混合输入 (capabilities.interleaved)"), "false");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn model_detail_toggle_is_safe_without_modal_and_reversible_with_modal() {
    let mut app = crate::state::ModelSettingsState::default();

    let _ = update(&mut app, ModelMessage::ModelDetailToggleRaw);
    assert!(app.detail_modal.is_none());

    app.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );
    let _ = update(&mut app, ModelMessage::ModelDetailToggleRaw);
    assert!(app.detail_modal.as_ref().expect("detail modal").show_raw);

    let _ = update(&mut app, ModelMessage::ModelDetailToggleRaw);
    assert!(!app.detail_modal.as_ref().expect("detail modal").show_raw);

    let _ = update(&mut app, ModelMessage::ModelDetailClose);
    assert!(app.detail_modal.is_none());
}
