//! 常规设置的宿主投影、跨页组合和消息映射。
use crate::app::{App, Message, Shell, TerminalTheme, message};
#[cfg(test)] use crate::app::PreviewAutoSaveMode;
#[cfg(test)]
use crate::app::components::system_settings::general_fields::{self, GeneralSettingField};
use iced::Element;
#[cfg(test)] use iced::Theme;
#[cfg(all(test, not(target_arch = "wasm32")))] use iced::Length;
#[cfg(test)] const GENERAL_CONTROL_WIDTH: f32 = 280.0;
use desktop_settings_appearance::general as domain;
pub(super) use domain::GENERAL_TABS;
fn inputs(app: &App) -> domain::View<'_, Shell> {
 domain::View { desktop_locale: app.desktop_locale, app_theme: &app.app_theme,
 shell: app.terminal.shell, shell_options: Shell::all().to_vec(), font_family: &app.terminal.font_family,
 displayed_font_size: app.terminal_config_persistence.displayed_font_size(app.terminal.font_size),
 preview_auto_save_mode: app.preview_auto_save_mode, chat_timezone: &app.chat_timezone,
 launch_at_login: app.launch_at_login, launch_at_login_busy: app.launch_at_login_busy,
 close_to_tray: app.close_to_tray, task_completion_notification: app.task_completion_notification,
 notification_supported: crate::app::desktop_notification::SUPPORTED, general_settings_tab: app.general_settings_tab }
}
pub fn view(app: &App) -> Element<'_, Message> {
 use crate::app::state::GeneralSettingsTab;
 let subpage = match app.general_settings_tab {
 GeneralSettingsTab::Editor => super::system_settings_editor::content(app),
 GeneralSettingsTab::Companion => super::system_settings_companion::content(app),
 GeneralSettingsTab::Themes => super::system_settings_themes::view(app),
 _ => iced::widget::Space::new().into(),
 };
 domain::view(&inputs(app), map_message, render_help, subpage)
}
fn map_message(action: domain::Action<Shell>) -> Message {
 use domain::Action as A;
 match action {
 A::AppThemeSelected(v) => Message::View(message::ViewMessage::AppThemeSelected(v)),
 A::ShellSelected(v) => Message::Terminal(message::TerminalMessage::ShellSelected(v)),
 A::TerminalSystemTheme => Message::Terminal(message::TerminalMessage::ThemeSelected(TerminalTheme::System)),
 A::FontSelected(v) => Message::Terminal(message::TerminalMessage::FontSelected(v)),
 A::FontSizePreview(v) => Message::Terminal(message::TerminalMessage::FontSizePreview(v)),
 A::FontSizePreviewCommit => Message::Terminal(message::TerminalMessage::FontSizePreviewCommit),
 A::AutoSaveModeChanged(v) => Message::Preview(message::PreviewMessage::AutoSaveModeChanged(v)),
 A::LocaleSelected(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::LocaleSelected(v))),
 A::TimezoneSelected(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::TimezoneSelected(v))),
 A::LaunchAtLoginToggled(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::LaunchAtLoginToggled(v))),
 A::CloseToTrayToggled(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::CloseToTrayToggled(v))),
 A::TaskCompletionNotificationToggled(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::TaskCompletionNotificationToggled(v))),
 A::TestTaskCompletionNotification => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::TestTaskCompletionNotification)),
 A::TabSelected(v) => Message::Settings(message::SettingsMessage::General(message::GeneralSettingsMessage::TabSelected(v))),

 }
}
fn render_help<'a>(title: &'a str, body: std::borrow::Cow<'a, str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> iced::Element<'a, Message> {
 super::chat_panel::message_view::selectable_titled_text(title, body, color, i18n)
}
#[cfg(test)]
fn general_field_with_help<'a>(app: &App, field: &GeneralSettingField, description: impl Into<std::borrow::Cow<'a, str>>, control: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
 super::system_settings_common::settings_flexible_row_with_help(desktop_i18n::I18n::new(app.desktop_locale), &app.app_theme, desktop_i18n::I18n::new(app.desktop_locale).__(field.label), description, 280.0, control).id(field.id)
}
#[cfg(test)]
mod test_adapters {
use super::*;
pub(super) fn app_theme_selected(theme: iced::Theme) -> Message {
    map_message(domain::Action::AppThemeSelected(theme))
}

/// 返回常规设置可选择的全部应用主题：Iced 内置主题与本地预设/自定义主题。
pub(super) fn app_theme_options(app: &App) -> Vec<iced::Theme> { domain::app_theme_options(&app.app_theme) }

pub(super) fn terminal_shell_selected(shell: Shell) -> Message {
    map_message(domain::Action::ShellSelected(shell))
}

pub(super) fn terminal_theme_selected(theme: TerminalTheme) -> Message {
    match theme { TerminalTheme::System => map_message(domain::Action::TerminalSystemTheme) }
}

pub(super) fn terminal_font_selected(font_family: String) -> Message {
    map_message(domain::Action::FontSelected(font_family))
}

pub(super) fn preview_auto_save_mode_changed(mode: PreviewAutoSaveMode) -> Message {
    map_message(domain::Action::AutoSaveModeChanged(mode))
}

pub(super) fn timezone_selected(timezone: String) -> Message {
    map_message(domain::Action::TimezoneSelected(timezone))
}

pub(super) fn terminal_font_size_changed(font_size: f32) -> Message {
    map_message(domain::Action::FontSizePreview(font_size))
}

}
#[cfg(test)]
use test_adapters::*;
#[cfg(test)]
#[path = "system_settings_general_tests.rs"]
mod system_settings_general_tests;
