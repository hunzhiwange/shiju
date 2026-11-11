//! Desktop MCP probes use the host's authorized diagnostic jobs, never production mounts.
use crate::app::{
    App, Message,
    message::settings::{McpMessage, SettingsMessage, mcp_management_list as catalog},
};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps_diagnostics::*, mcp_management::McpRecordId};

pub(crate) mod mcp_results;
mod mcp_state;
pub(crate) use mcp_state::*;

#[derive(Debug, Clone)]
pub enum McpDiagnosticMessage {
    Select(AppChoice),
    Start(Vec<McpRecordId>),
    Cancel,
    Refresh,
    Started(Context, u64, Result<DiagnosticJobIdentity, String>),
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
    let records =
        catalog::visible(app)?.as_ref().ok()?.records.iter().map(|r| r.record.clone()).collect();
    Some(Context {
        records,
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
    // Capture the transport together with the authorization context before yielding.
    let client = match crate::app::config::gateway_client() {
        Ok(client)
            if client.endpoint().base_url() == context.gateway
                && client.endpoint().tenant_scope() == &context.tenant =>
        {
            client
        }
        _ => return Task::none(),
    };
    let state = &mut app.mcp_settings.diagnostics;
    let Some(job) = state.job.clone() else {
        return Task::none();
    };
    let generation = state.generation;
    state.request_serial = state.request_serial.wrapping_add(1);
    let serial = state.request_serial;
    let token = app.apps_catalog.token.clone();
    Task::perform(
        async move {
            let result = async {
                if cancel {
                    client
                        .diagnostic_cancel(&context.query, &job.task_id, &token)
                        .await
                        .map_err(safe_error)
                } else {
                    crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                    client
                        .diagnostic_status(&context.query, &job.task_id, &token)
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
            let state = &mut app.mcp_settings.diagnostics;
            state.begin(context.clone(), targets);
            let generation = state.generation;
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    let result = submit(client, &context, ids, &token).await;
                    McpDiagnosticMessage::Started(context, generation, result)
                },
                message,
            );
        }
        McpDiagnosticMessage::Started(request, generation, result) => {
            if context(app).as_ref() != Some(&request) {
                return Task::none();
            }
            let state = &mut app.mcp_settings.diagnostics;
            if state.context.as_ref() != Some(&request)
                || state.generation != generation
                || state.job.is_some()
            {
                return Task::none();
            }
            match result {
                Ok(job) if valid_identity(&request, &job) => {
                    state.job = Some(job);
                    return poll(app, request, false);
                }
                _ => {
                    state.terminal = true;
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
            if state.context.as_ref() != Some(&context) || !state.active() || state.job.is_none() {
                return Task::none();
            }
            let cancel = matches!(event, McpDiagnosticMessage::Cancel);
            if cancel {
                state.cancelling = true;
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
                        return Task::none();
                    }
                    if !state.terminal {
                        return poll(app, request, false);
                    }
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
    }
    Task::none()
}
