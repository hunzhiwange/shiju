//! Single-shot queue writes; server snapshots alone confirm changes.
use desktop_provider_apps::Scope;
use iced::Task;
use gateway_app_failover::{GatewayAppFailoverClient, AgentAppsHighAvailabilityError as Error};
use {
    vw_api_app_failover::*,
    vw_api_transport::ProviderId,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub query: FailoverQueueQuery,
    pub list_generation: u64,
    pub catalog_generation: u64,
    pub access_generation: u64,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub open: bool,
    context: Option<Context>,
    pub serial: u64,
    pub busy: bool,
    pub queue: Option<FailoverQueueResponse>,
    pub candidates: Vec<FailoverQueueEntry>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Operation { Refresh, Add(ProviderId), Remove(ProviderId), Move(usize, usize), Enabled(bool) }
#[derive(Clone, Debug)]
pub enum Event {
    Open, Close, Run(Operation),
    Loaded(Context, u64, Result<(FailoverQueueResponse, FailoverCandidatesResponse), Error>),
}
impl State { pub fn can_show(&self, current: Option<&Context>) -> bool { self.open && current == self.context.as_ref() } }
fn matches_scope(context: &Context, scope: &FailoverQueueScope) -> bool {
    scope.project.project_id() == &context.query.project_id
        && scope.project.tenant_scope() == &context.scope.context.tenant
        && scope.target_host_id == context.query.target_host_id && scope.app_id == context.query.app_id
}
async fn request(client: GatewayAppFailoverClient, context: Context, token: String, op: Operation,
    queue: Option<FailoverQueueResponse>) -> Result<(FailoverQueueResponse, FailoverCandidatesResponse), Error> {
    let query = &context.query;
    let token = Some(token.as_str());
    let response = match op {
        Operation::Refresh => client.agent_apps_failover_queue(query, token).await?,
        op => {
            let queue = queue.ok_or(Error::Decode)?;
            let expected_version = queue.version;
            match op {
                Operation::Add(provider_id) => client.agent_apps_failover_add_member(query, &FailoverQueueMemberRequest { expected_version, provider_id }, token).await?,
                Operation::Remove(provider_id) => client.agent_apps_failover_remove_member(query, &FailoverQueueMemberRequest { expected_version, provider_id }, token).await?,
                Operation::Enabled(enabled) => client.agent_apps_failover_set_enabled(query, &FailoverQueueEnabledRequest { expected_version, enabled }, token).await?,
                Operation::Move(from, to) => {
                    let mut provider_ids: Vec<_> = queue.entries.into_iter().map(|e| e.provider_id).collect();
                    if from >= provider_ids.len() || to >= provider_ids.len() { return Err(Error::Decode); }
                    provider_ids.swap(from, to);
                    client.agent_apps_failover_reorder(query, &FailoverQueueReorderRequest { expected_version, provider_ids }, token).await?
                }
                Operation::Refresh => unreachable!(),
            }
        }
    };
    let candidates = client.agent_apps_failover_candidates(query, token).await?;
    if !matches_scope(&context, &response.scope) || !matches_scope(&context, &candidates.scope) { return Err(Error::Decode); }
    Ok((response, candidates))
}
fn error_message(error: &Error) -> String {
    match error {
        Error::Http { status, body } => {
            let reason = body.as_ref().map(|b| b.error.message.as_str()).unwrap_or("服务端未提供原因");
            match status {
                401 | 403 => format!("未获备用队列操作授权：{reason}"),
                409 => format!("队列版本冲突，正在刷新；请确认后重试：{reason}"),
                _ => format!("备用队列操作失败（HTTP {status}）：{reason}"),
            }
        }
        _ => "备用队列请求失败，未确认变更；请刷新后重试".into(),
    }
}
pub fn update(state: &mut State, current: Option<Context>, client: Result<vw_gateway_client::GatewayClient, String>, token: &str, event: Event, notice: &mut Option<crate::Notice>) -> Task<Event> {
    match event {
        Event::Close => {
            state.open = false; state.serial = state.serial.wrapping_add(1); state.busy = false;
        }
        Event::Open => {
            let Some(context) = current.clone() else { return crate::error(notice, "网关不可用，请刷新后重试"); };
            let serial = state.serial.wrapping_add(1);
            *state = State { open: true, context: Some(context), serial, ..Default::default() };
            return update(state, current, client, token, Event::Run(Operation::Refresh), notice);
        }
        Event::Run(op) => {
            let Some(context) = current.clone() else { return Task::none(); };
            if !state.open || state.context.as_ref() != Some(&context) || state.busy { return Task::none(); }
            if !matches!(op, Operation::Refresh) && state.queue.is_none() { return Task::none(); }
            let client = match client.clone() { Ok(c) => c, Err(e) => return crate::error(notice, e) };
            state.serial = state.serial.wrapping_add(1);
            let serial = state.serial;
            state.busy = true; state.error = None;
            let queue = state.queue.clone();
            let token = token.to_string();
            return Task::perform(async move {
                let client = GatewayAppFailoverClient::new(client.transport().clone());
                let result = request(client, context.clone(), token, op, queue).await;
                Event::Loaded(context, serial, result)
            }, |event| event);
        }
        Event::Loaded(context, serial, result) => {
            if current.clone().as_ref() != Some(&context) { return Task::none(); }
            if !state.open || state.context.as_ref() != Some(&context) || state.serial != serial || !state.busy { return Task::none(); }
            state.busy = false;
            match result {
                Ok((queue, candidates)) => { state.queue = Some(queue); state.candidates = candidates.candidates; state.error = None; }
                Err(error) => {
                    let text = error_message(&error);
                    state.error = Some(text.clone());
                    state.queue = None; state.candidates.clear();
                    let toast = crate::error(notice, text);
                    if matches!(error, Error::Http { status: 409, .. }) {
                        return Task::batch([toast, update(state, current, client, token, Event::Run(Operation::Refresh), notice)]);
                    }
                    return toast;
                }
            }
        }
    }
    Task::none()
}
