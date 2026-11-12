//! Endpoint probes share the authorized job SDK. Selecting never writes provider configuration.
use super::{endpoint_state::*, model_state::ModelContext};
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_diagnostics::*;
#[derive(Clone, Debug)]
pub struct Reply {
    pub(crate) context: ModelContext,
    pub(crate) generation: u64,
    pub(crate) transport: Transport,
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
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::EndpointDiagnostics(event))
}
fn error(_: impl std::fmt::Debug) -> String {
    "端点测速请求未验证，请检查授权与目标主机后重试".into()
}
fn current(app: &App, context: &ModelContext, generation: u64) -> bool {
    app.app_providers.generation == generation
        && app.app_providers.scope.as_ref() == Some(&context.scope)
        && app.apps_catalog.context.as_ref() == Some(&context.scope.context)
        && app.apps_catalog.model_diagnostics.access_generation == context.credentials_generation
        && app.project_id.as_deref() == Some(context.project.as_ref())
        && !app.apps_catalog.token.is_empty()
        && list::current(app, context.scope.app).is_ok_and(|(_, s)| s == context.scope)
        && app
            .app_providers
            .form
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
            transport
                .client
                .diagnostic_cancel(&context.query(), &job.task_id, &transport.token)
                .await
                .map_err(error)
        },
        |r| message(Event::Cleaned(r)),
    )
}
fn poll(app: &mut App, cancel: bool) -> Task<Message> {
    let Some(state) = &mut app.app_providers.endpoint_speed else { return Task::none() };
    let Some(job) = state.job.clone().filter(|_| !state.terminal()) else { return Task::none() };
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    state.cancelling |= cancel;
    let (context, generation, transport) =
        (state.context.clone(), state.generation, state.transport.clone());
    Task::perform(
        async move {
            let result = if cancel {
                transport
                    .client
                    .diagnostic_cancel(&context.query(), &job.task_id, &transport.token)
                    .await
            } else {
                crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                transport
                    .client
                    .diagnostic_status(&context.query(), &job.task_id, &transport.token)
                    .await
            }
            .map_err(error);
            Event::Snapshot(Reply { context, generation, transport }, serial, result)
        },
        message,
    )
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            if app.app_providers.endpoint_speed.is_some() {
                return Task::none();
            }
            let Some(form) = app.app_providers.form.as_ref().filter(|f| editable(f)) else {
                return Task::none();
            };
            let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
            let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else {
                return app.show_error_toast("请先打开已注册项目");
            };
            let context = ModelContext {
                scope,
                project: project.into(),
                provider: form.id.clone().into(),
                credentials_generation: app.apps_catalog.model_diagnostics.access_generation,
            };
            let generation = app.app_providers.generation;
            if !current(app, &context, generation) {
                return app.show_error_toast(error(()));
            }
            let client = match crate::app::config::gateway_client() {
                Ok(c) => c,
                Err(e) => return app.show_error_toast(e),
            };
            let transport = Transport { client, token: app.apps_catalog.token.clone() };
            app.app_providers.generation = generation.wrapping_add(1);
            let generation = app.app_providers.generation;
            app.app_providers.endpoint_speed = Some(State {
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
            let Some(state) = app.app_providers.endpoint_speed.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation)
                || state.active()
                || state.loading
                || app.app_providers.endpoints.is_some()
            {
                return Task::none();
            }
            // Reuse the existing address-book form and SDK, retaining the provider draft underneath.
            app.app_providers.endpoints =
                Some(crate::app::state::app_providers::endpoints::Form::new(
                    state.context.provider.as_ref().to_owned(),
                ));
            return crate::app::message::app_providers::endpoints::update(
                app,
                crate::app::message::app_providers::endpoints::Event::Refresh,
            );
        }
        Event::Preset(value) => {
            if let Some(state) =
                app.app_providers.endpoint_speed.as_mut().filter(|s| !s.active() && !s.loading)
            {
                state.preset_name = value;
            }
        }
        Event::Reload => {
            let Some(state) = app.app_providers.endpoint_speed.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation) || state.active() || state.loading {
                return Task::none();
            }
            let Some(form) = &app.app_providers.form else { return Task::none() };
            let query = match query(form, &state.preset_name) {
                Ok(q) => q,
                Err(e) => return app.show_error_toast(e),
            };
            let state = app.app_providers.endpoint_speed.as_mut().unwrap();
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
                        transport
                            .client
                            .diagnostic_endpoint_candidates(
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
            let Some(state) = &mut app.app_providers.endpoint_speed else { return Task::none() };
            if state.context != context || state.generation != generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(response) => state.candidates = response.candidates,
                Err(e) => {
                    state.error = Some(e.clone());
                    return app.show_error_toast(e);
                }
            }
        }
        Event::Select(id) => {
            if let Some(state) = &mut app.app_providers.endpoint_speed {
                state.choose(id);
            }
        }
        Event::Apply => {
            let Some(state) = app.app_providers.endpoint_speed.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation) {
                return Task::none();
            }
            let Some(form) = app.app_providers.form.as_mut() else { return Task::none() };
            if let Err(e) = state.apply(form) {
                return app.show_error_toast(e);
            }
            app.app_providers.endpoint_speed = None;
            return app.show_info_toast("端点已回填草稿，请点击供应商表单的保存");
        }
        Event::Start => {
            let Some(state) = app.app_providers.endpoint_speed.as_ref() else {
                return Task::none();
            };
            if !current(app, &state.context, state.generation)
                || state.active()
                || state.loading
                || state.candidates.is_empty()
            {
                return Task::none();
            }
            let state = app.app_providers.endpoint_speed.as_mut().unwrap();
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
                    let result = transport
                        .client
                        .diagnostic_start(&context.query(), &request, &transport.token)
                        .await
                        .map_err(error);
                    Event::Started(Reply { context, generation, transport }, result)
                },
                message,
            );
        }
        Event::Started(Reply { context, generation, transport }, result) => {
            if !current(app, &context, generation)
                || app
                    .app_providers
                    .endpoint_speed
                    .as_ref()
                    .is_none_or(|s| s.generation != generation || s.context != context)
            {
                if let Ok(job) = result {
                    return cleanup(context, transport, job);
                }
                return Task::none();
            }
            let state = app.app_providers.endpoint_speed.as_mut().unwrap();
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
                    return app.show_error_toast(error(()));
                }
            }
            let cancel = state.cancelling;
            return poll(app, cancel);
        }
        Event::Cancel | Event::Close => {
            let Some(state) = &mut app.app_providers.endpoint_speed else { return Task::none() };
            if !state.active() {
                if matches!(event, Event::Close) {
                    app.app_providers.endpoint_speed = None;
                }
                return Task::none();
            }
            state.closing |= matches!(event, Event::Close);
            state.cancelling = true;
            return poll(app, true);
        }
        Event::Snapshot(Reply { context, generation, transport }, serial, result) => {
            if !current(app, &context, generation)
                || app
                    .app_providers
                    .endpoint_speed
                    .as_ref()
                    .is_none_or(|s| s.generation != generation || s.context != context)
            {
                if let Ok(snapshot) = result {
                    return cleanup(context, transport, snapshot.job);
                }
                return Task::none();
            }
            let state = app.app_providers.endpoint_speed.as_mut().unwrap();
            if state.serial != serial || state.terminal() {
                return Task::none();
            }
            match result {
                Ok(snapshot) => {
                    if !state.accept(serial, snapshot) {
                        state.error = Some(error(()));
                        return app.show_error_toast(error(()));
                    }
                    state.error = None;
                    if state.terminal() {
                        if state.closing {
                            app.app_providers.endpoint_speed = None;
                        }
                    } else {
                        return poll(app, false);
                    }
                }
                _ => {
                    state.error = Some(error(()));
                    state.cancelling = false;
                    return app.show_error_toast(error(()));
                }
            }
        }
        Event::Cleaned(result) => {
            if result.is_err() {
                return app.show_error_toast("旧测速取消未确认，请检查执行主机任务");
            }
        }
    }
    Task::none()
}
