use crate::input_panel::model_views::{self, Action, ModelPopoverInput};
use desktop_settings_providers::state::{ModelSettingsState, ModelSummary, ProviderModelsSummary};
use iced::Length;
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
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
fn model_toggle_button_uses_auto_label_and_expanded_state() {
    let app = ModelSettingsState::default();

    let auto = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        true,
        "openai/gpt-4.1",
        false,
        std::convert::identity::<Action>,
    );
    let expanded = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "openai/gpt-4.1",
        true,
        std::convert::identity::<Action>,
    );

    assert_eq!(auto.as_widget().size().width, Length::Shrink);
    assert_eq!(auto.as_widget().children().len(), 1);
    assert_eq!(expanded.as_widget().size().width, Length::Shrink);
    assert_eq!(expanded.as_widget().children().len(), 1);
}

#[test]
fn model_toggle_button_resolves_provider_model_and_bare_model_names() {
    let mut app = ModelSettingsState::default();
    app.providers = vec![provider(
        "openai",
        "OpenAI",
        vec![model("gpt-4.1", "GPT 4.1", true), model("z-disabled", "Disabled", false)],
    )];

    let provider_model = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "openai/gpt-4.1",
        false,
        std::convert::identity::<Action>,
    );
    let bare_model = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "gpt-4.1",
        false,
        std::convert::identity::<Action>,
    );
    let fallback = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "custom/model",
        false,
        std::convert::identity::<Action>,
    );
    let bare_fallback = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "unknown",
        false,
        std::convert::identity::<Action>,
    );

    assert_eq!(provider_model.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_model.as_widget().size().width, Length::Shrink);
    assert_eq!(fallback.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_fallback.as_widget().size().width, Length::Shrink);
}

#[test]
fn model_popover_content_renders_loading_empty_and_filtered_states() {
    let mut app = ModelSettingsState::default();

    app.loading = true;
    let loading = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "auto".into(),
        },
        true,
        "auto",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(loading.as_widget().size().width, Length::Fixed(320.0));
    drop(loading);

    app.loading = false;
    let empty = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "missing".into(),
        },
        false,
        "missing",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(empty.as_widget().size().width, Length::Fixed(320.0));
    drop(empty);

    app.providers = vec![
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
    app.query = String::new();
    let unfiltered = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "gpt-4.1".into(),
        },
        true,
        "gpt-4.1",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(unfiltered.as_widget().size().width, Length::Fixed(320.0));
    drop(unfiltered);

    app.query = "gpt".to_string();
    let filtered = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "openai/gpt-4.1".into(),
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(filtered.as_widget().size().width, Length::Fixed(320.0));
    drop(filtered);

    app.query = "vision".to_string();
    let capability_flags = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "openai/vision".into(),
        },
        false,
        "openai/vision",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(capability_flags.as_widget().size().width, Length::Fixed(320.0));
    drop(capability_flags);

    let bare_selected = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "gpt-4.1".into(),
        },
        false,
        "gpt-4.1",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(bare_selected.as_widget().size().width, Length::Fixed(320.0));
    drop(bare_selected);

    app.query = "no-match".to_string();
    let no_match = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "openai/gpt-4.1".into(),
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );
    assert_eq!(no_match.as_widget().size().width, Length::Fixed(320.0));
}

#[test]
fn model_popover_content_keeps_available_models_visible_while_refreshing() {
    let mut app = ModelSettingsState::default();
    app.loading = true;
    app.providers = vec![provider("openai", "OpenAI", vec![model("gpt-4.1", "GPT 4.1", true)])];

    let content = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
            input_model: "openai/gpt-4.1".into(),
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        std::convert::identity::<Action>,
    );

    assert_eq!(content.as_widget().size().width, Length::Fixed(320.0));
}
