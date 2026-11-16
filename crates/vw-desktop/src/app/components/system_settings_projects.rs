//! Desktop project navigation and shared presentation adapters.
use crate::app::{App, Message, i18n::I18n};
use crate::app::message::{SettingsMessage, ProjectMessage};
use crate::app::components::system_settings_common::settings_flexible_row_with_help;
use crate::app::views::home::{project_avatar, parse_hex_color};
use desktop_settings_workspace::system_settings_projects::{self as domain, Event, ViewState};
use iced::{Element, Theme};
fn message(event: Event) -> Message {
    match event {
        Event::ProjectEnableWorktreeToggled(path, value) => Message::Settings(SettingsMessage::ProjectEnableWorktreeToggled(path, value)),
        Event::RecentProjectDeleteRequested(idx) => Message::Settings(SettingsMessage::RecentProjectDeleteRequested(idx)),
        Event::RecentProjectDeleteCanceled => Message::Settings(SettingsMessage::RecentProjectDeleteCanceled),
        Event::RecentProjectDeleteConfirmed(idx) => Message::Settings(SettingsMessage::RecentProjectDeleteConfirmed(idx)),
        Event::RecentProjectQueryChanged(query) => Message::Settings(SettingsMessage::RecentProjectQueryChanged(query)),
        Event::OpenRecentPressed(path) => Message::Project(ProjectMessage::OpenRecentPressed(path)),
        Event::ProjectToolsMenuToggled(path) => Message::Project(ProjectMessage::ProjectToolsMenuToggled(path)),
        Event::ProjectEditOpened(path) => Message::Project(ProjectMessage::ProjectEditOpened(path)),
        Event::ProjectToolsMenuClosed => Message::Project(ProjectMessage::ProjectToolsMenuClosed),
        Event::OpenFolderPressed => Message::Project(ProjectMessage::OpenFolderPressed),
    }
}
fn avatar<'a>(name: &str, path: &str, icon: Option<&str>, color: Option<&str>, size: f32) -> Element<'a, Message> {
    project_avatar(name, path, icon, color.and_then(parse_hex_color), size)
}
fn help<'a>(i18n: I18n, theme: &Theme, title: &'a str, description: &'a str, width: f32, control: Element<'a, Message>) -> iced::widget::Container<'a, Message> {
    settings_flexible_row_with_help(i18n, theme, title, description, width, control)
}
fn state(app: &App) -> ViewState<'_> {
    ViewState {
        project_path: &app.navigation_tabs.project_path, project_worktree_enabled: &app.workspace_state.project_worktree_enabled,
        project_tools_menu_path: &app.workspace_state.project_tools_menu_path, recent_projects: &app.workspace_state.recent_projects,
        recent_projects_edits: &app.workspace_state.recent_projects_edits, recent_project_delete_confirm_idx: app.workspace_state.recent_project_delete_confirm_idx,
        recent_project_query: &app.workspace_state.recent_project_query, desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    let appearance = |path: &str| {
        let meta = app.workspace_state.recent_projects_meta.iter().find(|meta| meta.path == path);
        (meta.and_then(|meta| meta.icon.as_deref()), meta.and_then(|meta| meta.icon_color.as_deref()))
    };
    domain::view(&state(app), message, avatar, help, &appearance)
}
#[cfg(test)]
#[path = "system_settings_projects_ui_refresh_30_tests.rs"]
mod system_settings_projects_ui_refresh_30_tests;
