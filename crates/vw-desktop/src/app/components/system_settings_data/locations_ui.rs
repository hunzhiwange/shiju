use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_data::locations_ui as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::data_management::backup_list_ui::message(match event {
        domain::Event::Cleanup(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Cleanup(v0),
        domain::Event::Reveal(v0) => crate::app::message::settings::data_management::backup_list_ui::Event::Reveal(v0),
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog.data_backups;
    let state = domain::ViewState { busy: state.busy, pending: state.pending.is_some(), principal: &state.principal, cleanup_result: &state.cleanup_result, locations: &state.locations };
    let local = crate::app::config::gateway_client().ok().is_some_and(|client| crate::app::message::settings::data_management::locations_ui::can_reveal(client.endpoint().is_loopback(), &app.apps_catalog.host));
    domain::view(state, local, &app.apps_catalog.host).map(message)
}
