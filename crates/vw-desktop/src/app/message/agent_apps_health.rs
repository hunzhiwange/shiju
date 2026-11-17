//! Desktop authorization projection for health polling.
use crate::app::{App, Message};
use super::app_providers::list::{self, ListMessage};
use iced::Task;
use vw_api_app_failover::FailoverQueueQuery;
pub use desktop_apps_ui::agent_apps_health::{Context, Event};
pub fn message(event: Event) -> Message { list::message(ListMessage::FailoverHealth(event)) }
fn current(app: &App, c: &Context) -> bool {
    app.app_providers.scope.as_ref() == Some(&c.scope)
        && app.app_providers.generation == c.generation
        && app.apps_catalog.context.as_ref() == Some(&c.scope.context)
        && app.apps_catalog.connection_open

        && app.apps_catalog.model_diagnostics.access_generation == c.access
        && vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()) == c.query.project_id.as_ref()
        && list::current(app, c.scope.app).is_ok_and(|(_, s)| s == c.scope)
}
pub fn visible(app: &App) -> bool {
    app.apps_catalog.failover_health.context.as_ref().is_some_and(|c| current(app, c))
}
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let context = if let Event::Open(provider) = &event {
        let Some(scope) = app.app_providers.scope.clone() else { return Task::none(); };
        if !app.app_providers.snapshot.as_ref().is_some_and(|s| s.catalog.rows.iter().any(|r| &r.provider.id == provider)) { return Task::none(); }
        let project = vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref());
        let context = Context {
            query: FailoverQueueQuery { project_id: project.into(), target_host_id: scope.host.clone(), app_id: scope.app },
            scope, generation: app.app_providers.generation,
            access: app.apps_catalog.model_diagnostics.access_generation, provider: provider.clone().into(),
        };
        if !current(app, &context) { return app.show_error_toast("请重新连接应用"); }
        Some(context)
    } else {
        app.apps_catalog.failover_health.context.clone().filter(|c| current(app, c))
    };
    let polling = crate::app::components::third_party_settings::is_visible(app);
    let mut notice = None;
    let task = desktop_apps_ui::agent_apps_health::update(&mut app.apps_catalog.failover_health,
        context, polling, crate::app::config::gateway_client(), &app.apps_catalog.token, event, &mut notice).map(message);
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}
pub(crate) fn selected(app: &App, provider: &str) -> bool {
    app.apps_catalog.failover_health.context.as_ref().is_some_and(|c| c.provider.as_ref() == provider)
}

#[cfg(test)]
pub use desktop_apps_ui::agent_apps_health::State;
