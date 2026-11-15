use crate::app::{App, Message};
use desktop_navigation::components::top_bar::external as domain;
pub(super) fn state(app: &App) -> domain::State<'_> {
 domain::State { screen: app.screen, desktop_locale: app.desktop_locale, open_external_platform: app.open_external_platform, open_external_app: app.open_external_app, open_external_exists: &app.open_external_exists, active_menu: super::mapping::local_menu(app.active_menu), has_project: app.project_path.is_some() }
}
pub(super) fn open_external_module(app: &App) -> iced::Element<'_, Message> { domain::open_external_module(&state(app)).map(super::mapping::map_message) }
#[cfg(test)] pub(super) fn supported_external_apps(app: &App) -> Vec<crate::app::state::ExternalOpenApp> { domain::supported_external_apps(&state(app)) }
#[cfg(test)] pub(super) use domain::{OPEN_EXTERNAL_TARGETS};
