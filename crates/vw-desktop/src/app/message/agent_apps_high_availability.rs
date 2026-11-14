//! Single-shot queue writes; server snapshots alone confirm changes.
use crate::app::{App, Message, state::app_providers::list::Scope};
use super::app_providers::list::{self, ListMessage};
use iced::Task;
use vw_gateway_client::{GatewayClient, AgentAppsHighAvailabilityError as Error};
use vw_gateway_client::vw_api_types::{agent_apps_high_availability::*, ProviderId};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    scope: Scope,
    query: FailoverQueueQuery,
    list_generation: u64,
    catalog_generation: u64,
    access_generation: u64,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
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
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Failover(event)) }
fn current(app: &App) -> Option<Context> {
    let scope = app.app_providers.scope.as_ref()?;
    if list::current(app, scope.app).ok()?.1 != *scope
        || app.apps_catalog.context.as_ref() != Some(&scope.context)
        || app.apps_catalog.token.is_empty() || scope.host.is_empty() { return None; }
    Some(Context {
        scope: scope.clone(),
        query: FailoverQueueQuery {
            project_id: app.project_id.as_deref().filter(|s| !s.is_empty())?.into(),
            target_host_id: scope.host.clone(), app_id: scope.app,
        },
        list_generation: app.app_providers.generation,
        catalog_generation: app.apps_catalog.generation,
        access_generation: app.apps_catalog.model_diagnostics.access_generation,
    })
}
pub(crate) fn can_show(app: &App) -> bool {
    app.app_providers.failover.open && current(app).as_ref() == app.app_providers.failover.context.as_ref()
}
fn matches_scope(context: &Context, scope: &FailoverQueueScope) -> bool {
    scope.project.project_id() == &context.query.project_id
        && scope.project.tenant_scope() == &context.scope.context.tenant
        && scope.target_host_id == context.query.target_host_id && scope.app_id == context.query.app_id
}
async fn request(client: GatewayClient, context: Context, token: String, op: Operation,
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
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Close => {
            let state = &mut app.app_providers.failover;
            state.open = false; state.serial = state.serial.wrapping_add(1); state.busy = false;
        }
        Event::Open => {
            let Some(context) = current(app) else { return app.show_error_toast("请先选择项目并连接应用执行主机"); };
            let state = &mut app.app_providers.failover;
            let serial = state.serial.wrapping_add(1);
            *state = State { open: true, context: Some(context), serial, ..Default::default() };
            return update(app, Event::Run(Operation::Refresh));
        }
        Event::Run(op) => {
            let Some(context) = current(app) else { return Task::none(); };
            let state = &mut app.app_providers.failover;
            if !state.open || state.context.as_ref() != Some(&context) || state.busy { return Task::none(); }
            if !matches!(op, Operation::Refresh) && state.queue.is_none() { return Task::none(); }
            let client = match crate::app::config::gateway_client() { Ok(c) => c, Err(e) => return app.show_error_toast(e) };
            state.serial = state.serial.wrapping_add(1);
            let serial = state.serial;
            state.busy = true; state.error = None;
            let queue = state.queue.clone();
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let result = request(client, context.clone(), token, op, queue).await;
                Event::Loaded(context, serial, result)
            }, message);
        }
        Event::Loaded(context, serial, result) => {
            if current(app).as_ref() != Some(&context) { return Task::none(); }
            let state = &mut app.app_providers.failover;
            if !state.open || state.context.as_ref() != Some(&context) || state.serial != serial || !state.busy { return Task::none(); }
            state.busy = false;
            match result {
                Ok((queue, candidates)) => { state.queue = Some(queue); state.candidates = candidates.candidates; state.error = None; }
                Err(error) => {
                    let text = error_message(&error);
                    state.error = Some(text.clone());
                    state.queue = None; state.candidates.clear();
                    let toast = app.show_error_toast(text);
                    if matches!(error, Error::Http { status: 409, .. }) {
                        return Task::batch([toast, update(app, Event::Run(Operation::Refresh))]);
                    }
                    return toast;
                }
            }
        }
    }
    Task::none()
}
