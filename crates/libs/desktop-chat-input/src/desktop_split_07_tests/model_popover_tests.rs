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
        None,
        std::convert::identity::<Action>,
    );
    let expanded = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "openai/gpt-4.1",
        true,
        None,
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
        None,
        std::convert::identity::<Action>,
    );
    let bare_model = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "gpt-4.1",
        false,
        None,
        std::convert::identity::<Action>,
    );
    let fallback = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "custom/model",
        false,
        None,
        std::convert::identity::<Action>,
    );
    let bare_fallback = model_views::model_toggle_button(
        i18n(),
        &app.providers,
        false,
        false,
        "unknown",
        false,
        None,
        std::convert::identity::<Action>,
    );

    assert_eq!(provider_model.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_model.as_widget().size().width, Length::Shrink);
    assert_eq!(fallback.as_widget().size().width, Length::Shrink);
    assert_eq!(bare_fallback.as_widget().size().width, Length::Shrink);
}

#[tokio::test]
async fn model_toggle_keeps_one_line_and_reserves_the_chevron_at_narrow_widths() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, widget::Tree};
    use iced::{Font, Pixels, Size};

    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("renderer");
    let mut toggle = model_views::model_toggle_button(
        i18n(),
        &[],
        false,
        false,
        "xiaomi/Xiaomi MiMo-V2.6-Pro Extended\nPreview",
        false,
        Some("极高"),
        std::convert::identity::<Action>,
    );
    let mut tree = Tree::new(toggle.as_widget());
    let mut layout_at = |width| {
        toggle.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, 96.0)),
        )
    };

    let wide = layout_at(300.0);
    let narrow = layout_at(112.0);
    assert!(narrow.size().width < wide.size().width);
    assert_eq!(narrow.size().height, wide.size().height);

    let height_wrapper = Layout::new(&narrow).children().next().expect("button content");
    let row = height_wrapper.children().next().expect("model row");
    let label = row.children().nth(1).expect("label and chevron");
    let mut layers = label.children();
    let name = layers.next().unwrap().children().next().unwrap().bounds();
    let chevron = layers.next().unwrap().children().next().unwrap().bounds();
    assert_eq!(chevron.width, 11.0);
    assert!(name.x + name.width < chevron.x);
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
        },
        true,
        "auto",
        iced::widget::Space::new().into(),
        None,
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
        },
        false,
        "missing",
        iced::widget::Space::new().into(),
        None,
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
        },
        true,
        "gpt-4.1",
        iced::widget::Space::new().into(),
        None,
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
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        None,
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
        },
        false,
        "openai/vision",
        iced::widget::Space::new().into(),
        None,
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
        },
        false,
        "gpt-4.1",
        iced::widget::Space::new().into(),
        None,
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
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        None,
        std::convert::identity::<Action>,
    );
    assert_eq!(no_match.as_widget().size().width, Length::Fixed(320.0));
}

#[test]
fn model_popover_content_keeps_available_models_visible_while_refreshing() {
    let app = ModelSettingsState {
        loading: true,
        providers: vec![provider("openai", "OpenAI", vec![model("gpt-4.1", "GPT 4.1", true)])],
        ..Default::default()
    };

    let content = model_views::model_popover_content(
        ModelPopoverInput {
            settings: &app,
            i18n: i18n(),
            cli_local: false,
            started: false,
            can_select: true,
            is_acp: false,
        },
        false,
        "openai/gpt-4.1",
        iced::widget::Space::new().into(),
        None,
        std::convert::identity::<Action>,
    );

    assert_eq!(content.as_widget().size().width, Length::Fixed(320.0));
}
