//! Desktop mapping for workspace settings.
use crate::app::{App, Message, i18n::I18n, state::WorktreeSettingsState};
use crate::app::message::{SettingsMessage, settings::worktrees::WorktreeSettingsMessage as Event};
use iced::Element;
fn message(event: Event) -> Message { Message::Settings(SettingsMessage::Worktrees(event)) }
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    page(&app.worktree_settings, I18n::new(app.desktop_locale))
}
pub(crate) fn page(state: &WorktreeSettingsState, i18n: I18n) -> Element<'_, Message> {
    desktop_settings_workspace::system_settings_worktrees::page(state, i18n, desktop_settings_workspace::environment::can_add(&crate::app::gateway_client_endpoint())).map(message)
}
pub(crate) fn editor<'a>(state: &'a WorktreeSettingsState, i18n: I18n, base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_settings_workspace::system_settings_worktrees::editor_with(state, i18n, base, message)
}
pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let base = editor(&app.worktree_settings, i18n, base);
    let base = desktop_settings_workspace::system_settings_worktrees::environment::editor_with(&app.worktree_settings.environment, i18n, base, message);
    desktop_settings_workspace::system_settings_worktrees::overlay_with(&app.worktree_settings, base, message)
}
#[cfg(test)]
#[path = "system_settings_worktrees/managed_tests.rs"] mod managed_tests;
