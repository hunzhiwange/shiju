//! Model probes only submit IDs to the authorized execution host.
use super::model_state::*;
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::agent_apps_diagnostics::*};

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
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::ModelDiagnostics(event))
}
fn error(_: impl std::fmt::Debug) -> String {
    "诊断请求未验证，请检查执行主机、授权和已保存的供应商/模型后重试".into()
}
fn current(app: &App, context: &ModelContext) -> bool {
    app.apps_catalog.model_diagnostics.access_generation == context.credentials_generation
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.context.as_ref() == Some(&context.scope.context)
        && app.project_id.as_deref() == Some(context.project.as_ref())
        && list::current(app, context.scope.app).is_ok_and(|(_, s)| s == context.scope)
        && app.app_providers.scope.as_ref() == Some(&context.scope)
}
fn visible(app: &App, context: &ModelContext) -> bool {
    current(app, context)
        && app.show_system_settings
        && app.system_settings_tab == crate::app::components::system_settings::SystemTab::Apps
        && app.apps_catalog.model_diagnostics.open.as_ref() == Some(context)
}
pub(crate) fn invalidate(app: &mut App) {
    if app.apps_catalog.model_diagnostics.open.as_ref().is_some_and(|c| !visible(app, c)) {
        app.apps_catalog.model_diagnostics.close();
    }
}
pub(crate) fn can_show(app: &App) -> bool {
    app.apps_catalog.model_diagnostics.open.as_ref().is_some_and(|c| visible(app, c))
}
pub(crate) async fn submit(
    client: &GatewayClient,
    run: &ModelRun,
    token: &str,
) -> Result<DiagnosticJobIdentity, String> {
    let provider_id = run.context.provider.clone();
    let model_id = run.model.clone().into();
    let request = match run.kind {
        DiagnosticProbeKind::Reachability => DiagnosticStartRequest::Reachability { provider_id },
        DiagnosticProbeKind::Model => DiagnosticStartRequest::Model { provider_id, model_id },
        DiagnosticProbeKind::Streaming => {
            DiagnosticStartRequest::Streaming { provider_id, model_id }
        }
        _ => return Err(error(())),
    };
    client.diagnostic_start(&run.context.query(), &request, token).await.map_err(error)
}
fn poll(app: &mut App, index: usize, cancel: bool) -> Task<Message> {
    let Some(run) = app.apps_catalog.model_diagnostics.runs.get(index) else {
        return Task::none();
    };
    if !visible(app, &run.context) || run.terminal() || run.starting || run.job.is_none() {
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() {
        Ok(c) => c,
        Err(e) => return app.show_error_toast(e),
    };
    let token = app.apps_catalog.token.clone();
    let run = &mut app.apps_catalog.model_diagnostics.runs[index];
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
                client.diagnostic_cancel(&query, &job.task_id, &token).await
            } else {
                crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                client.diagnostic_status(&query, &job.task_id, &token).await
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
    app.apps_catalog.model_diagnostics.runs[index].poll = Some(handle);
    task
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Open(provider) => {
            let Some(scope) = app.app_providers.scope.clone() else {
                return Task::none();
            };
            if !matches!(
                scope.app,
                vw_gateway_client::vw_api_types::agent_apps::AppId::Claude
                    | vw_gateway_client::vw_api_types::agent_apps::AppId::Codex
                    | vw_gateway_client::vw_api_types::agent_apps::AppId::Gemini
            ) || !app
                .app_providers
                .snapshot
                .as_ref()
                .is_some_and(|s| s.catalog.rows.iter().any(|r| r.provider.id == provider))
            {
                return Task::none();
            }
            let Some(project) = app.project_id.as_deref().filter(|p| !p.is_empty()) else {
                return app.show_error_toast("请先打开已注册的项目");
            };
            let context = ModelContext {
                scope,
                project: project.into(),
                credentials_generation: app.apps_catalog.model_diagnostics.access_generation,
                provider: provider.into(),
            };
            if !current(app, &context) {
                return app.show_error_toast(error(()));
            }
            app.apps_catalog.model_diagnostics.close();
            app.apps_catalog.model_diagnostics.open = Some(context);
            return update(app, Event::Refresh);
        }
        Event::Close => {
            app.apps_catalog.model_diagnostics.close();
        }
        Event::Model(value) => app.apps_catalog.model_diagnostics.model = value,
        Event::Refresh => {
            let count = app.apps_catalog.model_diagnostics.runs.len();
            return Task::batch((0..count).map(|i| poll(app, i, false)).collect::<Vec<_>>());
        }
        Event::Start(kind) => {
            if !matches!(
                kind,
                DiagnosticProbeKind::Reachability
                    | DiagnosticProbeKind::Model
                    | DiagnosticProbeKind::Streaming
            ) {
                return app.show_error_toast(error(()));
            }
            let Some(context) =
                app.apps_catalog.model_diagnostics.open.clone().filter(|c| visible(app, c))
            else {
                return Task::none();
            };
            let model = if kind == DiagnosticProbeKind::Reachability {
                String::new()
            } else {
                app.apps_catalog.model_diagnostics.model.trim().to_owned()
            };
            if kind != DiagnosticProbeKind::Reachability && model.is_empty() {
                return app.show_error_toast(
                    "请输入已保存的模型 ID，真实检查会发送最小请求并可能产生费用",
                );
            }
            let client = match crate::app::config::gateway_client() {
                Ok(c) => c,
                Err(e) => return app.show_error_toast(e),
            };
            let token = app.apps_catalog.token.clone();
            let state = &mut app.apps_catalog.model_diagnostics;
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
                return app.show_error_toast("本次会话诊断记录已满，请重新打开应用后继续");
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
            let Some(run) = app.apps_catalog.model_diagnostics.runs.get_mut(index) else {
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
                    return app.show_error_toast(error(()));
                }
            }
            // Closing the panel stops polling, but preserves the accepted identity for re-entry.
            return poll(app, index, false);
        }
        Event::Cancel(index) => {
            return poll(app, index, true);
        }
        Event::Snapshot(index, serial, result) => {
            let Some(run) = app.apps_catalog.model_diagnostics.runs.get(index) else {
                return Task::none();
            };
            if !visible(app, &run.context) {
                return Task::none();
            }
            let run = &mut app.apps_catalog.model_diagnostics.runs[index];
            if run.serial != serial || run.terminal() {
                return Task::none();
            }
            match result {
                Ok(snapshot) => {
                    if run.accept(serial, snapshot) {
                        return poll(app, index, false);
                    }
                    run.error = Some(error(()));
                    return app.show_error_toast(error(()));
                }
                Err(_) => {
                    run.error = Some(error(()));
                    return app.show_error_toast(error(()));
                }
            }
        }
    }
    Task::none()
}
