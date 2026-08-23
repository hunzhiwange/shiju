//! 验证代理设置共享控件、标签与状态样式的构建契约。
//! 帮助文本、禁用状态和明暗主题用例共同约束共享组件的一致性。

use super::shared::{
    agent_sidebar_button_style, detail_tab_button, editor_style, entry_kind_label, is_dark_theme,
    label_col, models_for_provider, section_card, selected_entry, with_selected_option,
};
use crate::app::state::{AgentSettingsEntryKind, DelegateAgentSettingsEntry};
use iced::widget::{button, text_editor};
use iced::{Background, Color, Element, Length, Theme};

fn entry(key: &str, kind: AgentSettingsEntryKind) -> DelegateAgentSettingsEntry {
    DelegateAgentSettingsEntry {
        key: key.to_string(),
        label: key.to_string(),
        description: String::new(),
        kind,
        role_controls_locked: false,
        can_disable: true,
        enabled: true,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        provider: String::new(),
        model: String::new(),
        force_model_selection: false,
        system_prompt_editor: text_editor::Content::new(),
        api_key_input: String::new(),
        temperature: 0.7,
        compact_context: false,
        max_history_messages: 100,
        parallel_tools: false,
        max_depth: 1,
        agentic: false,
        allowed_tools: Vec::new(),
        skill_policy: vw_config_types::agent::DelegateCapabilityPolicy::default(),
        allowed_skills: Vec::new(),
        enabled_identity_files: Vec::new(),
        max_iterations: 50_000,
    }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid background, got {other:?}"),
    }
}

fn solid_background(background: Background) -> Color {
    match background {
        Background::Color(color) => color,
        other => panic!("expected solid background, got {other:?}"),
    }
}

