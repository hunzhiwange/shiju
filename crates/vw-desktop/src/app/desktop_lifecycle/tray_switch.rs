//! Tray writes use the same scoped SDK as provider forms.
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::state::app_providers::list::Scope;
use crate::app::{App, Message};
use iced::Task;
pub(crate) fn start(app: &mut App, id: String) -> Task<Message> {
    let state = &app.app_providers;
    if !desktop_provider_apps::tray_switch::can_start(state.scope.as_ref(), state.snapshot.as_ref(), &id,
        app.shutdown_requested, state.loading, state.tray_switch.is_some()) {
        return Task::none();
    }
    let Some(scope) = state.scope.clone() else { return Task::none() };
    let (client, current) = match crate::app::message::app_providers::list::current(app, scope.app)
    {
        Ok(value) => value,
        Err(error) => return app.show_error_toast(error),
    };
    if current != scope {
        return Task::none();
    }
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    app.app_providers.tray_switch = Some(id.clone());
    let worker_scope = scope.clone();
    let worker_id = id.clone();
    Task::perform(
        async move {
            desktop_provider_apps::tray_switch::switch(client, worker_scope, worker_id, token).await
        },
        move |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TrayProviderSwitched {
                    scope: scope.clone(),
                    generation,
                    id: id.clone(),
                    result,
                },
            ))
        },
    )
}

pub(crate) fn finish(
    app: &mut App,
    scope: Scope,
    generation: u64,
    id: String,
    result: Result<(), String>,
) -> Task<Message> {
    if app.app_providers.scope.as_ref() != Some(&scope)
        || app.app_providers.generation != generation
        || app.app_providers.tray_switch.as_ref() != Some(&id)
    {
        return Task::none();
    }
    app.app_providers.tray_switch = None;
    if crate::app::message::app_providers::list::current(app, scope.app)
        .map_or(true, |(_, current)| current != scope)
    {
        return Task::none();
    }
    match desktop_provider_apps::tray_switch::apply_result(
        app.app_providers.snapshot.as_mut(), scope.app, id, result,
    ) {
        Ok(message) => app.show_success_toast(message),
        Err(error) => app.show_error_toast(error),
    }
}
