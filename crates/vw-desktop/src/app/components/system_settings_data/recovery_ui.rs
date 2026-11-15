use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_data::recovery_ui as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::data_management::recovery_ui::message(match event {
        domain::Event::Open => crate::app::message::settings::data_management::recovery_ui::Event::Open,
        domain::Event::Close => crate::app::message::settings::data_management::recovery_ui::Event::Close,
        domain::Event::Principal(v0) => crate::app::message::settings::data_management::recovery_ui::Event::Principal(v0),
        domain::Event::Diagnose => crate::app::message::settings::data_management::recovery_ui::Event::Diagnose,
        domain::Event::Preview(v0) => crate::app::message::settings::data_management::recovery_ui::Event::Preview(v0),
        domain::Event::Refresh => crate::app::message::settings::data_management::recovery_ui::Event::Refresh,
        domain::Event::Confirm => crate::app::message::settings::data_management::recovery_ui::Event::Confirm,
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    domain::view(crate::app::message::settings::data_management::export_ui::identity(app).is_some()).map(message)
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !crate::app::message::settings::data_management::recovery_ui::visible(app) { return base; }
    let state = &app.apps_catalog.recovery;
    let state = domain::ViewState { busy: state.busy, submitted: state.submitted, principal: &state.principal, diagnosis: &state.diagnosis, preview: &state.preview, progress: &state.progress, error: &state.error };
    domain::overlay(state, true, base).map(|event| match event { domain::OverlayEvent::Local(event) => message(event), domain::OverlayEvent::External(message) => message })
}
