//! SDK-only lifecycle interaction. Recover reads; never replay an uncertain submission.
use super::{AppsCatalogMessage, SettingsMessage};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::{
    AgentAppsLifecycleError, GatewayClient,
    vw_api_types::{agent_apps::*, task::TaskStatus},
};

#[derive(Clone, Debug)]
pub enum LifecycleMessage {
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
    Read(vw_gateway_client::vw_api_types::id::TaskId),
    Poll(vw_gateway_client::vw_api_types::id::TaskId),
    Cancel(vw_gateway_client::vw_api_types::id::TaskId),
}
fn context(client: &GatewayClient, app: &App) -> AgentAppsContext {
    AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    }
}
pub(crate) async fn execute(
    client: GatewayClient,
    host: String,
    token: String,
    work: Work,
) -> Result<LifecycleResult, LifecycleFailure> {
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
pub(super) fn update(app: &mut App, message: LifecycleMessage) -> Task<Message> {
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(_) => return app.show_error_toast("网关不可用"),
    };
    let current = context(&client, app);
    if app.apps_catalog.reset_if_context_changed(&current) {
        return Task::none();
    }
    let host = app.apps_catalog.host.clone();
    if host.is_empty() || app.apps_catalog.token.is_empty() {
        return Task::none();
    }
    let target_label = if let LifecycleMessage::Preview(request) = &message {
        match &request.target {
            AppActionTarget::Environment { environment } => {
                if request.action == AppLifecycleAction::Uninstall {
                    return app.show_error_toast("卸载必须选择具体实例");
                }
                format!("新安装环境：{environment:?}")
            }
            AppActionTarget::Installation { installation_id } => {
                let Some(installation) = app
                    .apps_catalog
                    .rows
                    .iter()
                    .filter(|r| r.entry.id == request.app_id)
                    .filter_map(|r| r.observation.as_ref().ok())
                    .flat_map(|o| &o.installations)
                    .find(|i| &i.id == installation_id)
                else {
                    return app.show_error_toast("目标实例已失效，请刷新列表");
                };
                if request.action == AppLifecycleAction::Uninstall
                    && !crate::app::components::system_settings_apps::uninstall::selectable(
                        request,
                        installation,
                    )
                {
                    return app.show_error_toast(
                        crate::app::components::system_settings_apps::uninstall::MANUAL_GUIDANCE,
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
    let state = &mut app.apps_catalog.lifecycle;
    let work = match message {
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
                    } else {
                        state.preview = Some(preview);
                    }
                    return state
                        .error
                        .clone()
                        .map_or_else(Task::none, |error| app.show_error_toast(error));
                }
                Ok(LifecycleResult::Status(status)) => {
                    if !state.accept_status(status) {
                        return app.show_error_toast("操作响应不匹配，拒绝覆盖当前状态");
                    }
                    state.error = None;
                    let operation = state.operation.as_ref().unwrap();
                    if operation.phase == AppOperationPhase::Terminal {
                        return if operation.status == TaskStatus::Failed {
                            app.show_error_toast("应用操作失败，请刷新应用诊断")
                        } else {
                            Task::none()
                        };
                    }
                    Work::Poll(operation.operation_id.clone())
                }
                Err(error) => {
                    state.uncertain |= error.uncertain;
                    state.error = Some(error.label.clone());
                    return app.show_error_toast(error.label);
                }
            }
        }
    };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.busy = true;
    let token = app.apps_catalog.token.clone();
    Task::perform(execute(client, host.clone(), token, work), move |result| {
        Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Lifecycle(
            LifecycleMessage::Finished {
                context: current.clone(),
                host: host.clone(),
                generation,
                result,
            },
        )))
    })
}
