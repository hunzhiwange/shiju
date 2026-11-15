//! External history catalog; credentials stay in the application connection.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::{Element, Task};
#[path = "external_sessions/list.rs"]
mod list;
#[path = "external_sessions/reader.rs"]
mod reader;
#[path = "external_sessions/details.rs"]
mod details;
#[path = "external_sessions/images.rs"]
mod images;
#[path = "external_sessions/find.rs"]
mod find;
#[path = "external_sessions/delete.rs"]
mod delete;
#[path = "external_sessions/resume.rs"]
mod resume;
#[path = "external_sessions/paths.rs"]
mod paths;
#[path = "external_sessions/export.rs"]
mod export;
#[path = "external_sessions/maintenance.rs"]
mod maintenance;
#[path = "external_sessions/compression.rs"]
mod compression;

pub use desktop_apps_ui::agent_apps::external_sessions::{State, Scope, Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ExternalSessions(event)))
}
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    let base = if let Some(reader) = &state.reader { reader::view(state, reader) } else { list::view(state) };
    compression::view(state, maintenance::view(state, paths::view(state, resume::view(state, delete::view(state, base)))))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Compression(event) = event { return compression::update(app, event); }
    if app.apps_catalog.external_sessions.compression.is_open() { return Task::none(); }
    if let Event::Maintenance(event) = event { return maintenance::update(app, event); }
    // Keep the catalog generation and conflicting actions fixed while maintenance is open.
    if app.apps_catalog.external_sessions.maintenance.is_open() { return Task::none(); }
    if let Event::Export(event) = event { return export::update(app, event); }
    if let Event::Path(event) = event { return paths::update(app, event); }
    if let Event::Resume(event) = event { return resume::update(app, event); }
    if let Event::Delete(event) = event { return delete::update(app, event); }
    if let Event::Details(event) = event { return details::update(app, event); }
    if let Event::Image(event) = event { return images::update(app, event); }
    if reader::handles(&event) { return reader::update(app, event); }
    match event {
        Event::Open => {
            app.apps_catalog.external_sessions.open = true;
            let navigation = crate::app::message::view::update(app,
                crate::app::message::view::ViewMessage::OpenSystemSettingsTab(
                    crate::app::components::system_settings::SystemTab::Apps));
            return navigation;
        }
        Event::Close => {
            desktop_apps_ui::agent_apps::external_sessions::close(&mut app.apps_catalog.external_sessions);
            return Task::none();
        }
        Event::GroupByTime(value) => { app.apps_catalog.external_sessions.by_time = value; return Task::none(); }
        _ => {}
    }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用，请重新连接"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    // A loaded response must still be checked when the current project is absent.
    let project = app.project_id.clone().unwrap_or_default();
    let connection = desktop_apps_ui::Connection { client, context, catalog_context: app.apps_catalog.context.clone(), project,
        host: app.apps_catalog.host.clone(), generation: app.apps_catalog.generation, token: app.apps_catalog.token.clone() };
    let mut notice = None;
    let task = desktop_apps_ui::agent_apps::external_sessions::update(&mut app.apps_catalog.external_sessions, connection, event, &mut notice).map(message);
    let feedback = match notice { Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text), Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text), None => Task::none() };
    Task::batch([task, feedback])
}
