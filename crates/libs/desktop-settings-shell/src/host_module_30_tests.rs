//! Localized shared help preserves the overlay in both copy states.
use desktop_ui_controls::system_settings_common::help::localized_settings_help_modal;
#[derive(Clone)]
enum Message {
    None,
    GatewayHealthTick,
}
use iced::widget::text;

#[test]
fn help_modal_fills_overlay_for_default_and_copied_states() {
    let mut last_copied_code_hash = None;
    let help_text = String::from("帮助内容");

    let default = localized_settings_help_modal(
        Default::default(),
        last_copied_code_hash,
        text("base").into(),
        "帮助",
        help_text.clone(),
        |_| Message::None,
        Message::GatewayHealthTick,
    );
    assert_eq!(default.as_widget().children().len(), 2);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&help_text, &mut hasher);
    last_copied_code_hash = Some(std::hash::Hasher::finish(&hasher));

    let copied = localized_settings_help_modal(
        Default::default(),
        last_copied_code_hash,
        text("base").into(),
        "",
        help_text,
        |_| Message::None,
        Message::None,
    );
    assert_eq!(copied.as_widget().children().len(), 2);
}

mod help_ui_refresh_03_tests;
mod support;

mod tab_appearance;

mod shell_scene;

#[test]
fn settings_envelope_routes_existing_domains_and_preserves_payload() {
    use crate::messages::SettingsRoute;
    use support::SettingsMessage;
    assert!(matches!(
        SettingsMessage::SystemHelpClose.route(),
        SettingsRoute::TabSelected(SettingsMessage::SystemHelpClose)
    ));
    assert!(
        matches!(SettingsMessage::ProxyHttpChanged("http://fixture".into()).route(), SettingsRoute::ProxyEnabledToggled(SettingsMessage::ProxyHttpChanged(value)) if value == "http://fixture")
    );
    assert!(matches!(
        SettingsMessage::SecurityHelpOpen.route(),
        SettingsRoute::SecurityBashSecurityEnabledToggled(SettingsMessage::SecurityHelpOpen)
    ));
    assert!(matches!(
        SettingsMessage::CronHelpClose.route(),
        SettingsRoute::CronFullAccessConfirm(SettingsMessage::CronHelpClose)
    ));
}

impl shell_scene::SceneMessage for support::Message {
    fn copied_text(&self) -> Option<&str> {
        if let Self::CopyCode(text) = self { Some(text) } else { None }
    }
}

#[tokio::test]
async fn mobile_shell_keeps_real_layout_and_draw_path() {
    use support::Message;
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let page = crate::mobile_shell(
            iced::widget::text("设置 / Settings").into(),
            None,
            Message::None,
            Some(Message::None),
        );
        let mut scene = shell_scene::Scene::new(page, 320, 640).await;
        assert_eq!(scene.node.size().width, 320.0);
        assert_eq!(scene.node.size().height, 640.0);
        assert!(!scene.snapshot(&theme, iced::mouse::Cursor::Unavailable, "").is_empty());
    }
}

mod messages_tests;

mod types_tests;

impl tab_appearance::AppearanceMessage for support::Message {
    type Settings = support::SettingsMessage;
    fn none() -> Self {
        Self::None
    }
    fn settings(self) -> <Self as tab_appearance::AppearanceMessage>::Settings {
        match self {
            Self::Settings(message) => message,
            other => panic!("expected settings message, got {other:?}"),
        }
    }
}
struct TabReference(config_ui::DesktopLocale);
impl tab_appearance::AppearanceContext<support::Message> for TabReference {
    fn locale(&self) -> config_ui::DesktopLocale {
        self.0
    }
    fn general_view(&self) -> iced::Element<'_, support::Message> {
        use desktop_ui_controls::system_settings_common::{settings_tab_button, settings_tab_row};
        let i18n = desktop_i18n::I18n::new(self.0);
        iced::widget::container(settings_tab_row(
            ["界面", "终端", "编辑器", "主题"].into_iter().enumerate().map(|(index, label)| {
                settings_tab_button(
                    i18n.__(label),
                    index == 0,
                    Some(support::Message::Settings(support::SettingsMessage::SystemHelpClose)),
                )
                .into()
            }),
        ))
        .padding([0, 14])
        .into()
    }
}
#[tokio::test]
async fn tab_appearance_preserves_pixels_geometry_and_events() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    use iced::advanced::renderer::Headless;
    use tab_appearance::AppearanceContext;
    for locale in [
        config_ui::DesktopLocale::SimplifiedChinese,
        config_ui::DesktopLocale::TraditionalChinese,
        config_ui::DesktopLocale::English,
    ] {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            let mut renderer =
                iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia"))
                    .await
                    .unwrap();
            let reference = TabReference(locale);
            let i18n = desktop_i18n::I18n::new(locale);
            let mut scene = tab_appearance::TabAppearance {
                renderer: &mut renderer,
                theme,
                width: if locale == config_ui::DesktopLocale::English { 320 } else { 640 },
                message: std::marker::PhantomData,
            };
            let event = scene.check(
                &reference,
                reference.general_view(),
                &["界面", "终端", "编辑器", "主题"].map(|label| i18n.__(label)),
                1,
            );
            assert!(matches!(event, support::SettingsMessage::SystemHelpClose));
        }
    }
}
