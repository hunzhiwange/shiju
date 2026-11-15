use crate::app::App;
#[cfg(test)] use crate::app::Message;
use crate::app::state::DelegateAgentSettingsEntry;
#[cfg(test)] use iced::Element;
#[cfg(test)]
pub(super) use desktop_settings_agents::skills::*;
#[cfg(test)]
pub(super) fn view<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> {
let projects = super::agent_project_options(app);
desktop_settings_agents::skills::view(input(app, projects), entry).map(super::domain::event)
}
pub(super) fn input<'a>(app: &'a App, projects: Vec<String>) -> desktop_settings_agents::skills::Input<'a> { let state = &app.agents_settings; desktop_settings_agents::skills::Input { locale: app.desktop_locale, theme: &app.app_theme, help: super::domain::help, project_options: projects, skill_directory: &state.skill_directory, skill_directory_scope: state.skill_directory_scope, skill_catalog: &state.skill_catalog, skill_catalog_loading: state.skill_catalog_loading, skill_catalog_error: &state.skill_catalog_error } }
#[cfg(test)]
pub(super) fn discovery_order_text(app: &App) -> String { desktop_settings_agents::skills::discovery_order_text(&input(app, vec![])) }
