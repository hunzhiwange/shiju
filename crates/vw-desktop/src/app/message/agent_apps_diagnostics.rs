//! Desktop MCP probes use the host's authorized diagnostic jobs, never production mounts.
use crate::app::{
    App, Message,
    message::settings::{McpMessage, SettingsMessage, mcp_management_list as catalog},
};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps_diagnostics::*, mcp_management::McpRecordId};

pub(crate) mod mcp_results;
mod mcp_state;
use super::settings::mcp_management_probe::ProbeTransport;
pub(crate) use mcp_state::*;

#[derive(Debug, Clone)]
pub enum McpDiagnosticMessage {
    Select(AppChoice),
    Start(Vec<McpRecordId>),
    Cancel,
    Refresh,
    Started(Context, u64, ProbeTransport, Result<DiagnosticJobIdentity, String>),
    Cleaned(Result<(), String>),
    Snapshot(Context, u64, u64, Result<DiagnosticJobSnapshot, String>),
}
pub(crate) fn message(event: McpDiagnosticMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Diagnostic(event)))
}
pub(crate) fn context(app: &App) -> Option<Context> {
    let catalog = catalog::context(app)?;
    let project = app.project_id.as_deref().filter(|p| !p.is_empty())?;
    let choice = app.mcp_settings.diagnostics.app.as_ref()?;
    if catalog.host.is_empty()
        || app.apps_catalog.token.is_empty()
        || app.apps_catalog.context.as_ref() != Some(&catalog.endpoint)
        || !app.apps_catalog.rows.iter().any(|r| r.entry.id == choice.0)
    {
        return None;
    }
    let connections = catalog::visible(app)?.as_ref().ok()?.records.clone();
    let records = connections.iter().map(|r| r.record.clone()).collect();
    Some(Context {
        records,
        connections,
        query: DiagnosticScopeQuery {
            project_id: project.into(),
            target_host_id: catalog.host.clone(),
            app_id: choice.0,
        },
        gateway: catalog.endpoint.gateway,
        tenant: catalog.endpoint.tenant,
        project_path: catalog.endpoint.project,
        credentials_generation: catalog.credentials_generation,
    })
}
fn valid_identity(context: &Context, job: &DiagnosticJobIdentity) -> bool {
    job.kind == DiagnosticProbeKind::Mcp
        && job.scope.target_host_id == context.query.target_host_id
        && job.scope.app_id == context.query.app_id
        && job.scope.project.project_id() == &context.query.project_id
        && job.scope.project.tenant_scope() == &context.tenant
}
fn safe_error(_: impl std::fmt::Debug) -> String {
    "MCP 检测请求失败，结果未验证；请检查授权和目标主机".into()
}
fn poll(app: &mut App, context: Context, cancel: bool) -> Task<Message> {
    let state = &mut app.mcp_settings.diagnostics;
    let Some(transport) = state.transport.clone() else {
        return Task::none();
    };
    let Some(job) = state.job.clone() else {
        return Task::none();
    };
    let generation = state.generation;
    state.request_serial = state.request_serial.wrapping_add(1);
    let serial = state.request_serial;
    Task::perform(
        async move {
            let result = async {
                if cancel {
                    transport
                        .client
                        .mcp_management_check_cancel(&context.query, &job.task_id, &transport.token)
                        .await
                        .map_err(safe_error)
                } else {
                    crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                    transport
                        .client
                        .mcp_management_check_status(&context.query, &job.task_id, &transport.token)
                        .await
                        .map_err(safe_error)
                }
            }
            .await;
            McpDiagnosticMessage::Snapshot(context, generation, serial, result)
        },
        message,
    )
}
pub(crate) fn update(app: &mut App, event: McpDiagnosticMessage) -> Task<Message> {
    match event {
        McpDiagnosticMessage::Select(choice) => {
            if !app.mcp_settings.diagnostics.active()
                || context(app).as_ref() != app.mcp_settings.diagnostics.context.as_ref()
            {
                app.mcp_settings.diagnostics.app = Some(choice);
            }
        }
        McpDiagnosticMessage::Start(ids) => {
            let Some(context) = context(app) else {
                return app.show_error_toast("请先选择项目、连接目标主机并选择检测应用");
            };
            let state = &app.mcp_settings.diagnostics;
            if state.active() && state.context.as_ref() == Some(&context) {
                return Task::none();
            }
            let Some(Ok(catalog)) = catalog::visible(app) else {
                return app.show_error_toast("请先刷新受管 MCP 目录");
            };
            let targets: Vec<_> = catalog
                .records
                .iter()
                .filter(|r| ids.contains(&r.record.id))
                .map(|r| r.record.clone())
                .collect();
            if ids.is_empty() || targets.len() != ids.len() {
                return app.show_error_toast("检测目标无效，请刷新目录");
            }
            let client = match crate::app::config::gateway_client() {
                Ok(c) => c,
                Err(e) => return app.show_error_toast(e),
            };
            let cleanup = if app.mcp_settings.diagnostics.context.as_ref() != Some(&context) {
                super::settings::mcp_management_probe::invalidate(app)
            } else {
                Task::none()
            };
            let state = &mut app.mcp_settings.diagnostics;
            state.begin(context.clone(), targets);
            let generation = state.generation;
            let transport = ProbeTransport { client, token: app.apps_catalog.token.clone() };
            state.transport = Some(transport.clone());
            let submit_task = Task::perform(
                async move {
                    let result = submit(
                        transport.client.clone(),
                        &context,
                        ids,
                        &transport.token,
                        generation,
                    )
                    .await;
                    McpDiagnosticMessage::Started(context, generation, transport, result)
                },
                message,
            );
            return Task::batch([cleanup, submit_task]);
        }
        McpDiagnosticMessage::Cleaned(result) => {
            if let Err(error) = result {
                return app.show_error_toast(error);
            }
        }
        McpDiagnosticMessage::Started(request, generation, transport, result) => {
            let stale = context(app).as_ref() != Some(&request)
                || app.mcp_settings.diagnostics.context.as_ref() != Some(&request)
                || app.mcp_settings.diagnostics.generation != generation
                || app.mcp_settings.diagnostics.terminal;
            if stale {
                if let Ok(job) = result
                    && valid_identity(&request, &job)
                {
                    return super::settings::mcp_management_probe::cleanup(transport, request, job);
                }
                return Task::none();
            }
            let state = &mut app.mcp_settings.diagnostics;
            if state.job.is_some() {
                return Task::none();
            }
            match result {
                Ok(job) if valid_identity(&request, &job) => {
                    state.job = Some(job);
                    let cancel = state.cancelling;
                    return poll(app, request, cancel);
                }
                _ => {
                    state.terminal = true;
                    state.transport = None;
                    state.targets.iter_mut().for_each(|(_, p)| *p = Phase::Unverified);
                    state.error = Some(safe_error(()));
                    return app.show_error_toast(safe_error(()));
                }
            }
        }
        McpDiagnosticMessage::Cancel | McpDiagnosticMessage::Refresh => {
            let Some(context) = context(app) else {
                return Task::none();
            };
            let state = &mut app.mcp_settings.diagnostics;
            if state.context.as_ref() != Some(&context) || !state.active() {
                return Task::none();
            }
            let cancel = matches!(event, McpDiagnosticMessage::Cancel);
            if cancel {
                if state.cancelling {
                    return Task::none();
                }
                state.cancelling = true;
                state.request_serial = state.request_serial.wrapping_add(1);
            }
            if state.job.is_none() {
                return Task::none();
            }
            state.error = None;
            return poll(app, context, cancel);
        }
        McpDiagnosticMessage::Snapshot(request, generation, serial, result) => {
            let Some(current) = context(app) else {
                return Task::none();
            };
            let state = &mut app.mcp_settings.diagnostics;
            if current != request
                || state.context.as_ref() != Some(&request)
                || state.generation != generation
                || state.request_serial != serial
                || state.terminal
            {
                return Task::none();
            }
            match result {
                Ok(snapshot) => {
                    if !state.accept(&current, &request, generation, serial, snapshot) {
                        state.error = Some(safe_error(()));
                        return app.show_error_toast(safe_error(()));
                    }
                    if !state.terminal {
                        return poll(app, request, false);
                    }
                    state.transport = None;
                }
                Err(error) => {
                    state.cancelling = false;
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
    }
    Task::none()
}
