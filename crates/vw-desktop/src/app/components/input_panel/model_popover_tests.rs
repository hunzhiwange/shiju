//! 验证模型选择弹窗的分组、搜索、能力标识与选择消息。
//! 同时约束限定模型名、裸模型名、Unicode 截断及刷新时保留旧目录的回退。

use super::model_popover::{
    model_is_selected, model_matches_query, model_popover_content, model_provider_icon_id,
    model_toggle_button, model_toggle_label, normalize_model_input, truncated_model_name,
};
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use iced::Length;

// 模型 fixture 默认开启工具能力；需要验证能力徽标时再显式覆盖各能力位。
fn model(id: &str, name: &str, enabled: bool) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: serde_json::json!({}),
    }
}

fn model_with_caps(
    id: &str,
    name: &str,
    enabled: bool,
    toolcall: bool,
    attachment: bool,
) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        toolcall,
        attachment,
        context_limit: 8_192,
        detail: serde_json::json!({}),
    }
}

fn provider(id: &str, name: &str, models: Vec<ModelSummary>) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: name.to_string(), models }
}

#[test]
fn normalize_model_input_defaults_blank_values_to_auto() {
    assert_eq!(normalize_model_input(""), "auto");
    assert_eq!(normalize_model_input("   "), "auto");
    assert_eq!(normalize_model_input(" openai/gpt-4.1 "), "openai/gpt-4.1");
}

#[test]
fn model_matches_query_searches_provider_and_model_fields() {
    assert!(model_matches_query("", "openai", "OpenAI", "gpt-4.1", "GPT 4.1"));
    assert!(model_matches_query("open", "openai", "OpenAI", "gpt-4.1", "GPT 4.1"));
    assert!(model_matches_query("provider", "custom", "Provider Name", "x", "Y"));
    assert!(model_matches_query("gpt", "openai", "OpenAI", "gpt-4.1", "Other"));
    assert!(model_matches_query("vision", "openai", "OpenAI", "x", "Vision Model"));
    assert!(!model_matches_query("claude", "openai", "OpenAI", "gpt-4.1", "GPT 4.1"));
}

#[test]
fn model_is_selected_supports_provider_and_bare_model_ids() {
    assert!(model_is_selected("openai/gpt-4.1", "openai", "gpt-4.1"));
    assert!(model_is_selected("gpt-4.1", "openai", "gpt-4.1"));
    assert!(!model_is_selected("anthropic/gpt-4.1", "openai", "gpt-4.1"));
    assert!(!model_is_selected("other", "openai", "gpt-4.1"));
}

#[test]
fn truncated_model_name_keeps_short_names_and_truncates_long_names() {
    assert_eq!(truncated_model_name("GPT 4.1"), "GPT 4.1");

    let truncated = truncated_model_name("A very long model display name that will be truncated");
    assert_eq!(truncated.chars().count(), 26);
    assert!(truncated.ends_with('…'));
}

#[test]
fn truncated_model_name_handles_exact_and_over_limit_unicode_boundaries() {
    let exact = "模".repeat(26);
    let over = "模".repeat(27);

    assert_eq!(truncated_model_name(&exact), exact);
    let truncated = truncated_model_name(&over);
    assert_eq!(truncated.chars().count(), 26);
    assert_eq!(truncated.chars().last(), Some('…'));
}

#[test]
fn model_toggle_helpers_resolve_labels_and_provider_icons() {
    let providers = vec![provider(
        "openai",
        "OpenAI",
        vec![model("gpt-4.1", "GPT 4.1", true), model("o3", "O3", true)],
    )];

    assert_eq!(model_toggle_label(&providers, true, "openai/gpt-4.1"), "自动模型");
    assert_eq!(model_toggle_label(&providers, false, "openai/gpt-4.1"), "GPT 4.1");
    assert_eq!(model_toggle_label(&providers, false, "gpt-4.1"), "GPT 4.1");
    assert_eq!(model_toggle_label(&providers, false, "custom/model"), "model");
    assert_eq!(model_toggle_label(&providers, false, "unknown"), "unknown");

    assert_eq!(model_provider_icon_id(&providers, "openai/gpt-4.1"), "openai");
    assert_eq!(model_provider_icon_id(&providers, "gpt-4.1"), "openai");
    assert_eq!(model_provider_icon_id(&providers, "unknown"), "agent");
    assert_eq!(model_provider_icon_id(&providers, "/gpt-4.1"), "");
}

