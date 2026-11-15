use super::model_state::*;
use crate::{Notice, error as notify_error, AgentAppsContext, ProviderScope};
use iced::Task;
use vw_gateway_client::GatewayClient;
use vw_api_app_diagnostics::*;
use gateway_app_diagnostics::GatewayAppDiagnosticsClient;
pub struct Access<'a> {
    pub token: &'a str,
    pub context: &'a Option<AgentAppsContext>,
    pub project: &'a Option<String>,
    pub scope: Option<&'a ProviderScope>,
    pub actual: Option<ProviderScope>,
    pub catalog: Option<&'a vw_api_app_provider_edit::catalog::ProviderCatalog>,
    pub client: Result<GatewayClient, String>,
    pub show_apps: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(String),
    Close,
    Model(String),
    Start(DiagnosticProbeKind),
    Cancel(usize),
    Refresh,
    Started(usize, Result<DiagnosticJobIdentity, String>),
    Snapshot(usize, u64, Result<DiagnosticJobSnapshot, String>),
}
pub fn message(event: Event) -> Event { event }
fn error(_: impl std::fmt::Debug) -> String {
    "诊断请求未验证，请检查执行主机、授权和已保存的供应商/模型后重试".into()
}
fn current(state: &ModelState, access: &Access<'_>, context: &ModelContext) -> bool {
    state.access_generation == context.credentials_generation
        && !access.token.is_empty()
        && access.context.as_ref() == Some(&context.scope.context)
        && access.project.as_deref() == Some(context.project.as_ref())
        && access.actual.as_ref() == Some(&context.scope)
        && access.scope == Some(&context.scope)
}
fn visible(state: &ModelState, access: &Access<'_>, context: &ModelContext) -> bool {
    current(state, access, context)
        && access.show_apps

        && state.open.as_ref() == Some(context)
}
pub fn can_show(state: &ModelState, access: &Access<'_>) -> bool {
    state.open.as_ref().is_some_and(|c| visible(state, access, c))
}
pub async fn submit(
    client: &GatewayClient,
    run: &ModelRun,
    token: &str,
) -> Result<DiagnosticJobIdentity, String> {
    let provider_id = run.context.provider.clone();
    let model_id = run.model.clone().into();
    let request = match run.kind {
        DiagnosticProbeKind::Reachability => DiagnosticStartRequest::Reachability { provider_id },
        DiagnosticProbeKind::Model => {
            DiagnosticStartRequest::Model { provider_id, model_id, protocol: None }
        }
        DiagnosticProbeKind::Streaming => {
            DiagnosticStartRequest::Streaming { provider_id, model_id }
        }
        _ => return Err(error(())),
    };
    GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_start(&run.context.query(), &request, token).await.map_err(error)
}
fn poll(state: &mut ModelState, access: &Access<'_>, notice: &mut Option<Notice>, index: usize, cancel: bool) -> Task<Event> {
    let Some(run) = state.runs.get(index) else {
        return Task::none();
    };
    if !visible(state, access, &run.context) || run.terminal() || run.starting || run.job.is_none() {
        return Task::none();
    }
    let client = match access.client.clone() {
        Ok(c) => c,
        Err(e) => return notify_error(notice, e),
    };
    let token = access.token.to_owned();
    let run = &mut state.runs[index];
    run.serial = run.serial.wrapping_add(1);
    let serial = run.serial;
    run.cancelling |= cancel;
    run.error = None;
    let query = run.context.query();
    let job = run.job.clone().unwrap();
    if let Some(handle) = run.poll.take() {
        handle.abort();
    }
    let task = Task::perform(
        async move {
            let result = if cancel {
                GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_cancel(&query, &job.task_id, &token).await
            } else {
                super::sleep::sleep(std::time::Duration::from_millis(350)).await;
                GatewayAppDiagnosticsClient::new(client.transport().clone()).diagnostic_status(&query, &job.task_id, &token).await
            };
            Event::Snapshot(index, serial, result.map_err(error))
        },
        message,
    );
    // A requested remote cancellation must finish even if its panel closes.
    // Only read-only polling belongs to the panel's abortable subscription.
    if cancel {
        return task;
    }
    let (task, handle) = task.abortable();
    state.runs[index].poll = Some(handle);
    task
}
pub fn update(state: &mut ModelState, access: &Access<'_>, notice: &mut Option<Notice>, event: Event) -> Task<Event> {
    match event {
        Event::Open(provider) => {
            let Some(scope) = access.scope.cloned() else {
                return Task::none();
            };
            if !matches!(
                scope.app,
                vw_api_app_identity::AppId::Claude
                    | vw_api_app_identity::AppId::Codex
                    | vw_api_app_identity::AppId::Gemini
            ) || !access
                .catalog
                .as_ref()
                .is_some_and(|s| s.rows.iter().any(|r| r.provider.id == provider))
            {
                return Task::none();
            }
            let Some(project) = access.project.as_deref().filter(|p| !p.is_empty()) else {
                return notify_error(notice, "请先打开已注册的项目");
            };
            let context = ModelContext {
                scope,
                project: project.into(),
                credentials_generation: state.access_generation,
                provider: provider.into(),
            };
            if !current(state, access, &context) {
                return notify_error(notice, error(()));
            }
            state.close();
            state.open = Some(context);
            return update(state, access, notice, Event::Refresh);
        }
        Event::Close => {
            state.close();
        }
        Event::Model(value) => state.model = value,
        Event::Refresh => {
            let count = state.runs.len();
            return Task::batch((0..count).map(|i| poll(state, access, notice, i, false)).collect::<Vec<_>>());
        }
        Event::Start(kind) => {
            if !matches!(
                kind,
                DiagnosticProbeKind::Reachability
                    | DiagnosticProbeKind::Model
                    | DiagnosticProbeKind::Streaming
            ) {
                return notify_error(notice, error(()));
            }
            let Some(context) =
                state.open.clone().filter(|c| visible(state, access, c))
            else {
                return Task::none();
            };
            let model = if kind == DiagnosticProbeKind::Reachability {
                String::new()
            } else {
                state.model.trim().to_owned()
            };
            if kind != DiagnosticProbeKind::Reachability && model.is_empty() {
                return notify_error(notice,
                    "请输入已保存的模型 ID，真实检查会发送最小请求并可能产生费用",
                );
            }
            let client = match access.client.clone() {
                Ok(c) => c,
                Err(e) => return notify_error(notice, e),
            };
            let token = access.token.to_owned();

            if state.runs.iter().any(|r| {
                r.context == context
                    && r.model == model
                    && r.kind == kind
                    && (r.starting || (r.job.is_some() && !r.terminal()))
            }) {
                return Task::none();
            }
            // Keep live job IDs for re-entry. Reject capacity rather than orphaning a probe.
            if state.runs.len() >= 20 {
                return notify_error(notice, "本次会话诊断记录已满，请重新打开应用后继续");
            }
            let index = state.runs.len();
            let run = ModelRun {
                context,
                model,
                kind,
                serial: 0,
                job: None,
                snapshot: None,
                starting: true,
                cancelling: false,
                error: None,
                poll: None,
            };
            state.runs.push(run.clone());
            return Task::perform(
                async move { Event::Started(index, submit(&client, &run, &token).await) },
                message,
            );
        }
        Event::Started(index, result) => {
            let Some(run) = state.runs.get_mut(index) else {
                return Task::none();
            };
            if !run.starting {
                return Task::none();
            }
            run.starting = false;
            match result {
                Ok(job) if run.context.matches(&job, run.kind) => run.job = Some(job),
                _ => {
                    run.error = Some(error(()));
                    return notify_error(notice, error(()));
                }
            }
            // Closing the panel stops polling, but preserves the accepted identity for re-entry.
            return poll(state, access, notice, index, false);
        }
        Event::Cancel(index) => {
            return poll(state, access, notice, index, true);
        }
        Event::Snapshot(index, serial, result) => {
            let Some(run) = state.runs.get(index) else {
                return Task::none();
            };
            if !visible(state, access, &run.context) {
                return Task::none();
            }
            let run = &mut state.runs[index];
            if run.serial != serial || run.terminal() {
                return Task::none();
            }
            match result {
                Ok(snapshot) => {
                    if run.accept(serial, snapshot) {
                        return poll(state, access, notice, index, false);
                    }
                    run.error = Some(error(()));
                    return notify_error(notice, error(()));
                }
                Err(_) => {
                    run.error = Some(error(()));
                    return notify_error(notice, error(()));
                }
            }
        }
    }
    Task::none()
}
