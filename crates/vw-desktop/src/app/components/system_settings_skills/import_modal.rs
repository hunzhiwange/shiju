use crate::app::{App, Message};

pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    desktop_settings_skills::import_modal::view_with_app_names(
        &app.skills_settings.import,
        base,
        crate::app::message::settings::skills::import_modal::message,
        desktop_settings_apps::system_settings_apps::list::brand_icon,
        |id| {
            app.apps_catalog
                .rows
                .iter()
                .find(|row| row.entry.id == id)
                .map_or_else(|| format!("{id:?}"), |row| row.entry.name.clone())
        },
    )
}
