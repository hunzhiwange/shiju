use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_data::backup_list_ui as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::data_management::backup_list_ui::message(match event {
        domain::Event::Open => crate::app::message::settings::data_management::backup_list_ui::Event::Open,
        domain::Event::Close => crate::app::message::settings::data_management::backup_list_ui::Event::Close,
        domain::Event::Principal(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Principal(v0),
        domain::Event::Name(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Name(v0),
        domain::Event::Interval(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Interval(v0),
        domain::Event::Retain(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Retain(v0),
        domain::Event::ToggleLocations => crate::app::message::settings::data_management::backup_list_ui::Event::ToggleLocations,
        domain::Event::Refresh => crate::app::message::settings::data_management::backup_list_ui::Event::Refresh,
        domain::Event::Create => crate::app::message::settings::data_management::backup_list_ui::Event::Create,
        domain::Event::SavePolicy => crate::app::message::settings::data_management::backup_list_ui::Event::SavePolicy,
        domain::Event::Cancel => crate::app::message::settings::data_management::backup_list_ui::Event::Cancel,
        domain::Event::Confirm => crate::app::message::settings::data_management::backup_list_ui::Event::Confirm,
        domain::Event::Rename(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Rename(v0),
        domain::Event::Delete(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Delete(v0),
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let available = crate::app::message::settings::data_management::identity::identity(app).is_some();
    let visible = crate::app::message::settings::data_management::backup_list_ui::visible(app);
    let state = &app.apps_catalog.data_backups;
    let state = domain::ViewState { busy: state.busy, show_locations: state.show_locations, principal: &state.principal, name: &state.name, interval: &state.interval, retain: &state.retain, pending: state.pending.as_ref().map(|p| match p { desktop_settings_memory::data_management::backup_list_ui::Pending::Cleanup(id) => domain::Pending::Cleanup(id), desktop_settings_memory::data_management::backup_list_ui::Pending::Rename(row) => domain::Pending::Rename(&row.name), desktop_settings_memory::data_management::backup_list_ui::Pending::Delete(row) => domain::Pending::Delete(&row.name) }), rows: &state.rows };
    domain::view(desktop_i18n::I18n::new(app.general_settings.desktop_locale), state, visible, available, super::recovery_ui::view(app), super::locations_ui::view(app), |id|
        crate::app::message::settings::data_management::recovery_ui::message(
            desktop_settings_memory::data_management::recovery_ui::Event::OpenBackup(id)
        )
    ).map(|event| match event { domain::OverlayEvent::Local(event) => message(event), domain::OverlayEvent::External(message) => message })
}
