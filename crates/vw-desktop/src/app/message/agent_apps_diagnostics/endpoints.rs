pub use desktop_apps_ui::agent_apps_diagnostics::endpoints::{Event, Reply};
use crate::app::{App, Message, message::app_providers::list::{self, ListMessage}};
use iced::Task;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::EndpointDiagnostics(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_apps_ui::agent_apps_diagnostics::endpoints::{self, Host};
    let current_scope = app.app_providers.scope.as_ref().and_then(|scope| list::current(app, scope.app).ok().map(|(_, scope)| scope));
    let client = matches!(event, Event::Open).then(crate::app::config::gateway_client);
    let mut notice = None;
    let mut manage_custom = None;
    let mut applied = false;
    let task = endpoints::update(&mut Host {
        state: &mut app.app_providers.endpoint_speed,
        form: &mut app.app_providers.form,
        generation: &mut app.app_providers.generation,
        scope: app.app_providers.scope.as_ref(),
        catalog_context: app.apps_catalog.context.as_ref(),
        credentials_generation: app.apps_catalog.model_diagnostics.access_generation,
        project: app.project_id.as_deref(),
        token: &app.apps_catalog.token,
        current_scope, endpoints_open: app.app_providers.endpoints.is_some(),
        client, notice: &mut notice, manage_custom: &mut manage_custom, applied: &mut applied,
    }, event).map(message);
    let custom = if let Some(provider) = manage_custom {
        app.app_providers.endpoints = Some(crate::app::state::app_providers::endpoints::Form::new(provider));
        crate::app::message::app_providers::endpoints::update(app, crate::app::message::app_providers::endpoints::Event::Refresh)
    } else { Task::none() };
    let toast = if applied { app.show_info_toast("端点已回填草稿，请点击供应商表单的保存") } else { match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    }};
    Task::batch([task, custom, toast])
}
