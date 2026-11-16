use crate::app::App;
#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use iced::Task;
pub(super) fn project_choices(app: &App) -> Vec<(String, String)> {
    desktop_settings_channels::update::projects::project_choices(&app.workspace_state.recent_projects, &app.workspace_state.recent_projects_edits, app.workspace_state.recent_projects_meta.iter().map(|meta| (meta.path.as_str(), meta.name.as_str())))
}

#[cfg(test)]
pub(super) fn select(app: &mut App, instance: String, project: Option<String>) -> Task<Message> {
 let services = super::updates::services(app);
 desktop_settings_channels::update::projects::select(&mut app.channels_settings, &services, instance, project).map(super::updates::map_effect)
}
