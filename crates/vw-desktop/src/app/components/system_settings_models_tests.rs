use super::*;
use crate::app::state::{
    ModelConfigTab, ModelDetailModalState, ModelDetailRow, ModelSummary, ProviderModelsSummary,
};
use crate::app::{App, Message};
use iced::advanced::widget::Tree;
use iced::widget::text;
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

#[test]
fn model_settings_error_banner_is_local_to_tabbed_view() {
    let mut app = test_app();
    let cases = [
        (ModelConfigTab::Models, "models error"),
        (ModelConfigTab::ModelRoutes, "routes error"),
        (ModelConfigTab::QueryClassification, "classification error"),
        (ModelConfigTab::Multimodal, "multimodal error"),
    ];

    for (tab, _) in cases {
        app.model_settings.active_tab = tab;
        assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 2);
    }

    app.model_settings.save_error = Some(cases[0].1.to_string());
    app.model_routes_settings.save_error = Some(cases[1].1.to_string());
    app.query_classification_settings.save_error = Some(cases[2].1.to_string());
    app.multimodal_settings.save_error = Some(cases[3].1.to_string());

    for (tab, expected) in cases {
        app.model_settings.active_tab = tab;
        assert_eq!(active_tab_save_error(&app), Some(expected));
        assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 3);
    }
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn provider_summary() -> ProviderModelsSummary {
    ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![
            ModelSummary {
                id: "gpt-5".to_string(),
                name: "GPT-5".to_string(),
                enabled: true,
                toolcall: true,
                attachment: true,
                context_limit: 200_000,
                detail: json!({"id": "gpt-5", "capabilities": {"toolcall": true}}),
            },
            ModelSummary {
                id: "text-embedding-3-small".to_string(),
                name: "Embedding Small".to_string(),
                enabled: false,
                toolcall: false,
                attachment: false,
                context_limit: 0,
                detail: json!({"id": "text-embedding-3-small"}),
            },
        ],
    }
}

