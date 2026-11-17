//! Codex subagent intent; configuration and publication remain gateway-owned.
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
};
use iced::Task;
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
#[cfg(test)]
use vw_api_app_aggregation::*;
pub use desktop_apps_ui::aggregation::subagent::{Context, Event, Form};
pub(crate) fn message(e: Event) -> Message {
    list::message(ListMessage::CodexSubagent(e))
}
fn current(app: &App) -> Result<(GatewayClient, Context), String> {
    let (client, scope) = list::current(app, AppId::Codex)?;
    let project = vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref());
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
    {
        return Err("应用列表已变化，请刷新后重试".into());
    }
    let endpoint = client.endpoint().clone();
    Ok((
        GatewayClient::new(endpoint)?,
        Context { scope, project: project.into(), access: app.apps_catalog.generation },
    ))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::None) { return Task::none(); }
    let (client, context) = match current(app) {
        Ok(value) => value,
        Err(error) => {
            app.app_providers.codex_subagent = None;
            return if matches!(event, Event::Finished { .. }) { Task::none() }
                else { app.show_error_toast(error) };
        }
    };
    let state = &mut app.app_providers;
    let blocked = state.loading || state.form.is_some() || state.editor.is_some();
    let mut notice = None;
    let task = desktop_settings_apps::aggregation::subagent::update(
        &mut state.codex_subagent, &mut state.generation, blocked, client, context, event, &mut notice,
    ).map(message);
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(error)) => app.show_error_toast(error),
        Some(desktop_settings_apps::aggregation::Notice::Info(info)) => app.show_info_toast(info),
        _ => Task::none(),
    };
    Task::batch([task, feedback])
}
