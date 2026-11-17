//! Desktop identity and effect adapter.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
pub(crate) use desktop_settings_memory::data_management::export_ui::{Event, State};
use desktop_settings_memory::data_management::export_ui as domain;
pub(crate) use desktop_settings_memory::data_management::export_ui::Identity;
use crate::app::state::AgentAppsContext;
pub(crate) fn identity(app: &App) -> Option<Identity> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    Some(Identity { context, project: vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation })
}
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DataExport(event)) }
pub(crate) fn visible(app: &App) -> bool { domain::visible(&app.apps_catalog.data_export, identity(app).as_ref()) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, value) = event { return if success { app.show_success_toast(value) } else { app.show_error_toast(value) }; }
    let current = identity(app);
    domain::update(&mut app.apps_catalog.data_export, current, crate::app::config::gateway_client().ok(), app.apps_catalog.token.clone(), event).then(|output| match output {
        domain::Output::Event(event) => Task::done(message(event)),
        domain::Output::Error(value) => Task::done(Message::Settings(SettingsMessage::DataExport(Event::Feedback(false, value)))),
        domain::Output::Success(value) => Task::done(Message::Settings(SettingsMessage::DataExport(Event::Feedback(true, value)))),
    })
}
