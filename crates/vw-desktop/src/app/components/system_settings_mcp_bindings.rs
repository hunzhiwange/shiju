//! Desktop visibility checks and installation observation projection.
use crate::app::{App, Message};
use crate::app::message::settings::{mcp_management_bindings as bindings, mcp_management_list as catalog};
use desktop_settings_mcp::bindings as ui;
use iced::Element;
pub(crate) fn panel(app: &App) -> Element<'_, Message> {
    let installations = if app.apps_catalog.context.as_ref() == catalog::context(app).as_ref().map(|c| &c.endpoint)
        && !app.apps_catalog.loading {
        app.apps_catalog.rows.iter().filter_map(|entry| entry.observation.as_ref().ok().map(|observation|
            observation.installations.iter().filter(move |installation| installation.app_id == entry.entry.id)))
            .flatten().collect()
    } else { Vec::new() };
    ui::panel(catalog::visible(app), bindings::visible(app).map(|s| ui::BindingsView {
        selected: &s.selected, running: s.running, result: &s.result,
    }), installations, &app.apps_catalog.host, catalog::loading(app)).map(|event| bindings::message(match event {
        ui::BindingsMessage::Select(v, b) => bindings::BindingsMessage::Select(v, b),
        ui::BindingsMessage::Apply(v, b) => bindings::BindingsMessage::Apply(v, b),
        ui::BindingsMessage::RetryFailed => bindings::BindingsMessage::RetryFailed,
    }))
}
