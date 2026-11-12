//! Explicit, revision-bound tool discovery. Specifications never grant invocation permission.
use super::{
    McpMessage, SettingsMessage,
    mcp_management_probe::{self, ProbeTransport},
};
use crate::app::{App, Message, message::agent_apps_diagnostics as diagnostics};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps_diagnostics::*, mcp_management::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub(crate) diagnostics: diagnostics::Context,
    pub(crate) record: McpCatalogRecord,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ToolsState {
    pub(crate) acp_session: String,
    pub(crate) acp_serial: u64,
    pub(crate) acp_loading: bool,
    pub(crate) acp: Option<Result<McpAcpUsage, String>>,
    pub(crate) context: Option<Context>,
    pub(crate) generation: u64,
    pub(crate) serial: u64,
    pub(crate) job: Option<DiagnosticJobIdentity>,
    pub(crate) transport: Option<ProbeTransport>,
    pub(crate) terminal: bool,
    pub(crate) cancelling: bool,
    pub(crate) result: Option<McpToolDetails>,
    pub(crate) error: Option<String>,
}
#[derive(Debug, Clone)]
pub enum ToolsMessage {
    AcpSessionChanged(String),
    ReadAcp,
    AcpLoaded(Context, u64, u64, String, Result<McpAcpUsage, String>),
    Open(McpCatalogRecord),
    Close,
    Cancel,
    Refresh,
    Started(Context, u64, ProbeTransport, Result<DiagnosticJobIdentity, String>),
    Snapshot(Context, u64, u64, Result<DiagnosticJobSnapshot, String>),
}
pub(crate) fn message(event: ToolsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Tools(event)))
}
fn current(app: &App, context: &Context) -> bool {
    diagnostics::context(app).as_ref() == Some(&context.diagnostics)
        && context.diagnostics.connections.iter().any(|record| record == &context.record)
}
pub(crate) fn visible(app: &App) -> Option<&ToolsState> {
    let state = &app.mcp_settings.tools;
    state.context.as_ref().filter(|context| current(app, context)).map(|_| state)
}
fn request(context: &Context, generation: u64) -> McpToolDetailsRequest {
    McpToolDetailsRequest {
        scope: context.record.scope.clone(),
        record: context.record.record.clone(),
        connection_revision: context.record.connection_revision,
        generation,
    }
}
fn identity(context: &Context, job: &DiagnosticJobIdentity) -> bool {
    job.kind == DiagnosticProbeKind::Mcp
        && job.scope.project.tenant_scope() == &context.diagnostics.tenant
        && job.scope.project.project_id() == &context.diagnostics.query.project_id
        && job.scope.target_host_id == context.diagnostics.query.target_host_id
        && job.scope.app_id == context.diagnostics.query.app_id
}
fn safe_error(_: impl std::fmt::Debug) -> String {
    "工具详情未验证，请检查目标主机、目录修订与授权后重试".into()
}
fn close(app: &mut App) -> Task<Message> {
    let state = &mut app.mcp_settings.tools;
    let cleanup = if !state.terminal {
        state.context.take().zip(state.job.take()).zip(state.transport.take()).map(
            |((context, job), transport)| {
                mcp_management_probe::cleanup(transport, context.diagnostics, job)
            },
        )
    } else {
        None
    };
    let generation = state.generation;
    *state = ToolsState { generation, ..Default::default() };
    cleanup.unwrap_or_else(Task::none)
}
/// Navigation, credentials and catalog changes revoke the captured job; they never discover tools.
pub(crate) fn invalidate(app: &mut App) -> Task<Message> {
    use crate::app::{components::system_settings::SystemTab, state::AgentConfigTab};
    let Some(context) = app.mcp_settings.tools.context.as_ref() else {
        return Task::none();
    };
    let on_page = app.show_system_settings
        && app.system_settings_tab == SystemTab::Agents
        && app.agents_settings.active_config_tab == AgentConfigTab::Mcp;
    let mutating = app.mcp_settings.saving
        || app.mcp_settings.bindings.running
        || app.mcp_settings.connection_form.as_ref().is_some_and(|form| form.saving);
    if on_page && current(app, context) && !mutating { Task::none() } else { close(app) }
}
fn poll(app: &mut App, cancel: bool) -> Task<Message> {
    let state = &mut app.mcp_settings.tools;
    let (Some(context), Some(job), Some(transport)) =
        (state.context.clone(), state.job.clone(), state.transport.clone())
    else {
        return Task::none();
    };
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    let generation = state.generation;
    Task::perform(
        async move {
            let result = if cancel {
                transport
                    .client
                    .mcp_management_check_cancel(
                        &context.diagnostics.query,
                        &job.task_id,
                        &transport.token,
                    )
                    .await
            } else {
                crate::app::message::sleep(std::time::Duration::from_millis(350)).await;
                transport
                    .client
                    .mcp_management_check_status(
                        &context.diagnostics.query,
                        &job.task_id,
                        &transport.token,
                    )
                    .await
            }
            .map_err(safe_error);
            ToolsMessage::Snapshot(context, generation, serial, result)
        },
        message,
    )
}
pub(crate) fn update(app: &mut App, event: ToolsMessage) -> Task<Message> {
    match event {
        ToolsMessage::AcpSessionChanged(session) => {
            let state = &mut app.mcp_settings.tools;
            state.acp_session = session;
            state.acp_serial = state.acp_serial.wrapping_add(1);
            state.acp_loading = false;
            state.acp = None;
            Task::none()
        }
        ToolsMessage::ReadAcp => {
            if visible(app).is_none() {
                return Task::none();
            }
            let state = &mut app.mcp_settings.tools;
            let session = state.acp_session.trim().to_owned();
            let (Some(context), Some(transport)) = (state.context.clone(), state.transport.clone())
            else {
                return Task::none();
            };
            if session.is_empty() || state.acp_loading {
                return Task::none();
            }
            state.acp_serial = state.acp_serial.wrapping_add(1);
            let serial = state.acp_serial;
            let generation = state.generation;
            state.acp_loading = true;
            state.acp = None;
            Task::perform(
                async move {
                    let query = McpAcpUsageQuery {
                        target_host_id: context.diagnostics.query.target_host_id.clone(),
                        project_id: context.diagnostics.query.project_id.clone(),
                        session_id: session.clone().into(),
                    };
                    let result = transport
                        .client
                        .mcp_management_acp_usage(&query, &transport.token)
                        .await
                        .map_err(|_| "ACP 挂载未验证，请检查会话身份及独立读取授权".to_owned());
                    ToolsMessage::AcpLoaded(context, generation, serial, session, result)
                },
                message,
            )
        }
        ToolsMessage::AcpLoaded(context, generation, serial, session, result) => {
            let state = &app.mcp_settings.tools;
            if !current(app, &context)
                || state.context.as_ref() != Some(&context)
                || state.generation != generation
                || state.acp_serial != serial
                || state.acp_session.trim() != session
            {
                return Task::none();
            }
            let result = result.and_then(|response| {
                let mut ids = std::collections::HashSet::new();
                if response.target_host_id != context.diagnostics.query.target_host_id
                    || response.project.tenant_scope() != &context.diagnostics.tenant
                    || response.project.project_id() != &context.diagnostics.query.project_id
                    || response.session_id.as_ref() != session
                    || response.mounts.iter().any(|mount| {
                        !ids.insert(&mount.record.id)
                            || match &mount.scope {
                                McpCatalogScope::Tenant { scope } => {
                                    scope != &context.diagnostics.tenant
                                }
                                McpCatalogScope::Project { scope } => scope != &response.project,
                            }
                    })
                {
                    Err("ACP 会话响应身份不匹配，结果未验证".to_owned())
                } else {
                    Ok(response)
                }
            });
            let error = result.as_ref().err().cloned();
            app.mcp_settings.tools.acp_loading = false;
            app.mcp_settings.tools.acp = Some(result);
            if let Some(error) = error { app.show_error_toast(error) } else { Task::none() }
        }
        ToolsMessage::Open(record) => {
            let Some(diagnostics) = diagnostics::context(app) else {
                return app.show_error_toast("请先选择项目、连接目标主机并选择检测应用");
            };
            if !diagnostics.connections.contains(&record) {
                return app.show_error_toast("目录已变化，请刷新后重试");
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(error) => return app.show_error_toast(error),
            };
            let cleanup = close(app);
            let context = Context { diagnostics, record };
            let state = &mut app.mcp_settings.tools;
            state.generation = state.generation.wrapping_add(1);
            state.context = Some(context.clone());
            let generation = state.generation;
            let transport = ProbeTransport { client, token: app.apps_catalog.token.clone() };
            state.transport = Some(transport.clone());
            let task = Task::perform(
                async move {
                    let result = transport
                        .client
                        .mcp_management_tools(
                            &context.diagnostics.query,
                            &request(&context, generation),
                            &transport.token,
                        )
                        .await
                        .map_err(safe_error);
                    ToolsMessage::Started(context, generation, transport, result)
                },
                message,
            );
            Task::batch([cleanup, task])
        }
        ToolsMessage::Close => close(app),
        ToolsMessage::Cancel => {
            if visible(app).is_none() || app.mcp_settings.tools.terminal {
                return Task::none();
            }
            app.mcp_settings.tools.cancelling = true;
            poll(app, true)
        }
        ToolsMessage::Refresh => {
            if visible(app).is_some() && !app.mcp_settings.tools.terminal {
                poll(app, false)
            } else {
                Task::none()
            }
        }
        ToolsMessage::Started(context, generation, transport, result) => {
            let state = &app.mcp_settings.tools;
            if !current(app, &context)
                || state.context.as_ref() != Some(&context)
                || state.generation != generation
            {
                if let Ok(job) = result
                    && identity(&context, &job)
                {
                    return mcp_management_probe::cleanup(transport, context.diagnostics, job);
                }
                return Task::none();
            }
            let state = &mut app.mcp_settings.tools;
            if state.job.is_some() || state.terminal {
                return Task::none();
            }
            match result {
                Ok(job) if identity(&context, &job) => {
                    state.job = Some(job);
                    let cancel = state.cancelling;
                    poll(app, cancel)
                }
                _ => {
                    state.terminal = true;
                    state.error = Some(safe_error(()));
                    app.show_error_toast(safe_error(()))
                }
            }
        }
        ToolsMessage::Snapshot(context, generation, serial, result) => {
            let state = &app.mcp_settings.tools;
            if !current(app, &context)
                || state.context.as_ref() != Some(&context)
                || state.generation != generation
                || state.serial != serial
                || state.terminal
            {
                return Task::none();
            }
            let snapshot = match result {
                Ok(snapshot) if state.job.as_ref() == Some(&snapshot.job) => snapshot,
                _ => {
                    app.mcp_settings.tools.error = Some(safe_error(()));
                    return app.show_error_toast(safe_error(()));
                }
            };
            let expected = request(&context, generation);
            let matching: Vec<_> = snapshot
                .results
                .iter()
                .flatten()
                .filter_map(|result| match result {
                    DiagnosticResult::McpToolDetails { details } if details.request == expected => {
                        Some(details)
                    }
                    _ => None,
                })
                .collect();
            let state = &mut app.mcp_settings.tools;
            state.error = None;
            if let [details] = matching.as_slice() {
                state.result = Some((*details).clone());
            } else if !matching.is_empty() {
                state.result = None;
                state.error = Some(safe_error(()));
            }
            state.terminal = matches!(
                snapshot.state,
                DiagnosticJobState::Succeeded {}
                    | DiagnosticJobState::Failed { .. }
                    | DiagnosticJobState::Cancelled {}
                    | DiagnosticJobState::Interrupted {}
            );
            state.cancelling |= matches!(snapshot.state, DiagnosticJobState::Cancelling {});
            if state.terminal {
                if state.result.is_none() {
                    state.error =
                        Some(if matches!(snapshot.state, DiagnosticJobState::Cancelled {}) {
                            "工具发现已取消".into()
                        } else {
                            safe_error(())
                        });
                }
                Task::none()
            } else {
                poll(app, false)
            }
        }
    }
}
