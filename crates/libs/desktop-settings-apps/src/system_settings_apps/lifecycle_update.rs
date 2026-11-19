use vw_api_app_operations::{AppActionPreview, AppActionPreviewRequest, AppActionSubmitRequest, AppActionTarget, AppLifecycleAction, AppOperationPhase, AppOperationStatus};
use vw_api_app_identity::{AppInstallationSource};
// SDK-only lifecycle interaction. Recover reads; never replay an uncertain submission.
use desktop_provider_apps::AgentAppsContext;
use super::{catalog::CatalogRow, lifecycle::LifecycleState};
use crate::aggregation::{Notice, error as notify_error};
use iced::Task;
use gateway_app_lifecycle::{AgentAppsLifecycleError, GatewayAppLifecycleClient};
use vw_gateway_client::GatewayClient;
use vw_api_task::TaskStatus;

#[derive(Clone, Debug)]
pub enum LifecycleMessage {
    Batch(Vec<vw_api_app_identity::AppInstallation>),
    BatchNext,
    Preview(AppActionPreviewRequest),
    Confirm,
    Close,
    Refresh,
    Cancel,
    Finished {
        context: AgentAppsContext,
        host: String,
        generation: u64,
        result: Result<LifecycleResult, LifecycleFailure>,
    },
}
#[derive(Clone, Debug)]
pub enum LifecycleResult {
    Preview(AppActionPreview),
    Status(AppOperationStatus),
}
#[derive(Clone, Debug)]
pub struct LifecycleFailure {
    pub uncertain: bool,
    pub label: String,
}
#[derive(Clone)]
pub(crate) enum Work {
    Preview(AppActionPreviewRequest),
    Submit(AppActionSubmitRequest),
    Read(vw_api_transport::TaskId),
    Poll(vw_api_transport::TaskId),
    Cancel(vw_api_transport::TaskId),
}
pub(crate) async fn execute(
    client: GatewayClient,
    host: String,
    token: String,
    work: Work,
) -> Result<LifecycleResult, LifecycleFailure> {
    let client = GatewayAppLifecycleClient::new(client.transport().clone());
    let submitting = matches!(work, Work::Submit(_));
    let result = match work {
        Work::Preview(request) => client
            .agent_apps_action_preview_authorized(&host, &request, &token)
            .await
            .map(LifecycleResult::Preview),
        Work::Submit(request) => {
            client.agent_apps_action_submit_authorized(&host, &request, &token).await.map(|r| {
                LifecycleResult::Status(AppOperationStatus {
                    operation_id: r.operation_id,
                    status: r.status,
                    phase: AppOperationPhase::Queued,
                })
            })
        }
        Work::Read(id) => client
            .agent_apps_operation_authorized(&host, &id, &token)
            .await
            .map(LifecycleResult::Status),
        Work::Poll(id) => client
            .agent_apps_operation_events_authorized(&host, &id, &token)
            .await
            .map(LifecycleResult::Status),
        Work::Cancel(id) => client
            .agent_apps_operation_cancel_authorized(&host, &id, &token)
            .await
            .map(LifecycleResult::Status),
    };
    result.map_err(|error| LifecycleFailure {
        uncertain: submitting
            && !matches!(error, AgentAppsLifecycleError::Http { status: 400..=499, .. }),
        label: match error {
            AgentAppsLifecycleError::Http { status: 422, .. } => {
                "预检拒绝：请检查目标、安装来源及 Node 依赖；未开始执行".into()
            }
            AgentAppsLifecycleError::Http { status: 401 | 403, .. } => {
                "应用管理授权不足，请联系管理员".into()
            }
            AgentAppsLifecycleError::Http { status: 404 | 409, .. } => {
                "预览或操作已失效，请重新预检并确认；不会自动重试删除".into()
            }
            _ if submitting => "提交结果未知，请联系管理员核对；禁止重复提交".into(),
            _ => "连接中断或服务不可用，可刷新恢复同一操作".into(),
        },
    })
}
pub fn update(
    state: &mut LifecycleState,
    rows: &[CatalogRow],
    client: GatewayClient,
    current: AgentAppsContext,
    host: String,
    token: String,
    message: LifecycleMessage,
    notice: &mut Option<Notice>,
    invalidate: &mut bool,
) -> Task<LifecycleMessage> {
    let mut message = message;
    if let LifecycleMessage::Batch(items) = &message {
        if items.is_empty() || state.active() || state.preview.is_some()
            || state.batch.as_ref().is_some_and(|b| !b.stopped) { return Task::none(); }
        state.invalidate();
        state.batch = Some(super::lifecycle::BatchUpgrade { items: items.clone(), ..Default::default() });
        return Task::none();
    }
    if matches!(message, LifecycleMessage::Confirm)
        && state.batch.as_ref().is_some_and(|b| !b.confirmed && !b.stopped) {
        state.batch.as_mut().unwrap().confirmed = true;
        message = LifecycleMessage::BatchNext;
    }
    let internal_preview = matches!(message, LifecycleMessage::BatchNext);
    if internal_preview {
        let Some(batch) = &state.batch else { return Task::none(); };
        if batch.stopped || !batch.confirmed || state.active() { return Task::none(); }
        let Some(item) = batch.items.get(batch.completed) else { return Task::none(); };
        if !rows.iter().any(|row| super::presentation::preferred(row) == Some(item)) {
            state.batch.as_mut().unwrap().stopped = true;
            return notify_error(notice, "批量升级目标已变化，已停止，请重新检查更新");
        }
        message = LifecycleMessage::Preview(AppActionPreviewRequest {
            app_id: item.app_id, action: AppLifecycleAction::Update,
            target: AppActionTarget::Installation { installation_id: item.id.clone() },
        });
        state.operation = None;
        state.preview = None;
    }
    if matches!(message, LifecycleMessage::Preview(_)) && !internal_preview
        && state.batch.as_ref().is_some_and(|b| !b.stopped) { return Task::none(); }
    if matches!(message, LifecycleMessage::Close | LifecycleMessage::Cancel) {
        if let Some(batch) = &mut state.batch { batch.stopped = true; }
    }
    if matches!(message, LifecycleMessage::Preview(_)) && !internal_preview
        && !state.active() && state.preview.is_none() {
        state.batch = None;
    }
    let target_label = if let LifecycleMessage::Preview(request) = &message {
        match &request.target {
            AppActionTarget::Environment { environment } => {
                if request.action == AppLifecycleAction::Uninstall {
                    return notify_error(notice, "卸载必须选择具体实例");
                }
                format!("新安装环境：{environment:?}")
            }
            AppActionTarget::Installation { installation_id } => {
                let Some(installation) = rows
                    .iter()
                    .filter(|r| r.entry.id == request.app_id)
                    .filter_map(|r| r.observation.as_ref().ok())
                    .flat_map(|o| &o.installations)
                    .find(|i| &i.id == installation_id)
                else {
                    return notify_error(notice, "目标实例已失效，请刷新列表");
                };
                if request.action == AppLifecycleAction::Uninstall
                    && !super::uninstall::selectable(
                        request,
                        installation,
                    )
                {
                    return notify_error(notice,
                        super::uninstall::MANUAL_GUIDANCE,
                    );
                }
                format!(
                    "实例路径：{}\n真实路径：{}\n观察来源：{:?} · {:?}",
                    installation.path,
                    installation.real_path.as_deref().unwrap_or("未确认"),
                    installation.source,
                    installation.environment
                )
            }
        }
    } else {
        String::new()
    };
    let work = match message {
        LifecycleMessage::Batch(_) | LifecycleMessage::BatchNext => return Task::none(),
        LifecycleMessage::Preview(request) => {
            if state.active() || state.preview.is_some() {
                return Task::none();
            }
            state.target_label = target_label;
            state.request = Some(request.clone());
            state.operation = None;
            state.error = None;
            Work::Preview(request)
        }
        LifecycleMessage::Confirm => {
            if state.busy || state.uncertain || state.operation.is_some() {
                return Task::none();
            }
            if let Some(batch) = &mut state.batch {
                if batch.stopped { return Task::none(); }
                if !batch.items.get(batch.completed).is_some_and(|item|
                    rows.iter().any(|row| super::presentation::preferred(row) == Some(item))) {
                    batch.stopped = true;
                    return notify_error(notice, "批量升级目标已变化，拒绝提交");
                }
            }
            let Some(preview) = state.preview.take() else {
                return Task::none();
            };
            Work::Submit(AppActionSubmitRequest { preview_id: preview.preview_id, confirmed: true })
        }
        LifecycleMessage::Close => {
            if !state.active() {
                state.preview = None;
                state.request = None;
                state.error = None;
            }
            return Task::none();
        }
        LifecycleMessage::Refresh | LifecycleMessage::Cancel => {
            let Some(operation) = &state.operation else {
                return Task::none();
            };
            if operation.phase == AppOperationPhase::Terminal {
                return Task::none();
            }
            let id = operation.operation_id.clone();
            if matches!(message, LifecycleMessage::Cancel) {
                if state.cancel_requested {
                    return Task::none();
                }
                state.cancel_requested = true;
                Work::Cancel(id)
            } else {
                Work::Read(id)
            }
        }
        LifecycleMessage::Finished { context, host: requested_host, generation, result } => {
            if context != current || requested_host != host || generation != state.generation {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(LifecycleResult::Preview(preview)) => {
                    if state.request.as_ref() != Some(&preview.request)
                        || preview.source == AppInstallationSource::Unknown
                    {
                        state.error = Some("预检目标或来源不匹配，拒绝提交".into());
                        if let Some(batch) = &mut state.batch { batch.stopped = true; batch.failed += 1; }
                    } else {
                        if let Some(batch) = &mut state.batch {
                            if batch.stopped { return Task::none(); }
                            if !batch.items.get(batch.completed).is_some_and(|item| item.source == preview.source) {
                                batch.stopped = true;
                                batch.failed += 1;
                                return notify_error(notice, "批量升级预检来源已变化，已停止");
                            }
                        }
                        state.preview = Some(preview);
                        if state.batch.as_ref().is_some_and(|b| b.confirmed && !b.stopped) {
                            return update(state, rows, client, current, host, token, LifecycleMessage::Confirm, notice, invalidate);
                        }
                    }
                    return state
                        .error
                        .clone()
                        .map_or_else(Task::none, |error| notify_error(notice, error));
                }
                Ok(LifecycleResult::Status(status)) => {
                    if !state.accept_status(status) {
                        state.cancel_requested = false;
                        if let Some(batch) = &mut state.batch { batch.stopped = true; }
                        return notify_error(notice, "操作响应不匹配，拒绝覆盖当前状态");
                    }
                    state.error = None;
                    let operation = state.operation.as_ref().unwrap();
                    if operation.phase == AppOperationPhase::Terminal {
                        *invalidate = true;
                        if let Some(batch) = &mut state.batch {
                            match operation.status {
                                TaskStatus::Completed => batch.completed += 1,
                                TaskStatus::Failed => { batch.failed += 1; batch.stopped = true; }
                                TaskStatus::Cancelled => { batch.cancelled += 1; batch.stopped = true; }
                                _ => batch.stopped = true,
                            }
                            if batch.completed == batch.items.len() { batch.stopped = true; }
                            if !batch.stopped {
                                return update(state, rows, client, current, host, token, LifecycleMessage::BatchNext, notice, invalidate);
                            }
                        }
                        return if operation.status == TaskStatus::Failed {
                            notify_error(notice, "应用操作失败，请刷新应用诊断")
                        } else {
                            Task::none()
                        };
                    }
                    Work::Poll(operation.operation_id.clone())
                }
                Err(error) => {
                    state.cancel_requested = false;
                    state.uncertain |= error.uncertain;
                    if let Some(batch) = &mut state.batch { batch.stopped = true; }
                    state.error = Some(error.label.clone());
                    return notify_error(notice, error.label);
                }
            }
        }
    };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.busy = true;
    Task::perform(execute(client, host.clone(), token, work), move |result| {
        LifecycleMessage::Finished {
                context: current.clone(),
                host: host.clone(),
                generation,
                result,
            }
    })
}