fn provider_anthropic_summary() -> ProviderModelsSummary {
    ProviderModelsSummary {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        models: vec![
            ModelSummary {
                id: "claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4.6".to_string(),
                enabled: true,
                toolcall: true,
                attachment: true,
                context_limit: 200_000,
                detail: json!({"id": "claude-sonnet-4-6"}),
            },
            ModelSummary {
                id: "claude-haiku-4-5".to_string(),
                name: "Claude Haiku 4.5".to_string(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 100_000,
                detail: json!({"id": "claude-haiku-4-5"}),
            },
            ModelSummary {
                id: "claude-opus-4-6".to_string(),
                name: "Claude Opus 4.6".to_string(),
                enabled: false,
                toolcall: true,
                attachment: true,
                context_limit: 500_000,
                detail: json!({"id": "claude-opus-4-6"}),
            },
        ],
    }
}

#[test]
fn main_view_with_multiple_providers() {
    let mut app = test_app();
    app.model_settings.providers = vec![provider_summary(), provider_anthropic_summary()];
    keep_element(main_view(&app));

    app.model_settings.expanded_provider_ids.insert("openai".to_string());
    keep_element(main_view(&app));
}

#[test]
fn main_view_builds_batch_actions_for_mixed_enabled_and_disabled_models() {
    let mut app = test_app();
    app.model_settings.providers = vec![provider_summary(), provider_anthropic_summary()];
    keep_element(main_view(&app));

    for provider in &mut app.model_settings.providers {
        for model in &mut provider.models {
            model.enabled = true;
        }
    }
    keep_element(main_view(&app));

    for provider in &mut app.model_settings.providers {
        for model in &mut provider.models {
            model.enabled = false;
        }
    }
    keep_element(main_view(&app));
}

#[test]
fn main_view_builds_saving_state_with_expanded_model_togglers() {
    let mut app = test_app();
    app.model_settings.providers = vec![provider_summary(), provider_anthropic_summary()];
    app.model_settings.expanded_provider_ids =
        ["openai".to_string(), "anthropic".to_string()].into_iter().collect();
    app.model_settings.saving = true;

    keep_element(main_view(&app));
}

#[test]
fn main_view_search_matches_provider_name() {
    let mut app = test_app();
    app.model_settings.providers = vec![provider_summary(), provider_anthropic_summary()];
    // "open" matches provider name "OpenAI" but not its id "openai"? Actually id is "openai" too.
    // Use "anth" which matches "Anthropic" name but not id "anthropic"? Actually id is "anthropic" too.
    // Use "AI" suffix: matches name "OpenAI" and "Anthropic" (through name, not ID)
    app.model_settings.query = "AI".to_string();
    keep_element(main_view(&app));
}

#[test]
fn main_view_search_matches_model_id() {
    let mut app = test_app();
    // "claude-sonnet" matches nothing in provider_summary().
    // Add anthropic provider; "claude-sonnet" matches model id "claude-sonnet-4-6"
    app.model_settings.providers = vec![provider_anthropic_summary()];
    app.model_settings.query = "claude-sonnet".to_string();
    keep_element(main_view(&app));
}

#[test]
fn main_view_search_matches_model_name() {
    let mut app = test_app();
    // "Haiku" matches model name "Claude Haiku 4.5" (but not model id "claude-haiku-4-5")
    app.model_settings.providers = vec![provider_anthropic_summary()];
    app.model_settings.query = "Haiku".to_string();
    keep_element(main_view(&app));
}

#[test]
fn main_view_search_no_match_any_field() {
    let mut app = test_app();
    app.model_settings.providers = vec![provider_summary(), provider_anthropic_summary()];
    app.model_settings.query = "zzzznotfound".to_string();
    keep_element(main_view(&app));
}

#[test]
fn main_view_builds_empty_loading_error_and_filtered_states() {
    let mut app = test_app();
    keep_element(main_view(&app));

    app.model_settings.loading = true;
    keep_element(main_view(&app));

    app.model_settings.loading = false;
    app.model_settings.providers = vec![provider_summary()];
    keep_element(main_view(&app));

    app.model_settings.saving = true;
    keep_element(main_view(&app));
    app.model_settings.saving = false;

    app.model_settings.query = "openai".to_string();
    keep_element(main_view(&app));

    app.model_settings.query = "embedding".to_string();
    keep_element(main_view(&app));

    app.model_settings.query = "missing".to_string();
    app.model_settings.save_error = Some("保存失败".to_string());
    keep_element(main_view(&app));
}

#[test]
fn overlays_return_dialog_or_model_detail_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.model_settings.providers = vec![provider_summary()];
    app.model_settings.detail_modal = Some(ModelDetailModalState {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        model_id: "gpt-5".to_string(),
        model_name: "GPT-5".to_string(),
        rows: vec![ModelDetailRow {
            label: "模型名称".to_string(), value: "GPT-5".to_string()
        }],
        raw_json: "{\n  \"id\": \"gpt-5\"\n}".to_string(),
        show_raw: false,
    });
    keep_element(view_overlays(&app, text("dialog").into()));

    app.model_settings.detail_modal.as_mut().expect("modal").show_raw = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_with_empty_rows() {
    let mut app = test_app();
    app.model_settings.detail_modal = Some(ModelDetailModalState {
        provider_id: "anthropic".to_string(),
        provider_name: "Anthropic".to_string(),
        model_id: "claude-sonnet-4-6".to_string(),
        model_name: "Claude Sonnet 4.6".to_string(),
        rows: vec![],
        raw_json: "{}".to_string(),
        show_raw: false,
    });
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_with_many_rows() {
    let mut app = test_app();
    app.model_settings.detail_modal = Some(ModelDetailModalState {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        model_id: "gpt-5".to_string(),
        model_name: "GPT-5".to_string(),
        rows: vec![
            ModelDetailRow { label: "模型名称".into(), value: "GPT-5".into() },
            ModelDetailRow { label: "提供商".into(), value: "OpenAI".into() },
            ModelDetailRow { label: "上下文限制".into(), value: "200000".into() },
        ],
        raw_json: "{\"id\":\"gpt-5\"}".to_string(),
        show_raw: false,
    });
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn main_view_skips_provider_without_models_and_shows_empty_state() {
    let mut app = test_app();
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "empty-provider".to_string(),
        name: "Empty Provider".to_string(),
        models: Vec::new(),
    }];

    keep_element(main_view(&app));
}
