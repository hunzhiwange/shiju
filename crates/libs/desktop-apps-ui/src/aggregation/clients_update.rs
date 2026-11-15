// Scoped Codex observations; acknowledgement never implies a process refresh.
use super::Selection;
use iced::Task;
use gateway_endpoint::GatewayAuth;
use gateway_app_settings::AgentAppsSettingsError;
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::*;
use vw_api_task::TaskStatus;

use super::clients::{Session, Action, Reply, Event};
fn message(event: Event) -> Event { event }
pub async fn execute(
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
pub fn update(
    form: &mut Option<super::list::Form>,
    counter: &mut u64,
    _loading: bool,
    token: &str,
    current: Option<(GatewayClient, Selection)>,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<Event> {
    let Some((client, selection)) = current else {
        if let Some(f) = &mut (*form) {
            f.clients = None;
        }
        return if matches!(event, Event::Finished { .. }) {
            Task::none()
        } else {
            super::error(notice, "请先选择项目并连接已授权执行主机")
        };
    };
    let Some(form) = &mut (*form) else { return Task::none() };
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
            return super::error(notice, error);
        }
        if let Some(s) = &form.clients {
            if s.operation.as_ref().is_some_and(|p| p.status == TaskStatus::Completed)
                && let Some(label) = s.result_label()
            {
                return super::info(notice, label);
            }
            if matches!(action, Action::Ack) && s.acknowledged {
                return super::info(notice, "已确认此修订提醒；未执行重启。");
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
    *counter = counter.wrapping_add(1);
    let generation = *counter;
    form.generation = generation;
    let token = token.to_owned();
    Task::perform(execute(client, selection.clone(), token, intent, action), move |result| {
        message(Event::Finished { selection: selection.clone(), generation, action, result })
    })
}
impl super::Form {
    pub fn finish_clients(
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
#[path = "clients_tests.rs"]
mod clients_tests;
