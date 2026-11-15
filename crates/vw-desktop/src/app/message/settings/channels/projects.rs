use crate::app::App;
#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use iced::Task;
pub(super) fn project_choices(app: &App) -> Vec<(String, String)> {
    app.recent_projects
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let name = app
                .recent_projects_edits
                .get(index)
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .or_else(|| {
                    app.recent_projects_meta
                        .iter()
                        .find(|meta| meta.path == *path && !meta.name.trim().is_empty())
                        .map(|meta| meta.name.clone())
                })
                .unwrap_or_else(|| {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                        .to_owned()
                });
            (path.clone(), name)
        })
        .collect()
}

#[cfg(test)]
pub(super) fn retain_configured_bindings(app: &mut App) {
 desktop_settings_channels::update::projects::retain_configured_bindings(&mut app.channels_settings);
}
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
#[cfg(test)]
pub(super) fn select(app: &mut App, instance: String, project: Option<String>) -> Task<Message> {
 let services = super::updates::services(app);
 desktop_settings_channels::update::projects::select(&mut app.channels_settings, &services, instance, project).map(super::updates::map_effect)
}
