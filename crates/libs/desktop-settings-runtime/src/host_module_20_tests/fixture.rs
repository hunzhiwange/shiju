//! Local page inputs; no gateway, application lifecycle, or global state.
pub use crate::Message;
use crate::state::{HooksSettingsState, ReliabilitySettingsState, RuntimeSettingsState};
pub use iced::{Element, Length, widget::container};
#[derive(Default)]
pub struct Fixture {
    pub hooks_settings: HooksSettingsState,
    pub runtime_settings: RuntimeSettingsState,
    pub reliability_settings: ReliabilitySettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
        title, body, color, i18n,
    )
}
pub mod runtime {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        crate::system_settings_runtime::view(crate::system_settings_runtime::View {
            runtime_settings: &app.runtime_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            select_focus_scope: "settings-runtime-test",
            render_help: help,
        })
    }
}
pub mod hooks {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        crate::system_settings_hooks::view(crate::system_settings_hooks::View {
            hooks_settings: &app.hooks_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            render_help: help,
        })
    }
}
pub mod reliability {
    use super::*;
    fn inputs(app: &Fixture) -> crate::system_settings_reliability::View<'_> {
        crate::system_settings_reliability::View {
            reliability_settings: &app.reliability_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            last_copied_code_hash: None,
            render_help: help,
        }
    }
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        crate::system_settings_reliability::view(inputs(app))
    }
    pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
        crate::system_settings_reliability::view_overlays(inputs(app), base)
    }
}
