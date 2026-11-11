//! Reconciliation jobs keep their gateway identity and never replay an uncertain submission.
use super::{
    McpMessage, SettingsMessage, mcp_management_bindings as bindings,
    mcp_management_import as import, mcp_management_list as catalog,
};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    AgentAppsSettingsError, GatewayClient,
    mcp_management_distribution::*,
    vw_api_types::{
        TaskId,
        agent_apps::{AppActionExecutionResult, AppId, AppOperationPhase},
        mcp_management::*,
        task::TaskStatus,
    },
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ResyncState {
    pub(crate) open: bool,
    pub(crate) context: Option<catalog::Context>,
    pub(crate) generation: u64,
    pub(crate) selected: Vec<McpAppTarget>,
    pub(crate) account: String,
    pub(crate) request: Option<McpResyncRequest>,
    pub(crate) operation: Option<TaskId>,
    pub(crate) status: Option<McpResyncOperationStatus>,
    pub(crate) busy: bool,
    pub(crate) uncertain: bool,
    pub(crate) cancelling: bool,
    pub(crate) error: Option<String>,
    pub(crate) observed: Vec<(McpAppTarget, Option<McpNativeStatus>)>,
}
#[derive(Debug, Clone)]
pub enum ResyncMessage {
    Open,
    Close,
    Select(McpAppTarget, bool),
    Account(String),
    Start,
    RetryFailed,
    Refresh,
    Cancel,
    Finished {
        context: catalog::Context,
        generation: u64,
        result: Result<ResyncResult, ResyncFailure>,
    },
}
#[derive(Debug, Clone)]
pub struct ResyncFailure {
    pub label: String,
    pub uncertain: bool,
}
#[derive(Debug, Clone)]
pub enum ResyncResult {
    Accepted(AppActionExecutionResult),
    Status(McpResyncOperationStatus),
    Observed(Vec<(McpAppTarget, Option<McpNativeStatus>)>),
}
#[derive(Clone)]
pub(crate) enum Work {
    Submit(McpResyncRequest),
    Read(TaskId),
    Cancel(TaskId),
    Observe(McpResyncRequest),
}
pub(crate) fn message(value: ResyncMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Resync(value)))
}
pub(crate) fn terminal(status: &McpResyncOperationStatus) -> bool {
    status.operation.phase == AppOperationPhase::Terminal
}
pub(crate) fn failed(target: &McpResyncTargetResult) -> bool {
    target.code.is_some()
        || target.distribution.as_ref().is_none_or(|d| {
            matches!(d.outcome, McpBatchOutcome::Failed | McpBatchOutcome::PartiallySucceeded)
                || d.items
                    .iter()
                    .any(|i| matches!(i.native_outcome, McpDistributionOutcome::Failed { .. }))
        })
}
pub(crate) fn successful(status: &McpResyncOperationStatus) -> bool {
    status.outcome == Some(McpBatchOutcome::Succeeded)
        && status.targets.iter().all(|target| {
            target.code.is_none()
                && target.distribution.as_ref().is_some_and(|d| {
                    d.outcome == McpBatchOutcome::Succeeded
                        && d.items.iter().all(|item| {
                            item.desired_committed
                                && matches!(
                                    item.native_outcome,
                                    McpDistributionOutcome::Applied
                                        | McpDistributionOutcome::Unchanged
                                )
                        })
                })
        })
}
fn validate(
    status: &McpResyncOperationStatus,
    request: &McpResyncRequest,
    id: &TaskId,
) -> Result<(), String> {
    let invalid = || "作业响应不完整或目标不匹配；请刷新同一作业".to_owned();
    if &status.operation.operation_id != id
        || (terminal(status)
            && (status.outcome.is_none() || status.targets.len() != request.targets.len()))
    {
        return Err(invalid());
    }
    if terminal(status) {
        let consistent = matches!(
            (&status.operation.status, status.outcome),
            (TaskStatus::Completed, Some(McpBatchOutcome::Succeeded))
                | (TaskStatus::Cancelled, Some(McpBatchOutcome::Cancelled))
                | (
                    TaskStatus::Failed,
                    Some(McpBatchOutcome::Failed | McpBatchOutcome::PartiallySucceeded)
                )
        );
        if !consistent {
            return Err(invalid());
        }
    }
    let mut seen = Vec::new();
    for target in &status.targets {
        let Some(expected) = request.targets.iter().find(|t| t.app_id == target.app_id) else {
            return Err(invalid());
        };
        if seen.contains(&target.app_id) || target.distribution.is_some() == target.code.is_some() {
            return Err(invalid());
        }
        seen.push(target.app_id);
        if let Some(d) = &target.distribution {
            let mut records = Vec::new();
            for item in &d.items {
                if item.target.app_id != expected.app_id
                    || item.target.installation_id != expected.installation_id
                    || item.target.target_host_id != request.target_host_id
                    || records.contains(&item.record.id)
                {
                    return Err(invalid());
                }
                records.push(item.record.id.clone());
            }
        }
    }
    Ok(())
}
/// Refresh actual catalog and native observations before submitting (including failed-only retry).
/// All paths, credentials and native writes remain on the execution host.
pub(crate) async fn execute(
    client: GatewayClient,
    host: String,
    token: String,
    work: Work,
) -> Result<ResyncResult, ResyncFailure> {
    match work {
        Work::Observe(request) => {
            let mut observed = Vec::new();
            for target in request.targets {
                let expected = McpAppTarget {
                    app_id: target.app_id,
                    installation_id: target.installation_id,
                    target_host_id: host.clone(),
                };
                let status = client
                    .mcp_management_native(&expected, target.account_id.as_deref(), &token)
                    .await
                    .ok()
                    .filter(|p| p.target == expected)
                    .map(|p| p.status);
                observed.push((expected, status));
            }
            Ok(ResyncResult::Observed(observed))
        }
        Work::Submit(request) => {
            client.mcp_management_catalog(&host, &token).await.map_err(|_| ResyncFailure {
                label: "无法刷新实际目录；未提交重同步".into(),
                uncertain: false,
            })?;
            for target in &request.targets {
                let expected = McpAppTarget {
                    app_id: target.app_id,
                    installation_id: target.installation_id.clone(),
                    target_host_id: host.clone(),
                };
                let preview = client
                    .mcp_management_native(&expected, target.account_id.as_deref(), &token)
                    .await
                    .map_err(|_| ResyncFailure {
                        label: "无法刷新目标原生状态；未提交重同步".into(),
                        uncertain: false,
                    })?;
                if preview.target != expected {
                    return Err(ResyncFailure {
                        label: "目标状态不匹配；未提交重同步".into(),
                        uncertain: false,
                    });
                }
            }
            client
                .mcp_management_resync(&request, &token)
                .await
                .map(ResyncResult::Accepted)
                .map_err(|error| match error {
                    AgentAppsSettingsError::Http { status: 400..=499, .. } => ResyncFailure {
                        label: "重同步被拒绝；请刷新目标与授权后重试".into(),
                        uncertain: false,
                    },
                    _ => ResyncFailure {
                        label: "提交结果未知；禁止重复提交，请核对执行端作业".into(),
                        uncertain: true,
                    },
                })
        }
        Work::Read(id) => {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            client
                .mcp_management_resync_status(&host, &id, &token)
                .await
                .map(ResyncResult::Status)
                .map_err(|_| ResyncFailure {
                    label: "读取失败；可刷新恢复同一作业".into(),
                    uncertain: false,
                })
        }
        Work::Cancel(id) => client
            .mcp_management_resync_cancel(&host, &id, &token)
            .await
            .map(ResyncResult::Status)
            .map_err(|_| ResyncFailure {
                label: "取消结果未知；请刷新同一作业核实".into(),
                uncertain: false,
            }),
    }
}
fn dispatch(app: &mut App, context: catalog::Context, work: Work) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用");
    };
    let state = &mut app.mcp_settings.resync;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.busy = true;
    state.error = None;
    let token = app.apps_catalog.token.clone();
    Task::perform(
        async move {
            let result = execute(client, context.host.clone(), token, work).await;
            ResyncMessage::Finished { context, generation, result }
        },
        message,
    )
}
pub(crate) fn visible(app: &App) -> Option<&ResyncState> {
    (catalog::context(app).as_ref() == app.mcp_settings.resync.context.as_ref())
        .then_some(&app.mcp_settings.resync)
}
pub(crate) fn update(app: &mut App, value: ResyncMessage) -> Task<Message> {
    let Some(context) = catalog::context(app) else {
        return Task::none();
    };
    if app.mcp_settings.resync.context.as_ref() != Some(&context) {
        let generation = app.mcp_settings.resync.generation.wrapping_add(1);
        app.mcp_settings.resync =
            ResyncState { context: Some(context.clone()), generation, ..Default::default() };
    }
    if app.apps_catalog.context.as_ref() != Some(&context.endpoint)
        || app.apps_catalog.token.is_empty()
        || context.host.is_empty()
    {
        return Task::none();
    }
    match value {
        ResyncMessage::Open => {
            if !locked(&app.mcp_settings.resync)
                && app.mcp_settings.import.is_none()
                && app.mcp_settings.connection_form.is_none()
            {
                app.mcp_settings.resync.open = true;
            }
        }
        ResyncMessage::Close => {
            if !locked(&app.mcp_settings.resync) {
                app.mcp_settings.resync.open = false;
            }
        }
        ResyncMessage::Select(target, checked) => {
            if locked(&app.mcp_settings.resync) || !import::targets(app).contains(&target) {
                return Task::none();
            }
            let selected = &mut app.mcp_settings.resync.selected;
            selected.retain(|t| t.app_id != target.app_id);
            if checked {
                selected.push(target);
            }
        }
        ResyncMessage::Account(account) => {
            if !locked(&app.mcp_settings.resync) {
                app.mcp_settings.resync.account = account;
            }
        }
        ResyncMessage::Start | ResyncMessage::RetryFailed => {
            let state = &app.mcp_settings.resync;
            if locked(state) {
                return Task::none();
            }
            let request = if matches!(value, ResyncMessage::RetryFailed) {
                let (Some(old), Some(status)) = (&state.request, &state.status) else {
                    return Task::none();
                };
                if !terminal(status) {
                    return Task::none();
                }
                let mut request = old.clone();
                request
                    .targets
                    .retain(|t| status.targets.iter().any(|r| r.app_id == t.app_id && failed(r)));
                request
            } else {
                let project_id = if app.mcp_settings.bindings.selected.is_empty() {
                    None
                } else {
                    if !matches!(catalog::visible(app), Some(Ok(c)) if app.mcp_settings.bindings.selected.iter().all(|r| c.records.contains(r)))
                    {
                        return app.show_error_toast("所选目录已失效，请刷新后重选");
                    }
                    match bindings::project(&app.mcp_settings.bindings.selected) {
                        Ok(project) => project,
                        Err(error) => return app.show_error_toast(error),
                    }
                };
                McpResyncRequest {
                    target_host_id: context.host.clone(),
                    project_id,
                    targets: state
                        .selected
                        .iter()
                        .map(|t| McpResyncTarget {
                            app_id: t.app_id,
                            installation_id: t.installation_id.clone(),
                            account_id: (t.app_id == AppId::Claude)
                                .then(|| state.account.trim().to_owned()),
                        })
                        .collect(),
                }
            };
            let trusted = import::targets(app);
            if request.targets.is_empty()
                || request.targets.len() > 12
                || request.targets.iter().any(|t| {
                    !trusted
                        .iter()
                        .any(|i| i.app_id == t.app_id && i.installation_id == t.installation_id)
                        || t.account_id.as_ref().is_some_and(|id| id.is_empty())
                })
            {
                return app.show_error_toast("请选择可信应用实例；Claude 需要官方账号引用");
            }
            let state = &mut app.mcp_settings.resync;
            state.open = false;
            state.request = Some(request.clone());
            state.operation = None;
            state.status = None;
            state.observed.clear();
            state.uncertain = true;
            return dispatch(app, context, Work::Submit(request));
        }
        ResyncMessage::Refresh | ResyncMessage::Cancel => {
            let state = &app.mcp_settings.resync;
            if state.cancelling || (matches!(value, ResyncMessage::Refresh) && state.busy) {
                return Task::none();
            }
            let Some(id) = state.operation.clone() else {
                return Task::none();
            };
            if matches!(value, ResyncMessage::Cancel) && state.status.as_ref().is_some_and(terminal)
            {
                return Task::none();
            }
            app.mcp_settings.resync.cancelling = matches!(value, ResyncMessage::Cancel);
            return dispatch(
                app,
                context,
                if matches!(value, ResyncMessage::Cancel) {
                    Work::Cancel(id)
                } else {
                    Work::Read(id)
                },
            );
        }
        ResyncMessage::Finished { context: request_context, generation, result } => {
            let state = &mut app.mcp_settings.resync;
            if request_context != context || generation != state.generation || !state.busy {
                return Task::none();
            }
            state.busy = false;
            state.cancelling = false;
            match result {
                Err(error) => {
                    // Preflight errors have no ambiguous write; later reads keep the operation ID.
                    if state.operation.is_none() {
                        state.uncertain = error.uncertain;
                    }
                    state.error = Some(error.label.clone());
                    return Task::batch([
                        app.show_error_toast(error.label),
                        catalog::update(app, catalog::CatalogMessage::Refresh),
                    ]);
                }
                Ok(ResyncResult::Observed(observed)) => {
                    state.observed = observed;
                }
                Ok(ResyncResult::Accepted(accepted)) => {
                    if state.operation.is_some() {
                        return Task::none();
                    }
                    if accepted.operation_id.0.is_empty() {
                        state.error = Some("作业标识缺失；提交结果未知，禁止重复提交".into());
                        return app.show_error_toast("作业标识缺失；提交结果未知，禁止重复提交");
                    }
                    state.uncertain = false;
                    state.operation = Some(accepted.operation_id.clone());
                    return dispatch(app, context, Work::Read(accepted.operation_id));
                }
                Ok(ResyncResult::Status(status)) => {
                    let Some(id) = &state.operation else {
                        return Task::none();
                    };
                    let Some(request) = &state.request else {
                        return Task::none();
                    };
                    if let Err(error) = validate(&status, request, id) {
                        state.error = Some(error.clone());
                        return app.show_error_toast(error);
                    }
                    let done = terminal(&status);
                    let id = id.clone();
                    state.status = Some(status);
                    if !done {
                        return dispatch(app, context, Work::Read(id));
                    }
                    let successful = state.status.as_ref().is_some_and(successful);
                    let feedback = if successful {
                        app.show_success_toast("重同步已完成")
                    } else {
                        app.show_error_toast("重同步存在失败、取消或未验证目标，请查看逐目标结果")
                    };
                    let request = app.mcp_settings.resync.request.clone().expect("owned request");
                    let observe = dispatch(app, context, Work::Observe(request));
                    return Task::batch([
                        feedback,
                        catalog::update(app, catalog::CatalogMessage::Refresh),
                        observe,
                    ]);
                }
            }
        }
    }
    Task::none()
}
pub(crate) fn locked(state: &ResyncState) -> bool {
    state.busy
        || state.uncertain
        || (state.operation.is_some() && !state.status.as_ref().is_some_and(terminal))
}