#[test]
fn with_selected_option_keeps_existing_selection_once() {
    let options = with_selected_option(vec!["alpha".to_string(), "beta".to_string()], "beta");

    assert_eq!(options, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn with_selected_option_appends_missing_selection() {
    let options = with_selected_option(vec!["alpha".to_string()], "beta");

    assert_eq!(options, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn with_selected_option_trims_empty_selection_sorts_and_dedups() {
    let options = with_selected_option(
        vec!["beta".to_string(), "alpha".to_string(), "alpha".to_string()],
        "   ",
    );

    assert_eq!(options, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn with_selected_option_trims_missing_nonempty_selection() {
    let options = with_selected_option(vec!["alpha".to_string()], "  beta  ");

    assert_eq!(options, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn entry_kind_label_maps_all_entry_kinds() {
    let mut entry = entry("main", AgentSettingsEntryKind::Main);

    assert_eq!(entry_kind_label(&entry), "内建 Agent");
    entry.kind = AgentSettingsEntryKind::BuiltinWorker;
    assert_eq!(entry_kind_label(&entry), "内建 Worker");
    entry.kind = AgentSettingsEntryKind::Custom;
    assert_eq!(entry_kind_label(&entry), "自定义");
}

#[test]
fn models_for_provider_returns_models_or_empty() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.provider_models = vec![crate::app::state::ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![
            crate::app::state::ModelSummary {
                id: "gpt-a".to_string(),
                name: "A".to_string(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 128_000,
                detail: serde_json::json!({}),
            },
            crate::app::state::ModelSummary {
                id: "gpt-b".to_string(),
                name: "B".to_string(),
                enabled: true,
                toolcall: false,
                attachment: true,
                context_limit: 64_000,
                detail: serde_json::json!({}),
            },
        ],
    }];

    assert_eq!(models_for_provider(&app, "openai"), vec!["gpt-a".to_string(), "gpt-b".to_string()]);
    assert!(models_for_provider(&app, "missing").is_empty());
}

#[test]
fn models_for_provider_uses_first_exact_provider_match() {
    let (mut app, _) = crate::app::App::new();
    let model = crate::app::state::ModelSummary {
        id: "second-model".to_string(),
        name: "Second".to_string(),
        enabled: true,
        toolcall: false,
        attachment: false,
        context_limit: 32_000,
        detail: serde_json::json!({}),
    };
    app.agents_settings.provider_models = vec![
        crate::app::state::ProviderModelsSummary {
            id: "duplicate".to_string(),
            name: "First".to_string(),
            models: Vec::new(),
        },
        crate::app::state::ProviderModelsSummary {
            id: "duplicate".to_string(),
            name: "Second".to_string(),
            models: vec![model],
        },
    ];

    assert!(models_for_provider(&app, "duplicate").is_empty());
}

#[test]
fn dark_theme_detection_and_sidebar_style_cover_branches() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));

    let light_selected = agent_sidebar_button_style(&Theme::Light, button::Status::Active, true);
    let light_hovered = agent_sidebar_button_style(&Theme::Light, button::Status::Hovered, false);
    let dark_pressed = agent_sidebar_button_style(&Theme::Dark, button::Status::Pressed, false);

    assert!(matches!(light_selected.background, Some(Background::Color(_))));
    assert!(matches!(light_hovered.background, Some(Background::Color(_))));
    assert!(matches!(dark_pressed.background, Some(Background::Color(_))));
    assert_eq!(light_selected.border.width, 1.0);
    assert_eq!(light_selected.border.radius.top_left, 14.0);
}

#[test]
fn sidebar_style_distinguishes_light_status_backgrounds() {
    let active = agent_sidebar_button_style(&Theme::Light, button::Status::Active, false);
    let hovered = agent_sidebar_button_style(&Theme::Light, button::Status::Hovered, false);
    let pressed = agent_sidebar_button_style(&Theme::Light, button::Status::Pressed, false);

    assert_ne!(background_color(active.background), background_color(hovered.background));
    assert_ne!(background_color(hovered.background), background_color(pressed.background));
    assert_ne!(active.border.color, hovered.border.color);
    assert_eq!(hovered.border.color, pressed.border.color);
}

#[test]
fn sidebar_style_distinguishes_dark_status_backgrounds() {
    let active = agent_sidebar_button_style(&Theme::Dark, button::Status::Disabled, false);
    let hovered = agent_sidebar_button_style(&Theme::Dark, button::Status::Hovered, false);
    let pressed = agent_sidebar_button_style(&Theme::Dark, button::Status::Pressed, false);

    assert_ne!(background_color(active.background), background_color(hovered.background));
    assert_ne!(background_color(hovered.background), background_color(pressed.background));
    assert_ne!(active.border.color, hovered.border.color);
    assert_eq!(hovered.border.color, pressed.border.color);
}

#[test]
fn sidebar_selected_style_uses_theme_specific_background_and_border() {
    let light = agent_sidebar_button_style(&Theme::Light, button::Status::Pressed, true);
    let dark = agent_sidebar_button_style(&Theme::Dark, button::Status::Active, true);

    assert_ne!(background_color(light.background), background_color(dark.background));
    assert_ne!(light.border.color, dark.border.color);
    assert_eq!(light.text_color, Theme::Light.palette().text);
    assert_eq!(dark.text_color, Theme::Dark.palette().text);
}

#[test]
fn selected_entry_returns_exact_selected_agent() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries = vec![
        entry("first", AgentSettingsEntryKind::Custom),
        entry("second", AgentSettingsEntryKind::BuiltinWorker),
    ];
    app.agents_settings.selected_agent = "second".to_string();

    assert_eq!(selected_entry(&app).map(|entry| entry.key.as_str()), Some("second"));
}

#[test]
fn selected_entry_falls_back_to_first_agent_when_selection_is_missing() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries = vec![
        entry("first", AgentSettingsEntryKind::Custom),
        entry("second", AgentSettingsEntryKind::BuiltinWorker),
    ];
    app.agents_settings.selected_agent = "missing".to_string();

    assert_eq!(selected_entry(&app).map(|entry| entry.key.as_str()), Some("first"));
}

#[test]
fn selected_entry_returns_none_when_agent_list_is_empty() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries.clear();

    assert!(selected_entry(&app).is_none());
}

#[test]
fn editor_style_preserves_shared_editor_shape_across_themes() {
    let light = editor_style(&Theme::Light, text_editor::Status::Active);
    let dark = editor_style(&Theme::Dark, text_editor::Status::Focused { is_hovered: true });

    assert_ne!(solid_background(light.background), solid_background(dark.background));
    assert_ne!(light.border.color, dark.border.color);
    assert_eq!(light.border.width, 1.0);
    assert_eq!(light.border.radius.top_left, 14.0);
}

#[test]
fn section_card_and_label_column_keep_expected_widget_shape() {
    let card: Element<'_, crate::app::Message> = section_card("Title", "Description").into();
    let labels: Element<'_, crate::app::Message> = label_col("Label", "Hint").into();

    assert_eq!(card.as_widget().size().width, Length::Fill);
    assert_eq!(card.as_widget().children().len(), 2);
    assert_eq!(labels.as_widget().children().len(), 2);
}

#[test]
fn detail_tab_button_is_a_shrink_button_with_one_label_child() {
    let active = detail_tab_button("基础", "basic", "basic");
    let inactive = detail_tab_button("工具", "tools", "basic");

    assert_eq!(active.as_widget().size().width, Length::Shrink);
    assert_eq!(active.as_widget().children().len(), 1);
    assert_eq!(inactive.as_widget().size().width, Length::Shrink);
    assert_eq!(inactive.as_widget().children().len(), 1);
}
