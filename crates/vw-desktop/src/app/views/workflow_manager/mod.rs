//! Desktop lifecycle manager wiring.
use crate::app::{App, Message};
use crate::app::components::system_settings::task_settings::project_path;
use crate::app::message::task_board::{
    TaskBoardMessage, next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
};
use crate::app::state::ToastKind;
use desktop_task_model::workflow::TaskPoolWorkflowSnapshotDto;
use desktop_workflow_ui::workflow_manager as domain;
pub use domain::{WorkflowManagerMessage, WorkflowManagerState, WorkflowManagerTab, WorkflowDraftStatus, WorkflowManagerFeedback, LifecycleWorkflowListResult, update};
use iced::{Element, Task};
use vw_api_agent_scheduler::AgentSchedulerSelectWorkflowRequest;
fn client() -> domain::ClientResult {
    Ok(gateway_task_lifecycle::GatewayTaskLifecycleClient::new(crate::app::config::gateway_client()?.transport().clone()))
}
fn map_message(message: domain::Message) -> Message {
    match message {
        domain::Message::WorkflowManager(message) => Message::WorkflowManager(message),
        domain::Message::CopyCode(text) => Message::CopyCode(text),
        domain::Message::AppendText(text) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text)),
        domain::Message::OpenUrl(url) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url)),
        domain::Message::None => Message::None,
    }
}
pub(crate) fn preload(app: &mut App) -> Task<Message> {
    if app.workflow_manager.begin_lifecycle_refresh() {
        domain::load_lifecycle_workflows_task(false, client()).map(map_message)
    } else {
        Task::none()
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(
        &app.workflow_manager,
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        &app.appearance.app_theme,
        project_path(app),
        app.clipboard_state.last_copied_code_hash,
    ).map(map_message)
}

/// 读取与项目编辑器、看板共用的调度工作流快照。
pub(crate) async fn load_project_workflow(path: &str) -> domain::ProjectWorkflowResult {
    let scope = resolve_task_board_scheduler_scope(path).await?;
    let client = crate::app::gateway_client()?;
    let response = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
        .agent_scheduler_workflow(&scope).await?;
    if &response.project_id != scope.project_scope().project_id() {
        return Err("网关返回了其他项目的工作流".into());
    }
    Ok(response.workflow)
}

/// 提交时携带读取到的快照，复用网关的影响检查与并发保护。
pub(crate) async fn select_project_workflow(
    path: &str,
    request: AgentSchedulerSelectWorkflowRequest,
) -> Result<TaskPoolWorkflowSnapshotDto, String> {
    let scope = resolve_task_board_scheduler_scope(path).await?;
    let client = crate::app::gateway_client()?;
    let response = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
        .agent_scheduler_select_workflow(&scope, &request).await?;
    if &response.project_id != scope.project_scope().project_id() {
        return Err("网关返回了其他项目的工作流".into());
    }
    response.workflow
        .filter(|workflow| workflow.id == request.workflow_id)
        .ok_or_else(|| "网关未确认所选项目工作流".into())
}

fn load_project_workflow_task(app: &mut App) -> Task<Message> {
    let Some(path) = project_path(app).map(str::to_owned) else {
        return Task::none();
    };
    let Some(generation) = app.workflow_manager.begin_project_workflow_load(&path) else {
        return Task::none();
    };
    let request_path = path.clone();
    Task::perform(async move { load_project_workflow(&request_path).await }, move |result| {
        Message::WorkflowManager(WorkflowManagerMessage::ProjectWorkflowLoaded {
            path: path.clone(), generation, result,
        })
    })
}

/// 更新应用中的工作流状态、刷新运行时缓存并派发网关副作用。
pub fn update_app(app: &mut App, message: WorkflowManagerMessage) -> Task<Message> {
    let project_task = match message {
        WorkflowManagerMessage::RefreshLifecycles => load_project_workflow_task(app),
        WorkflowManagerMessage::ApplyProjectWorkflow { path, workflow_id } => {
            if project_path(app) != Some(path.as_str()) {
                update(&mut app.workflow_manager, &mut domain::registry_from_runtime(),
                    WorkflowManagerMessage::CancelPendingConfirmation);
                return Task::none();
            }
            let Some((generation, expected_workflow)) = app.workflow_manager
                .begin_project_workflow_apply(&path, &workflow_id)
            else {
                return Task::none();
            };
            let request = AgentSchedulerSelectWorkflowRequest { workflow_id, expected_workflow };
            let request_path = path.clone();
            return Task::perform(
                async move { select_project_workflow(&request_path, request).await.map(Some) },
                move |result| Message::WorkflowManager(WorkflowManagerMessage::ProjectWorkflowApplied {
                    path: path.clone(), generation, result,
                }),
            );
        }
        WorkflowManagerMessage::ProjectWorkflowLoaded { path, generation, result } => {
            app.workflow_manager.finish_project_workflow(&path, generation, result, false);
            return Task::none();
        }
        WorkflowManagerMessage::ProjectWorkflowApplied { path, generation, result } => {
            let current = app.workflow_manager.finish_project_workflow(
                &path, generation, result.clone(), true,
            );
            return match result {
                Ok(snapshot) => {
                    let refresh = if app.navigation_tabs.project_path.as_deref() == Some(path.as_str()) {
                        app.task_board.task_board_workflow_generation =
                            next_agent_scheduler_status_generation();
                        if current {
                            app.task_board.task_board_settings.apply_workflow_snapshot(snapshot);
                        }
                        Task::done(Message::TaskBoard(TaskBoardMessage::LoadTasks))
                    } else {
                        Task::none()
                    };
                    if current && project_path(app) == Some(path.as_str()) {
                        Task::batch([
                            refresh,
                            app.show_localized_toast(ToastKind::Success, "项目工作流已应用", vec![]),
                        ])
                    } else {
                        refresh
                    }
                }
                Err(error) if current && project_path(app) == Some(path.as_str()) => app.show_localized_toast(
                    ToastKind::Error, "应用项目工作流失败：%s", vec![error.into()],
                ),
                Err(_) => Task::none(),
            };
        }
        _ => Task::none(),
    };
    let mut registry = domain::registry_from_runtime();
    let _changed = update(&mut app.workflow_manager, &mut registry, message);
    if let Some(effect) = app.workflow_manager.take_pending_effect() {
        return Task::batch([
            project_task,
            domain::workflow_manager_effect_task(effect, client()).map(map_message),
        ]);
    }
    let feedback_task = if let Some(feedback) = app.workflow_manager.feedback().cloned() {
        if let Some(message) = feedback.localized.clone() {
            let kind = if feedback.is_error() {
                crate::app::state::ToastKind::Error
            } else {
                crate::app::state::ToastKind::Success
            };
            app.show_localized_toast(
                kind,
                &message.source,
                message.arguments.into_iter().map(Into::into).collect(),
            )
        } else if feedback.is_error() {
            app.show_error_toast(feedback.message().to_string())
        } else if feedback.is_success() {
            app.show_success_toast(feedback.message().to_string())
        } else {
            Task::none()
        }
    } else {
        Task::none()
    };
    Task::batch([project_task, feedback_task])
}
