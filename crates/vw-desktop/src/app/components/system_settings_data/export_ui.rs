use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_data::export_ui as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::data_management::export_ui::message(match event {
        domain::Event::Open => crate::app::message::settings::data_management::export_ui::Event::Open,
        domain::Event::Close => crate::app::message::settings::data_management::export_ui::Event::Close,
        domain::Event::Principal(v0) => crate::app::message::settings::data_management::export_ui::Event::Principal(v0),
        domain::Event::Select(v0, v1) => crate::app::message::settings::data_management::export_ui::Event::Select(v0, v1),
        domain::Event::Export => crate::app::message::settings::data_management::export_ui::Event::Export,
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    domain::view(crate::app::message::settings::data_management::export_ui::identity(app).is_some()).map(message)
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !crate::app::message::settings::data_management::export_ui::visible(app) { return base; }
    let state = &app.apps_catalog.data_export;
    let state = domain::ViewState { busy: state.busy, principal: &state.principal, categories: &state.categories };
    domain::overlay(state, true, base).map(|event| match event { domain::OverlayEvent::Local(event) => message(event), domain::OverlayEvent::External(message) => message })
}
