use crate::{
    diagnostic_context as diagnostics,
    management_effect::Effect,
    management_probe::{self as mcp_management_probe, ProbeTransport},
};
use iced::Task;
use vw_api_app_diagnostics::*;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
pub struct Update<'a> {
    pub state: &'a mut ToolsState,
    pub current: Option<diagnostics::Context>,
    pub on_page: bool,
    pub mutating: bool,
    pub client: Result<GatewayClient, String>,
    pub token: String,
    pub effects: Vec<Effect>,
}
impl Update<'_> {
    fn show_error_toast(&mut self, value: impl Into<String>) -> Task<ToolsMessage> {
        self.effects.push(Effect::Error(value.into()));
        Task::none()
    }
}
fn current_visible(app: &Update<'_>) -> bool {
    app.state.context.as_ref().is_some_and(|context| current(app, context))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub diagnostics: diagnostics::Context,
    pub record: McpCatalogRecord,
}
#[derive(Debug, Clone, Default)]
pub struct ToolsState {
    pub acp_session: String,
    pub acp_serial: u64,
    pub acp_loading: bool,
    pub acp: Option<Result<McpAcpUsage, String>>,
    pub context: Option<Context>,
    pub generation: u64,
    pub serial: u64,
    pub job: Option<DiagnosticJobIdentity>,
    pub transport: Option<ProbeTransport>,
    pub terminal: bool,
    pub cancelling: bool,
    pub result: Option<McpToolDetails>,
    pub error: Option<String>,
}
#[derive(Debug, Clone)]
pub enum ToolsMessage {
    Cleaned(Result<(), String>),
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
fn current(app: &Update<'_>, context: &Context) -> bool {
    app.current.as_ref() == Some(&context.diagnostics)
        && context.diagnostics.connections.iter().any(|record| record == &context.record)
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
fn close(app: &mut Update<'_>) -> Task<ToolsMessage> {
    let state = &mut *app.state;
    let cleanup = if !state.terminal {
        state.context.take().zip(state.job.take()).zip(state.transport.take()).map(
            |((context, job), transport)| {
                mcp_management_probe::cleanup(transport, context.diagnostics, job)
                    .map(ToolsMessage::Cleaned)
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
pub fn invalidate(app: &mut Update<'_>) -> Task<ToolsMessage> {
    let Some(context) = app.state.context.as_ref() else {
        return Task::none();
    };
    if app.on_page && current(app, context) && !app.mutating { Task::none() } else { close(app) }
}
fn poll(app: &mut Update<'_>, cancel: bool) -> Task<ToolsMessage> {
    let state = &mut *app.state;
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
                gateway_mcp_probe::GatewayMcpProbeClient::new(transport.client.transport().clone())
                    .mcp_management_check_cancel(
                        &context.diagnostics.query,
                        &job.task_id,
                        &transport.token,
                    )
                    .await
            } else {
                crate::management_probe::sleep(std::time::Duration::from_millis(350)).await;
                gateway_mcp_probe::GatewayMcpProbeClient::new(transport.client.transport().clone())
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
        std::convert::identity,
    )
}
pub fn update(app: &mut Update<'_>, event: ToolsMessage) -> Task<ToolsMessage> {
    match event {
        ToolsMessage::Cleaned(result) => match result {
            Ok(()) => Task::none(),
            Err(error) => app.show_error_toast(error),
        },
        ToolsMessage::AcpSessionChanged(session) => {
            let state = &mut *app.state;
            state.acp_session = session;
            state.acp_serial = state.acp_serial.wrapping_add(1);
            state.acp_loading = false;
            state.acp = None;
            Task::none()
        }
        ToolsMessage::ReadAcp => {
            if !current_visible(app) {
                return Task::none();
            }
            let state = &mut *app.state;
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
                    let result = gateway_mcp_read::GatewayMcpReadClient::new(
                        transport.client.transport().clone(),
                    )
                    .mcp_management_acp_usage(&query, &transport.token)
                    .await
                    .map_err(|_| "ACP 挂载未验证，请检查会话身份及独立读取授权".to_owned());
                    ToolsMessage::AcpLoaded(context, generation, serial, session, result)
                },
                std::convert::identity,
            )
        }
        ToolsMessage::AcpLoaded(context, generation, serial, session, result) => {
            let state = &*app.state;
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
            app.state.acp_loading = false;
            app.state.acp = Some(result);
            if let Some(error) = error { app.show_error_toast(error) } else { Task::none() }
        }
        ToolsMessage::Open(record) => {
            let Some(diagnostics) = app.current.clone() else {
                return app.show_error_toast("请选择检测应用并刷新目录");
            };
            if !diagnostics.connections.contains(&record) {
                return app.show_error_toast("目录已变化，请刷新后重试");
            }
            let client = match app.client.clone() {
                Ok(client) => client,
                Err(error) => return app.show_error_toast(error),
            };
            let cleanup = close(app);
            let context = Context { diagnostics, record };
            let state = &mut *app.state;
            state.generation = state.generation.wrapping_add(1);
            state.context = Some(context.clone());
            let generation = state.generation;
            let transport = ProbeTransport { client, token: app.token.clone() };
            state.transport = Some(transport.clone());
            let task = Task::perform(
                async move {
                    let result = gateway_mcp_probe::GatewayMcpProbeClient::new(
                        transport.client.transport().clone(),
                    )
                    .mcp_management_tools(
                        &context.diagnostics.query,
                        &request(&context, generation),
                        &transport.token,
                    )
                    .await
                    .map_err(safe_error);
                    ToolsMessage::Started(context, generation, transport, result)
                },
                std::convert::identity,
            );
            Task::batch([cleanup, task])
        }
        ToolsMessage::Close => close(app),
        ToolsMessage::Cancel => {
            if !current_visible(app) || app.state.terminal {
                return Task::none();
            }
            app.state.cancelling = true;
            poll(app, true)
        }
        ToolsMessage::Refresh => {
            if current_visible(app) && !app.state.terminal {
                poll(app, false)
            } else {
                Task::none()
            }
        }
        ToolsMessage::Started(context, generation, transport, result) => {
            let state = &*app.state;
            if !current(app, &context)
                || state.context.as_ref() != Some(&context)
                || state.generation != generation
            {
                if let Ok(job) = result
                    && identity(&context, &job)
                {
                    return mcp_management_probe::cleanup(transport, context.diagnostics, job)
                        .map(ToolsMessage::Cleaned);
                }
                return Task::none();
            }
            let state = &mut *app.state;
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
            let state = &*app.state;
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
                    app.state.error = Some(safe_error(()));
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
            let state = &mut *app.state;
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

fn map_view_message(event: crate::tools::ToolsMessage) -> ToolsMessage {
    use self::ToolsMessage as M;
    match event {
        crate::tools::ToolsMessage::Open(v) => M::Open(v),
        crate::tools::ToolsMessage::Close => M::Close,
        crate::tools::ToolsMessage::AcpSessionChanged(v) => M::AcpSessionChanged(v),
        crate::tools::ToolsMessage::ReadAcp => M::ReadAcp,
        crate::tools::ToolsMessage::Cancel => M::Cancel,
        crate::tools::ToolsMessage::Refresh => M::Refresh,
    }
}

pub fn card<'a>(
    state: &'a ToolsState,
    catalog: Option<&'a Result<vw_api_mcp_management::McpCatalogResponse, String>>,
) -> iced::Element<'a, ToolsMessage> {
    let context = state.context.as_ref().expect("visible tool context");
    crate::tools::card(
        crate::tools::ToolsView {
            record: &context.record,
            acp_session: &state.acp_session,
            acp_loading: state.acp_loading,
            acp: &state.acp,
            result: &state.result,
            error: &state.error,
            cancelling: state.cancelling,
            terminal: state.terminal,
        },
        catalog,
    )
    .map(map_view_message)
}

pub fn visible<'a>(
    state: &'a ToolsState,
    current: Option<&crate::diagnostic_context::Context>,
) -> Option<&'a ToolsState> {
    state
        .context
        .as_ref()
        .filter(|c| {
            current == Some(&c.diagnostics) && c.diagnostics.connections.contains(&c.record)
        })
        .map(|_| state)
}
