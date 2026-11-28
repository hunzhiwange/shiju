use crate::app::{App, Message};

pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let storage = &app.skills_settings.storage;
    if !storage.open {
        return base;
    }
    let skill_count = storage.current.as_ref().and_then(|current| {
        app.skills_settings
            .workspace
            .workspace
            .inventory
            .as_ref()
            .filter(|inventory| {
                inventory.scope == current.scope && inventory.version == current.version
            })
            .map(|inventory| inventory.skills.iter().filter(|skill| skill.present).count())
    });
    let can_open_directory = storage.scope.as_ref().is_some_and(|scope| {
        scope.target_host_id == local_app_auth::HOST_ID
            && crate::app::config::gateway_client()
                .is_ok_and(|client| client.endpoint().is_loopback())
    });
    desktop_settings_skills::storage_modal::view(
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        storage,
        base,
        crate::app::message::settings::skills::storage_modal::message,
        skill_count,
        can_open_directory,
    )
}
