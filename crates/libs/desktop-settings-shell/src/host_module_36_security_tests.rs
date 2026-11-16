//! Security tab geometry and pixels are compared with the real general page.
use config_ui::DesktopLocale;
use desktop_settings_appearance::general;
use desktop_settings_security::{
    SecurityView,
    security_message::Message as SecurityMessage,
    security_state::{SecuritySettingsState, SecuritySettingsTab},
    system_settings_security::{self, SECURITY_TABS, security_tab_label},
};
use iced::{Element, Theme};
#[path = "host_module_30_tests/tab_appearance.rs"]
mod appearance;
#[allow(dead_code)]
#[derive(Clone, Debug)]
enum Message {
    None,
    Security(SecurityMessage),
    General(general::Action<&'static str>),
}
impl appearance::AppearanceMessage for Message {
    type Settings = Self;
    fn none() -> Self {
        Self::None
    }
    fn settings(self) -> Self {
        self
    }
}
struct Fixture {
    state: SecuritySettingsState,
    locale: DesktopLocale,
    theme: Theme,
}
impl appearance::AppearanceContext<Message> for Fixture {
    fn locale(&self) -> DesktopLocale {
        self.locale
    }
    fn general_view(&self) -> Element<'_, Message> {
        general::view(
            &general::View {
                desktop_locale: self.locale,
                app_theme: &self.theme,
                shell: "fixture",
                shell_options: vec!["fixture"],
                font_family: "fixture",
                displayed_font_size: 14.0,
                preview_auto_save_mode: Default::default(),
                chat_timezone: "UTC",
                launch_at_login: false,
                launch_at_login_busy: false,
                close_to_tray: false,
                task_completion_notification: false,
                notification_supported: false,
                general_settings_tab: crate::GeneralSettingsTab::Interface,
            },
            Message::General,
            |title, body, _, _| {
                iced::widget::column![iced::widget::text(title), iced::widget::text(body)].into()
            },
            iced::widget::Space::new().into(),
        )
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_security() {
    if !super::cron_render_test_support::isolated(concat!(
        module_path!(),
        "::settings_tabs_core_appearance_security"
    )) {
        return;
    }
    let mut renderer = super::cron_render_test_support::renderer().await;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut app =
                Fixture { state: SecuritySettingsState::default(), locale, theme: theme.clone() };
            app.state.active_tab = SecuritySettingsTab::Bash;
            let input = SecurityView {
                security_settings: &app.state,
                desktop_locale: locale,
                app_theme: &app.theme,
                last_copied_code_hash: None,
                help_text: |title, body, _, _| {
                    iced::widget::column![iced::widget::text(title), iced::widget::text(body)]
                        .into()
                },
            };
            let mut scene = appearance::TabAppearance {
                renderer: &mut renderer,
                theme,
                width: if locale == DesktopLocale::English { 320 } else { 640 },
                message: std::marker::PhantomData,
            };
            let labels: Vec<_> =
                SECURITY_TABS.iter().map(|tab| security_tab_label(locale, tab.tab)).collect();
            assert!(matches!(
                scene.check(
                    &app,
                    system_settings_security::view(&input).map(Message::Security),
                    &labels,
                    5
                ),
                Message::Security(SecurityMessage::SecurityTabSelected(SecuritySettingsTab::Guard))
            ));
        }
    }
}
