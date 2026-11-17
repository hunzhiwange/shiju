//! Desktop scope projection for the failover queue.
use crate::app::{App, Message};
use super::app_providers::list::{self, ListMessage};
use iced::Task;
use vw_api_app_failover::FailoverQueueQuery;
pub use desktop_apps_ui::failover_queue::{Context, State, Operation, Event};
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Failover(event)) }
fn current(app: &App) -> Option<Context> {
    let scope = app.app_providers.scope.as_ref()?;
    if list::current(app, scope.app).ok()?.1 != *scope
        || app.apps_catalog.context.as_ref() != Some(&scope.context)
        || scope.host.is_empty() { return None; }
    Some(Context {
        scope: scope.clone(),
        query: FailoverQueueQuery {
            project_id: vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).into(),
            target_host_id: scope.host.clone(), app_id: scope.app,
        },
        list_generation: app.app_providers.generation,
        catalog_generation: app.apps_catalog.generation,
        access_generation: app.apps_catalog.model_diagnostics.access_generation,
    })
}
pub(crate) fn can_show(app: &App) -> bool {
    app.app_providers.failover.can_show(current(app).as_ref())
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = current(app);
    let client = crate::app::config::gateway_client();
    let mut notice = None;
    let task = desktop_apps_ui::failover_queue::update(&mut app.app_providers.failover,
        current, client, &app.apps_catalog.token, event, &mut notice).map(message);
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}
