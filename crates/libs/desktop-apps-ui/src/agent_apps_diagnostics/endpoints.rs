//! Endpoint probes share the authorized job SDK. Selecting never writes provider configuration.
use gateway_app_diagnostics::GatewayAppDiagnosticsClient;
use super::endpoint_state::*;
use crate::diagnostics::model_state::ModelContext;
use desktop_provider_apps::state::edit::EditForm;
use crate::{Notice, error as notify_error};
type Message = Event;
fn message(event: Event) -> Event { event }
pub struct Host<'a> {
    pub state: &'a mut Option<State>,
    pub form: &'a mut Option<EditForm>,
    pub generation: &'a mut u64,
    pub scope: Option<&'a crate::ProviderScope>,
    pub catalog_context: Option<&'a crate::AgentAppsContext>,
    pub credentials_generation: u64,
    pub project: Option<&'a str>,
    pub token: &'a str,
    pub current_scope: Option<crate::ProviderScope>,
    pub endpoints_open: bool,
    pub client: Option<Result<vw_gateway_client::GatewayClient, String>>,
    pub notice: &'a mut Option<Notice>,
    pub applied: &'a mut bool,
    pub manage_custom: &'a mut Option<String>,
}
use iced::Task;
use vw_api_app_diagnostics::*;
#[derive(Clone, Debug)]
pub struct Reply {
    pub context: ModelContext,
    pub generation: u64,
    pub transport: Transport,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    ManageCustom,
    Close,
    Reload,
    Preset(String),
    Start,
    Cancel,
    Select(DiagnosticEndpointId),
    Apply,
    Candidates(Reply, Result<DiagnosticEndpointCandidatesResponse, String>),
    Started(Reply, Result<DiagnosticJobIdentity, String>),
    Snapshot(Reply, u64, Result<DiagnosticJobSnapshot, String>),
    Cleaned(Result<DiagnosticJobSnapshot, String>),
}
fn error(_: impl std::fmt::Debug) -> String {
    "端点测速请求未验证，请检查授权与目标主机后重试".into()
}
fn current(app: &Host<'_>, context: &ModelContext, generation: u64) -> bool {
    *app.generation == generation
        && app.scope == Some(&context.scope)
        && app.credentials_generation == context.credentials_generation
        && vw_api_transport::local_settings_project(app.project) == context.project.as_ref()

        && app.current_scope.as_ref() == Some(&context.scope)
        && app.form
            .as_ref()
            .is_some_and(|f| f.id == context.provider.as_ref() && editable(f))
}
fn cleanup(
    context: ModelContext,
    transport: Transport,
    job: DiagnosticJobIdentity,
) -> Task<Message> {
    if !context.matches(&job, DiagnosticProbeKind::Speed) {
        return Task::none();
    }
    Task::perform(
        async move {
            GatewayAppDiagnosticsClient::new(transport.client.transport().clone()).diagnostic_cancel(&context.query(), &job.task_id, &transport.token)
                .await
                .map_err(error)
        },
        |r| message(Event::Cleaned(r)),
    )
}
fn poll(app: &mut Host<'_>, cancel: bool) -> Task<Message> {
    let Some(state) = &mut *app.state else { return Task::none() };
    let Some(job) = state.job.clone().filter(|_| !state.terminal()) else { return Task::none() };
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    state.cancelling |= cancel;
    let (context, generation, transport) =
        (state.context.clone(), state.generation, state.transport.clone());
    Task::perform(
        async move {
            let result = if cancel {
                GatewayAppDiagnosticsClient::new(transport.client.transport().clone()).diagnostic_cancel(&context.query(), &job.task_id, &transport.token)
                    .await
            } else {
                crate::diagnostics::sleep::sleep(std::time::Duration::from_millis(350)).await;
                GatewayAppDiagnosticsClient::new(transport.client.transport().clone()).diagnostic_status(&context.query(), &job.task_id, &transport.token)
                    .await
            }
            .map_err(error);
            Event::Snapshot(Reply { context, generation, transport }, serial, result)
        },
        message,
    )
}
pub fn update(app: &mut Host<'_>, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            if app.state.is_some() {
                return Task::none();
            }
            let Some(form) = app.form.as_ref().filter(|f| editable(f)) else {
                return Task::none();
            };
            let Some(scope) = app.scope.cloned() else { return Task::none() };
            let project = vw_api_transport::local_settings_project(app.project).to_owned();
            let context = ModelContext {
                scope,
                project: project.into(),
                provider: form.id.clone().into(),
                credentials_generation: app.credentials_generation,
            };
            let generation = *app.generation;
            if !current(app, &context, generation) {
                return notify_error(app.notice, error(()));
            }
            let client = match app.client.take().expect("open endpoint client") {
                Ok(c) => c,
                Err(e) => return notify_error(app.notice, e),
            };
            let transport = Transport { client, token: app.token.to_owned() };
            *app.generation = generation.wrapping_add(1);
            let generation = *app.generation;
            *app.state = Some(State {
                context: context.clone(),
                generation,
                transport: transport.clone(),
                candidates: vec![],
                selected: None,
                preset_name: String::new(),
                loading: false,
                starting: false,
                cancelling: false,
                closing: false,
                serial: 0,
                job: None,
                snapshot: None,
                error: None,
            });
            return update(app, Event::Reload);
        }
        Event::ManageCustom => {
            let Some(state) = app.state.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation)
                || state.active()
                || state.loading
                || app.endpoints_open
            {
                return Task::none();
            }
            // Reuse the existing address-book form and SDK, retaining the provider draft underneath.
            *app.manage_custom = Some(state.context.provider.as_ref().to_owned());
            return Task::none();
        }
        Event::Preset(value) => {
            if let Some(state) =
                app.state.as_mut().filter(|s| !s.active() && !s.loading)
            {
                state.preset_name = value;
            }
        }
        Event::Reload => {
            let Some(state) = app.state.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation) || state.active() || state.loading {
                return Task::none();
            }
            let Some(form) = &*app.form else { return Task::none() };
            let query = match query(form, &state.preset_name) {
                Ok(q) => q,
                Err(e) => return notify_error(app.notice, e),
            };
            let state = app.state.as_mut().unwrap();
            state.loading = true;
            state.error = None;
            state.candidates.clear();
            state.selected = None;
            state.snapshot = None;
            state.job = None;
            let (context, generation, transport) =
                (state.context.clone(), state.generation, state.transport.clone());
            return Task::perform(
                async move {
                    Event::Candidates(
                        Reply {
                            context: context.clone(),
                            generation,
                            transport: transport.clone(),
                        },
                        GatewayAppDiagnosticsClient::new(transport.client.transport().clone()).diagnostic_endpoint_candidates(
                                &context.query(),
                                &query,
                                &transport.token,
                            )
                            .await
                            .map_err(error),
                    )
                },
                message,
            );
        }
        Event::Candidates(Reply { context, generation, .. }, result) => {
            if !current(app, &context, generation) {
                return Task::none();
            }
            let Some(state) = &mut *app.state else { return Task::none() };
            if state.context != context || state.generation != generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(response) => state.candidates = response.candidates,
                Err(e) => {
                    state.error = Some(e.clone());
                    return notify_error(app.notice, e);
                }
            }
        }
        Event::Select(id) => {
            if let Some(state) = &mut *app.state {
                state.choose(id);
            }
        }
        Event::Apply => {
            let Some(state) = app.state.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation) {
                return Task::none();
            }
            let Some(form) = app.form.as_mut() else { return Task::none() };
            if let Err(e) = state.apply(form) {
                return notify_error(app.notice, e);
            }
            *app.state = None;
            *app.applied = true;
            return Task::none();
        }
        Event::Start => {
            let Some(state) = app.state.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation)
                || state.active()
                || state.loading
                || state.candidates.is_empty()
            {
                return Task::none();
            }
            let state = app.state.as_mut().unwrap();
            state.starting = true;
            state.cancelling = false;
            state.error = None;
            state.job = None;
            state.snapshot = None;
            let (context, generation, transport) =
                (state.context.clone(), state.generation, state.transport.clone());
            let request = DiagnosticStartRequest::Speed {
                provider_id: context.provider.clone(),
                endpoint_ids: state.candidates.iter().map(|c| c.id.clone()).collect(),
            };
            return Task::perform(
                async move {
                    let result = GatewayAppDiagnosticsClient::new(transport.client.transport().clone()).diagnostic_start(&context.query(), &request, &transport.token)
                        .await
                        .map_err(error);
                    Event::Started(Reply { context, generation, transport }, result)
                },
                message,
            );
        }
        Event::Started(Reply { context, generation, transport }, result) => {
            if !current(app, &context, generation)
                || app.state
                    .as_ref()
                    .is_none_or(|s| s.generation != generation || s.context != context)
            {
                if let Ok(job) = result {
                    return cleanup(context, transport, job);
                }
                return Task::none();
            }
            let state = app.state.as_mut().unwrap();
            if !state.starting {
                return Task::none();
            }
            state.starting = false;
            match result {
                Ok(job) if context.matches(&job, DiagnosticProbeKind::Speed) => {
                    state.job = Some(job)
                }
                _ => {
                    state.error = Some(error(()));
                    return notify_error(app.notice, error(()));
                }
            }
            let cancel = state.cancelling;
            return poll(app, cancel);
        }
        Event::Cancel | Event::Close => {
            let Some(state) = &mut *app.state else { return Task::none() };
            if !state.active() {
                if matches!(event, Event::Close) {
                    *app.state = None;
                }
                return Task::none();
            }
            state.closing |= matches!(event, Event::Close);
            state.cancelling = true;
            return poll(app, true);
        }
        Event::Snapshot(Reply { context, generation, transport }, serial, result) => {
            if !current(app, &context, generation)
                || app.state
                    .as_ref()
                    .is_none_or(|s| s.generation != generation || s.context != context)
            {
                if let Ok(snapshot) = result {
                    return cleanup(context, transport, snapshot.job);
                }
                return Task::none();
            }
            let state = app.state.as_mut().unwrap();
            if state.serial != serial || state.terminal() {
                return Task::none();
            }
            match result {
                Ok(snapshot) => {
                    if !state.accept(serial, snapshot) {
                        state.error = Some(error(()));
                        return notify_error(app.notice, error(()));
                    }
                    state.error = None;
                    if state.terminal() {
                        if state.closing {
                            *app.state = None;
                        }
                    } else {
                        return poll(app, false);
                    }
                }
                _ => {
                    state.error = Some(error(()));
                    state.cancelling = false;
                    return notify_error(app.notice, error(()));
                }
            }
        }
        Event::Cleaned(result) => {
            if result.is_err() {
                return notify_error(app.notice, "旧测速取消未确认，请检查执行主机任务");
            }
        }
    }
    Task::none()
}
