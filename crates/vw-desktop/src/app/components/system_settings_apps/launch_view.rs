use crate::app::{App, Message, components::system_settings_common, i18n::I18n};
use desktop_chat_input::input_panel::quick_messages::{WorkspacePicker, workspace_picker};
use desktop_settings_apps::system_settings_apps::launch_view::overlay as launch_overlay;
use iced::{Element, Theme};

use super::{LaunchMessage, event};

pub(crate) fn overlay<'a>(
    app: &'a App,
    i18n: I18n,
    theme: &Theme,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let catalog = &app.apps_catalog;
    launch_overlay(
        i18n,
        &catalog.launch,
        &catalog.host,
        |selected| {
            let open = catalog.launch.project_picker_open;
            let can_browse = open
                && super::can_browse_project(
                    &catalog.host,
                    &crate::app::config::gateway_client_endpoint(),
                );
            workspace_picker(
                WorkspacePicker {
                    i18n,
                    projects: &app.workspace_state.recent_projects,
                    project_labels: &app.workspace_state.recent_projects_edits,
                    selected,
                    current: app.navigation_tabs.project_path.as_deref(),
                    query: &catalog.launch.project_query,
                    open,
                    within_window: true,
                },
                event(LaunchMessage::ProjectPickerOpen(!open)),
                event(LaunchMessage::ProjectPickerOpen(false)),
                |query| event(LaunchMessage::ProjectSearch(query)),
                |path| event(LaunchMessage::ProjectSelected(path)),
                None,
                can_browse.then(|| event(LaunchMessage::BrowseProject)),
            )
        },
        base,
        |title, description, size| {
            if size == 28 {
                system_settings_common::settings_page_heading_with_help(
                    i18n,
                    theme,
                    i18n.__(title),
                    i18n.__(description),
                )
            } else {
                system_settings_common::settings_group_heading_with_help(
                    i18n,
                    theme,
                    i18n.__(title),
                    i18n.__(description),
                )
            }
        },
        event,
    )
}
