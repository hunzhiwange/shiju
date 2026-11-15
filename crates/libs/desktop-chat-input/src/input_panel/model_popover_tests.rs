//! 验证模型选择弹窗的分组、搜索、能力标识与选择消息。
//! 同时约束限定模型名、裸模型名、Unicode 截断及刷新时保留旧目录的回退。

use super::{
    model_is_selected, model_matches_query, model_provider_icon_id,
    model_toggle_label, normalize_model_input, truncated_model_name,
};
use desktop_settings_providers::state::{ModelSummary, ProviderModelsSummary};

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
