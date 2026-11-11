//! Scoped Codex observations; acknowledgement never implies a process refresh.
use super::{Selection, current};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    AgentAppsSettingsError, GatewayAuth, GatewayClient,
    vw_api_types::{
        agent_apps::{AppId, aggregation::*},
        app_providers::operations::ProviderOperationProgress,
        task::TaskStatus,
    },
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Session {
    pub snapshot: Option<CodexClientsState>,
    pub pending: bool,
    pub confirming: bool,
    pub acknowledged: bool,
    pub operation: Option<ProviderOperationProgress>,
    pub error: Option<&'static str>,
}
impl Session {
    pub fn running(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|p| matches!(p.status, TaskStatus::Queued | TaskStatus::Running))
    }
    pub fn can_restart(&self) -> bool {
        !self.pending
            && self.error.is_none()
            && self.operation.is_none()
            && self.snapshot.as_ref().is_some_and(|s| s.daemon && !s.revision.is_empty())
    }
    pub fn begin(&mut self, action: Action) -> bool {
        if self.pending {
            return false;
        }
        let allowed = match action {
            Action::Read => !self.running(),
            Action::Ack => {
                !self.running()
                    && self.operation.is_none()
                    && !self.acknowledged
                    && self.error.is_none()
                    && self
                        .snapshot
                        .as_ref()
                        .is_some_and(|s| s.unverified && !s.revision.is_empty())
            }
            Action::Restart => self.confirming && self.can_restart(),
            Action::Poll => self.operation.is_some(),
            Action::Cancel => {
                self.running() && self.operation.as_ref().is_some_and(|p| !p.cancellation_requested)
            }
        };
        if allowed {
            self.pending = true;
            self.error = None;
            if matches!(action, Action::Read) {
                self.snapshot = None;
                self.acknowledged = false;
                self.operation = None;
                self.confirming = false;
            }
        }
        allowed
    }
    pub fn finish(&mut self, action: Action, reply: Result<Reply, AgentAppsSettingsError>) {
        self.pending = false;
        match reply {
            Ok(Reply::State(state)) => {
                self.acknowledged = matches!(action, Action::Ack)
                    && self.snapshot.as_ref().is_some_and(|old| old.revision == state.revision);
                self.snapshot = Some(state);
            }
            Ok(Reply::Operation(p)) => {
                if p.status == TaskStatus::Failed
                    || matches!(p.aggregation_outcome, Some(AggregationOutcome::Failed { .. }))
                {
                    self.error = Some("重启失败，请重新探测实际状态；不会自动重试。");
                } else if p.status == TaskStatus::Completed
                    && !matches!(p.aggregation_outcome, Some(AggregationOutcome::Restarted { .. }))
                {
                    self.error = Some("重启结果不完整，请重新探测实际状态。");
                }
                self.operation = Some(p);
            }
            Err(e) => {
                self.error = Some(feedback(&e));
                // The request may have reached the host. Require a fresh observation.
                if matches!(action, Action::Ack | Action::Restart) {
                    self.snapshot = None;
                }
            }
        }
    }
    pub fn result_label(&self) -> Option<&'static str> {
        let p = self.operation.as_ref()?;
        if self.error.is_some() {
            return None;
        }
        if p.status == TaskStatus::Completed {
            match p.aggregation_outcome.as_ref()? {
                AggregationOutcome::Restarted { result: CodexRestartResult::Restarted } => {
                    Some("托管守护进程已重启；桌面版与编辑器仍需手动彻底退出重开。")
                }
                AggregationOutcome::Restarted { result: CodexRestartResult::NotRunning } => {
                    Some("守护进程未运行，未执行重启或启动。")
                }
                _ => None,
            }
        } else if p.status == TaskStatus::Cancelled {
            Some("操作已取消；请重新探测客户端状态。")
        } else {
            Some("重启尚未完成；取消请求不代表进程已停止或恢复。")
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Read,
    Ack,
    Restart,
    Poll,
    Cancel,
}
#[derive(Clone)]
pub enum Reply {
    State(CodexClientsState),
    Operation(ProviderOperationProgress),
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    AskRestart,
    Back,
    Act(Action),
    Finished {
        selection: Selection,
        generation: u64,
        action: Action,
        result: Result<Reply, AgentAppsSettingsError>,
    },
}
pub(crate) fn message(e: Event) -> Message {
    super::message(super::Event::Clients(e))
}
fn feedback(e: &AgentAppsSettingsError) -> &'static str {
    match e {
        AgentAppsSettingsError::Http { status: 401 | 403, .. } => {
            "无权访问或重启此项目的 Codex；重启需要独立授权。"
        }
        AgentAppsSettingsError::Http { status: 409, .. } => {
            "客户端修订或操作冲突，请重新探测后确认。"
        }
        AgentAppsSettingsError::Http { status: 400 | 404 | 422 | 501, .. } => {
            "目标或托管安装身份不受支持，未确认安全时拒绝重启。"
        }
        AgentAppsSettingsError::Http { status: 502, .. } => {
            "目标执行失败，请重新探测，勿重复提交。"
        }
        AgentAppsSettingsError::Http { status: 503, .. } => {
            "执行主机不可用，请恢复连接后重新探测。"
        }
        _ => "结果未知或超时，请重新探测实际状态，勿重复提交。",
    }
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
    match action {
        Action::Read => client.aggregation_codex_clients(&query).await.map(Reply::State),
        Action::Ack => client
            .aggregation_codex_ack(&CodexClientsAckRequest {
                target_host_id: query.target_host_id,
                project_id: query.project_id,
                expected_revision: session.snapshot.as_ref().unwrap().revision.clone(),
            })
            .await
            .map(Reply::State),
        Action::Restart => client
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
                client.provider_aggregation_operation_get(id, &query).await
            } else {
                client.provider_aggregation_operation_cancel(id, &query).await
            };
            r.map(Reply::Operation).map_err(|e| match e {
                vw_gateway_client::AgentAppsReadError::Http { status, body } => {
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
