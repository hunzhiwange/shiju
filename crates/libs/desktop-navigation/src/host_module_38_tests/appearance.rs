#![allow(dead_code)]
#[path = "../../../desktop-settings-shell/src/host_module_30_tests/tab_appearance.rs"]
mod shared;
use super::navigation_fixture::{Fixture, help};
use crate::components::message::Message;
use config_ui::DesktopLocale;
use iced::{Element, Theme};
impl shared::AppearanceMessage for Message {
    type Settings = Self;
    fn none() -> Self {
        Self::None
    }
    fn settings(self) -> Self {
        self
    }
}
impl shared::AppearanceContext<Message> for Fixture {
    fn locale(&self) -> DesktopLocale {
        self.desktop_locale
    }
    fn general_view(&self) -> Element<'_, Message> {
        general(self.desktop_locale, &self.app_theme)
    }
}
fn general(locale: DesktopLocale, theme: &Theme) -> Element<'_, Message> {
    desktop_settings_appearance::general::view(
        &desktop_settings_appearance::general::View {
            desktop_locale: locale,
            app_theme: theme,
            shell: "zsh",
            shell_options: vec!["zsh"],
            font_family: "",
            displayed_font_size: 14.0,
            preview_auto_save_mode: Default::default(),
            chat_timezone: "Asia/Shanghai",
            launch_at_login: false,
            launch_at_login_busy: false,
            close_to_tray: false,
            task_completion_notification: false,
            notification_supported: false,
            general_settings_tab:
                desktop_ui_controls::settings_navigation::GeneralSettingsTab::Interface,
        },
        |_| Message::None,
        |title, body, _, i18n| help(i18n, None, title, body),
        iced::widget::Space::new().into(),
    )
}
pub async fn run_at_least(
    _: &str,
    min_width: u32,
    fixture: impl Fn() -> Fixture,
    check: impl Fn(&mut shared::TabAppearance<'_, Message>, &mut Fixture),
) {
    use iced::advanced::renderer::Headless;
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = iced::Renderer::new(
        iced::Font::with_name("Noto Sans CJK SC"),
        14.0.into(),
        Some("tiny-skia"),
    )
    .await
    .unwrap();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut input = fixture();
            input.desktop_locale = locale;
            input.app_theme = theme.clone();
            let mut scene = shared::TabAppearance {
                renderer: &mut renderer,
                theme,
                width: if locale == DesktopLocale::English { min_width } else { 640 },
                message: std::marker::PhantomData,
            };
            check(&mut scene, &mut input);
        }
    }
}
