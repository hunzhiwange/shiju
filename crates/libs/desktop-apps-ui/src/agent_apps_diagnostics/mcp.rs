//! Desktop MCP probes use the host's authorized diagnostic jobs, never production mounts.
use crate::{Notice, error as notify_error};
use super::{mcp_state::*, mcp_transport::ProbeTransport};
use vw_api_mcp_management::McpCatalogResponse;
type Message = McpDiagnosticMessage;
fn message(event: Message) -> Message { event }
use iced::Task;
use {
    vw_api_app_diagnostics::*,
    vw_api_mcp_management::McpRecordId,
};

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
fn poll(state: &mut McpState, context: Context, cancel: bool) -> Task<Message> {
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
                    gateway_mcp_probe::GatewayMcpProbeClient::new(transport.client.transport().clone())
                    .mcp_management_check_cancel(&context.query, &job.task_id, &transport.token)
                        .await
                        .map_err(safe_error)
                } else {
                    crate::diagnostics::sleep::sleep(std::time::Duration::from_millis(350)).await;
                    gateway_mcp_probe::GatewayMcpProbeClient::new(transport.client.transport().clone())
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
pub fn update(
    state: &mut McpState,
    current: Option<Context>,
    catalog: Option<&Result<McpCatalogResponse, String>>,
    token: &str,
    client: Option<Result<vw_gateway_client::GatewayClient, String>>,
    notice: &mut Option<Notice>,
    event: McpDiagnosticMessage,
) -> Task<Message> {
    match event {
        McpDiagnosticMessage::Select(choice) => {
            if !state.active()
                || current.as_ref() != state.context.as_ref()
            {
                state.app = Some(choice);
            }
        }
        McpDiagnosticMessage::Start(ids) => {
            let Some(context) = current.clone() else {
                return notify_error(notice, "请先选择项目、连接目标主机并选择检测应用");
            };
            if state.active() && state.context.as_ref() == Some(&context) {
                return Task::none();
            }
            let Some(Ok(catalog)) = catalog else {
                return notify_error(notice, "请先刷新受管 MCP 目录");
            };
            let targets: Vec<_> = catalog
                .records
                .iter()
                .filter(|r| ids.contains(&r.record.id))
                .map(|r| r.record.clone())
                .collect();
            if ids.is_empty() || targets.len() != ids.len() {
                return notify_error(notice, "检测目标无效，请刷新目录");
            }
            let client = match client.expect("start diagnostic client") {
                Ok(c) => c,
                Err(e) => return notify_error(notice, e),
            };
            let cleanup = if state.context.as_ref() != Some(&context) {
                invalidate(state)
            } else {
                Task::none()
            };
            state.begin(context.clone(), targets);
            let generation = state.generation;
            let transport = ProbeTransport { client, token: token.to_owned() };
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
                return notify_error(notice, error);
            }
        }
        McpDiagnosticMessage::Started(request, generation, transport, result) => {
            let stale = current.as_ref() != Some(&request)
                || state.context.as_ref() != Some(&request)
                || state.generation != generation
                || state.terminal;
            if stale {
                if let Ok(job) = result
                    && valid_identity(&request, &job)
                {
                    return cleanup(transport, request, job);
                }
                return Task::none();
            }
            if state.job.is_some() {
                return Task::none();
            }
            match result {
                Ok(job) if valid_identity(&request, &job) => {
                    state.job = Some(job);
                    let cancel = state.cancelling;
                    return poll(state, request, cancel);
                }
                _ => {
                    state.terminal = true;
                    state.transport = None;
                    state.targets.iter_mut().for_each(|(_, p)| *p = Phase::Unverified);
                    state.error = Some(safe_error(()));
                    return notify_error(notice, safe_error(()));
                }
            }
        }
        McpDiagnosticMessage::Cancel | McpDiagnosticMessage::Refresh => {
            let Some(context) = current.clone() else {
                return Task::none();
            };
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
            return poll(state, context, cancel);
        }
        McpDiagnosticMessage::Snapshot(request, generation, serial, result) => {
            let Some(current) = current.clone() else {
                return Task::none();
            };
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
                        return notify_error(notice, safe_error(()));
                    }
                    if !state.terminal {
                        return poll(state, request, false);
                    }
                    state.transport = None;
                }
                Err(error) => {
                    state.cancelling = false;
                    state.error = Some(error.clone());
                    return notify_error(notice, error);
                }
            }
        }
    }
    Task::none()
}

fn cleanup(transport: ProbeTransport, context: Context, job: DiagnosticJobIdentity) -> Task<Message> {
    super::mcp_cleanup::cleanup(transport, context, job).map(McpDiagnosticMessage::Cleaned)
}
fn invalidate(state: &mut McpState) -> Task<Message> {
    let active = state.active();
    let context = state.context.take();
    let job = state.job.take();
    let transport = state.transport.take();
    state.terminal = true;
    state.targets.clear();
    state.error = None;
    if active && let (Some(context), Some(job), Some(transport)) = (context, job, transport) {
        return cleanup(transport, context, job);
    }
    Task::none()
}
