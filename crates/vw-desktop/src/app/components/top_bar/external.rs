use crate::app::{App, Message};
use desktop_navigation::components::top_bar::external as domain;
pub(super) fn state(app: &App) -> domain::State<'_> {
 domain::State { screen: app.navigation_tabs.screen, desktop_locale: app.desktop_locale, open_external_platform: app.open_external_platform, open_external_app: app.external_navigation.open_external_app, open_external_exists: &app.external_navigation.open_external_exists, active_menu: super::mapping::local_menu(app.navigation_panels.active_menu), has_project: app.navigation_tabs.project_path.is_some() }
}
pub(super) fn open_external_module(app: &App) -> iced::Element<'_, Message> { domain::open_external_module(&state(app)).map(super::mapping::map_message) }
