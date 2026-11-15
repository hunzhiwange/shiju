//! Scoped Codex observations; acknowledgement never implies a process refresh.
use super::{Selection, current};
use crate::app::{App, Message};
use iced::Task;
use gateway_endpoint::GatewayAuth;
use gateway_app_settings::AgentAppsSettingsError;
use vw_gateway_client::GatewayClient;
use vw_api_types::agent_apps::AppId;
use vw_api_app_aggregation::*;
use vw_api_types::task::TaskStatus;

pub use desktop_apps_ui::aggregation::clients::{Session, Action, Reply, Event};
pub(crate) fn message(e: Event) -> Message {
    super::message(super::Event::Clients(e))
}
pub(crate) async fn execute(
    client: GatewayClient,
    selection: Selection,
    token: String,
    session: Session,
    action: Action,
) -> Result<Reply, AgentAppsSettingsError> {
    if selection.scope.app != AppId::Codex || !cfg!(target_os = "macos") {
        return Err(AgentAppsSettingsError::Transport("unsupported target".into()));
    }
    let mut check = session.clone();
    if !check.begin(action) {
        return Err(AgentAppsSettingsError::Transport("invalid intent".into()));
    }
    let client =
        GatewayClient::new(client.endpoint().clone().with_auth(GatewayAuth { skey: Some(token) }))
            .map_err(AgentAppsSettingsError::Transport)?;
    let query =
        AggregationQuery { target_host_id: selection.scope.host, project_id: selection.project };
    let codex = gateway_aggregation_codex::GatewayAggregationCodexClient::new(client.transport().clone());
    match action {
        Action::Read => codex.aggregation_codex_clients(&query).await.map(Reply::State),
        Action::Ack => codex
            .aggregation_codex_ack(&CodexClientsAckRequest {
                target_host_id: query.target_host_id,
                project_id: query.project_id,
                expected_revision: session.snapshot.as_ref().unwrap().revision.clone(),
            })
            .await
            .map(Reply::State),
        Action::Restart => codex
            .aggregation_codex_restart(&CodexRestartRequest {
                target_host_id: query.target_host_id,
                project_id: query.project_id,
                expected_revision: session.snapshot.as_ref().unwrap().revision.clone(),
                request_id: format!("codex-restart-{:032x}", rand::random::<u128>()),
                confirmed: true,
            })
            .await
            .map(Reply::Operation),
        Action::Poll | Action::Cancel => {
            let id = &session.operation.as_ref().unwrap().operation_id.0;
            let r = if matches!(action, Action::Poll) {
                gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).provider_aggregation_operation_get(id, &query).await
            } else {
                gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).provider_aggregation_operation_cancel(id, &query).await
            };
            r.map(Reply::Operation).map_err(|e| match e {
                gateway_http::AgentAppsReadError::Http { status, body } => {
                    AgentAppsSettingsError::Http { status, body }
                }
                _ => AgentAppsSettingsError::Transport("operation unavailable".into()),
            })
        }
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some((client, selection)) = current(app) else {
        if let Some(f) = &mut app.app_providers.aggregation {
            f.clients = None;
        }
        return if matches!(event, Event::Finished { .. }) {
            Task::none()
        } else {
            app.show_error_toast("请先选择项目并连接已授权执行主机")
        };
    };
    let Some(form) = &mut app.app_providers.aggregation else { return Task::none() };
    if selection.scope.app != AppId::Codex
        || form.selection != selection
        || !super::supported(selection.scope.app)
    {
        form.clients = None;
        return Task::none();
    }
    if let Event::Finished { selection: old, generation, action, result } = event {
        if old != selection || !form.finish_clients(&old, generation, action, result) {
            return Task::none();
        }
        if let Some(error) = form.clients.as_ref().and_then(|s| s.error) {
            return app.show_error_toast(error);
        }
        if let Some(s) = &form.clients {
            if s.operation.as_ref().is_some_and(|p| p.status == TaskStatus::Completed)
                && let Some(label) = s.result_label()
            {
                return app.show_info_toast(label);
            }
            if matches!(action, Action::Ack) && s.acknowledged {
                return app.show_info_toast("已确认此修订提醒；未执行重启。");
            }
        }
        return Task::none();
    }
    if matches!(event, Event::Open) {
        if form.loading || form.publish.is_some() || form.clients.is_some() {
            return Task::none();
        }
        form.clients = Some(Session::default());
    }
    let Some(session) = &mut form.clients else { return Task::none() };
    if session.pending {
        return Task::none();
    }
    let action = match event {
        Event::Open => Action::Read,
        Event::Close if !session.running() => {
            form.clients = None;
            return Task::none();
        }
        Event::AskRestart if session.can_restart() => {
            session.confirming = true;
            return Task::none();
        }
        Event::Back if !session.running() => {
            session.confirming = false;
            return Task::none();
        }
        Event::Act(a) => a,
        _ => return Task::none(),
    };
    let intent = session.clone();
    if !session.begin(action) {
        return Task::none();
    }
    app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
    let generation = app.app_providers.generation;
    form.generation = generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(execute(client, selection.clone(), token, intent, action), move |result| {
        message(Event::Finished { selection: selection.clone(), generation, action, result })
    })
}
impl super::Form {
    pub(crate) fn finish_clients(
        &mut self,
        selection: &Selection,
        generation: u64,
        action: Action,
        result: Result<Reply, AgentAppsSettingsError>,
    ) -> bool {
        if &self.selection != selection || self.generation != generation {
            return false;
        }
        let Some(session) = &mut self.clients else { return false };
        if !session.pending {
            return false;
        }
        session.finish(action, result);
        true
    }
}
#[cfg(test)]
#[path = "aggregation_clients_tests.rs"]
mod tests;

impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexClientsReply([redacted])")
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexClientsEvent([redacted])")
    }
}