#[test]
fn model_toggle_helpers_fall_back_when_known_provider_lacks_requested_model() {
    let providers = vec![provider(
        "openai",
        "OpenAI",
        vec![model("gpt-4.1", "GPT 4.1", true), model("disabled", "Disabled Model", false)],
    )];

    assert_eq!(model_toggle_label(&providers, false, "openai/missing"), "missing");
    assert_eq!(model_toggle_label(&providers, false, "openai/disabled"), "Disabled Model");
    assert_eq!(model_provider_icon_id(&providers, "openai/missing"), "openai");
    assert_eq!(model_provider_icon_id(&providers, "openai/nested/model"), "openai");
}

#[test]
fn model_toggle_button_uses_auto_label_and_expanded_state() {
    let app = crate::app::App::new().0;

    let auto = model_toggle_button(&app, true, "openai/gpt-4.1", false);
    let expanded = model_toggle_button(&app, false, "openai/gpt-4.1", true);

    assert_eq!(auto.as_widget().size().width, Length::Shrink);
    assert_eq!(auto.as_widget().children().len(), 1);
    assert_eq!(expanded.as_widget().size().width, Length::Shrink);
    assert_eq!(expanded.as_widget().children().len(), 1);
}

#[test]
fn model_toggle_button_resolves_provider_model_and_bare_model_names() {
    let mut app = crate::app::App::new().0;
    app.model_settings.providers = vec![provider(
        "openai",
        "OpenAI",
        vec![model("gpt-4.1", "GPT 4.1", true), model("z-disabled", "Disabled", false)],
    )];

    let provider_model = model_toggle_button(&app, false, "openai/gpt-4.1", false);
    let bare_model = model_toggle_button(&app, false, "gpt-4.1", false);
    let fallback = model_toggle_button(&app, false, "custom/model", false);
    let bare_fallback = model_toggle_button(&app, false, "unknown", false);

    assert_eq!(provider_model.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_model.as_widget().size().width, Length::Shrink);
    assert_eq!(fallback.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_fallback.as_widget().size().width, Length::Shrink);
}

#[test]
fn model_popover_content_renders_loading_empty_and_filtered_states() {
    let mut app = crate::app::App::new().0;

    app.model_settings.loading = true;
    let loading = model_popover_content(&app, true, "auto", false);
    assert_eq!(loading.as_widget().size().width, Length::Fixed(320.0));
    drop(loading);

    app.model_settings.loading = false;
    let empty = model_popover_content(&app, false, "missing", true);
    assert_eq!(empty.as_widget().size().width, Length::Fixed(320.0));
    drop(empty);

    app.model_settings.providers = vec![
        provider(
            "openai",
            "OpenAI",
            vec![
                model("gpt-4.1", "GPT 4.1", true),
                model_with_caps("vision", "Vision Attachments", true, false, true),
                model(
                    "a-very-long-model-id",
                    "A very long model display name that will be truncated",
                    true,
                ),
            ],
        ),
        provider("disabled", "Disabled", vec![model("hidden", "Hidden", false)]),
    ];
    app.model_settings.query = String::new();
    let unfiltered = model_popover_content(&app, true, "gpt-4.1", true);
    assert_eq!(unfiltered.as_widget().size().width, Length::Fixed(320.0));
    drop(unfiltered);

    app.model_settings.query = "gpt".to_string();
    let filtered = model_popover_content(&app, false, "openai/gpt-4.1", false);
    assert_eq!(filtered.as_widget().size().width, Length::Fixed(320.0));
    drop(filtered);

    app.model_settings.query = "vision".to_string();
    let capability_flags = model_popover_content(&app, false, "openai/vision", false);
    assert_eq!(capability_flags.as_widget().size().width, Length::Fixed(320.0));
    drop(capability_flags);

    let bare_selected = model_popover_content(&app, false, "gpt-4.1", true);
    assert_eq!(bare_selected.as_widget().size().width, Length::Fixed(320.0));
    drop(bare_selected);

    app.model_settings.query = "no-match".to_string();
    let no_match = model_popover_content(&app, false, "openai/gpt-4.1", false);
    assert_eq!(no_match.as_widget().size().width, Length::Fixed(320.0));
}

#[test]
fn model_popover_content_keeps_available_models_visible_while_refreshing() {
    let mut app = crate::app::App::new().0;
    app.model_settings.loading = true;
    app.model_settings.providers =
        vec![provider("openai", "OpenAI", vec![model("gpt-4.1", "GPT 4.1", true)])];

    let content = model_popover_content(&app, false, "openai/gpt-4.1", false);

    assert_eq!(content.as_widget().size().width, Length::Fixed(320.0));
}
