//! Execution decisions with explicit transport, timer and notification effects.
use desktop_task_model::{Task, TaskStatus, task_workflow_by_id};
use crate::message::TaskBoardMessage;
use vw_api_agent_scheduler::{AgentSchedulerCommandKindDto, AgentSchedulerDispatchModeDto, AgentSchedulerWakeResponse};
pub struct ExecutionState<'a> {
    pub project_path: &'a mut Option<String>,
    pub task_board_settings: &'a mut desktop_task_model::TaskBoardSettings,
    pub task_board_tasks: &'a mut Vec<Task>,
    pub task_board_executor: &'a mut desktop_task_executor::executor::state::TaskExecutorState,
    pub task_board_executor_running: &'a mut bool,
    pub task_board_last_log_flush_at_ms: &'a mut u64,
    pub task_board_log_scan_cursor: &'a mut usize,
    pub task_board_timeout_scan_cursor: &'a mut usize,
    pub task_board_schedule_scan_cursor: &'a mut usize,
    pub task_board_next_scheduler_tick_at_ms: &'a mut u64,
    pub next_deadline_ms: u64,
    pub effects: Vec<ExecutionEffect>,
}
pub enum WakeKind { Execution, Promote, Merge }
pub enum ExecutionEffect {
    Message(TaskBoardMessage), Error(String), SyncLogs(Option<Task>), Maintain(String),
    ScheduleTick, SchedulePromote, ScheduleMerge, SyncStatus(String), SyncInFlight(String, String), Wake(WakeKind), WakeCompleted(WakeKind),
    Command(String, Task, String, AgentSchedulerCommandKindDto, AgentSchedulerDispatchModeDto, Option<u32>, Option<String>),
}
fn batch(tasks: Vec<Vec<ExecutionEffect>>) -> Vec<ExecutionEffect> { tasks.into_iter().flatten().collect() }
fn done(message: TaskBoardMessage) -> Vec<ExecutionEffect> { vec![ExecutionEffect::Message(message)] }
fn error_toast(error: String) -> Vec<ExecutionEffect> { vec![ExecutionEffect::Error(error)] }
fn command(path: String, task: Task, workflow: String, kind: AgentSchedulerCommandKindDto, mode: AgentSchedulerDispatchModeDto, max: Option<u32>, reason: Option<String>) -> Vec<ExecutionEffect> {
    vec![ExecutionEffect::Command(path, task, workflow, kind, mode, max, reason)]
}
fn dispatch_wake(kind: WakeKind, _response: AgentSchedulerWakeResponse) -> Vec<ExecutionEffect> {
    vec![ExecutionEffect::WakeCompleted(kind)]
}
fn should_continue_existing_session_for_manual_retry(task: &Task, manual: bool) -> bool {
    manual && task.status == TaskStatus::Failed
}

fn add_agent_scheduler_command_log_once(task: &mut Task, message: &str) {
    if !task.logs.iter().rev().take(16).any(|entry| entry.message == message) {
        task.add_localized_log(message, vec![]);
    }
}

pub fn agent_scheduler_lifecycle_error(workflow_id: &str) -> Option<String> {
    task_workflow_by_id(workflow_id)
        .is_none()
        .then(|| format!("Agent Scheduler 已阻塞：任务生命周期未加载: {workflow_id}"))
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(
    app: &mut ExecutionState<'_>,
    message: TaskBoardMessage,
) -> Vec<ExecutionEffect> {
    match message {
    TaskBoardMessage::StartExecution => {
        if !(*app.project_path).as_deref().is_some_and(|path| std::path::Path::new(path).is_dir()) {
            return error_toast("项目目录不存在".to_string());
        }
        (*app.task_board_executor_running) = true;
        (*app.task_board_last_log_flush_at_ms) = 0;
        (*app.task_board_log_scan_cursor) = 0;
        (*app.task_board_timeout_scan_cursor) = 0;
        (*app.task_board_schedule_scan_cursor) = 0;
        (*app.task_board_next_scheduler_tick_at_ms) = app.next_deadline_ms;
        done(TaskBoardMessage::ExecutionTick)
    },
    TaskBoardMessage::StopExecution => {
        (*app.task_board_executor_running) = false;
        vec![]
    }
        TaskBoardMessage::ExecutionTick => {
        // Logs are flushed by the host before borrowing this domain state.

        if !(*app.task_board_executor_running) {
            return vec![];
        }

            let maintenance = (*app.project_path).clone().map(|path| {
                vec![ExecutionEffect::Maintain(path)]
            }).unwrap_or_else(Vec::new);
            batch(vec![maintenance, vec![ExecutionEffect::Wake(WakeKind::Execution)]])
        },
        TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    if gateway_http::is_gateway_unavailable_error(&error) {
                        tracing::debug!(
                            error = %error,
                            "agent scheduler execution wake skipped because gateway is unavailable"
                        );
                    } else {
                        tracing::warn!(error = %error, "agent scheduler execution wake failed");
                    }
                    return vec![ExecutionEffect::ScheduleTick];
                }
            };
            dispatch_wake(WakeKind::Execution, response)
        },
    TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick { project_path, task_id } => {
        if !super::projection::task_board_project_matches((*app.project_path).as_deref(), &project_path) {
            return vec![];
        }
        let Some(task) = app
            .task_board_tasks
            .iter_mut()
            .find(|task| task.id == task_id && task.status == TaskStatus::PrSubmitted)
        else {
            return vec![];
        };
        add_agent_scheduler_command_log_once(
            task,
            "Agent Scheduler 合并命令等待返回，正在同步网关状态/日志",
        );
        app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
        batch(vec![
            vec![ExecutionEffect::SyncStatus(project_path.clone())],
            vec![ExecutionEffect::SyncInFlight(project_path, task_id)],
        ])
    },
    TaskBoardMessage::AgentSchedulerMergeReadyPersisted { project_path, task, result } => {
        if !super::projection::task_board_project_matches((*app.project_path).as_deref(), &project_path) {
            return vec![];
        }
        let task_id = task.id.clone();
        if let Some(task_in_list) = (*app.task_board_tasks).iter_mut().find(|item| item.id == task_id)
        {
            match result {
                Ok(()) => {
                    add_agent_scheduler_command_log_once(
                        task_in_list,
                        "已发送 Agent Scheduler 合并命令",
                    );
                }
                Err(error) => {
                    task_in_list.mark_paused(format!(
                        "本地合并状态保存失败，未发送 Agent Scheduler 合并命令: {error}"
                    ));
                    app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
                    return vec![];
                }
            }
        }
            app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
            if let Some(error) = agent_scheduler_lifecycle_error(&app.task_board_settings.workflow_id) {
                if let Some(task) = (*app.task_board_tasks).iter_mut().find(|t| t.id == task_id) {
                    task.mark_paused(error.clone());
                }
                return error_toast(error);
            }
            batch(vec![
                command(
                    project_path.clone(),
                    task,
                    (*app.task_board_settings).workflow_id.clone(),
                AgentSchedulerCommandKindDto::Merge,
                (*app.task_board_settings).agent_scheduler_dispatch_mode,
                None,
                Some("自动审核已跳过，进入合并阶段".to_string()),
            ),
            vec![ExecutionEffect::SyncInFlight(project_path, task_id)],
        ])
    },
    TaskBoardMessage::AutoCodeReviewTick => { vec![] }
    TaskBoardMessage::SimulateStep { task_id } => {
        if !(*app.task_board_executor_running) {
            return vec![];
        }

        if let Some(task) = (*app.task_board_tasks).iter().find(|t| t.id == task_id) {
            let current_status = task.status;
            if let Some(new_status) = desktop_task_executor::executor::scheduling::simulate_task_execution_step(
                &(*app.project_path).clone().unwrap_or_default(),
                &task_id,
                current_status,
            ) {
                (*app.task_board_executor).finish_task(&task_id);
                return batch(vec![
                    done(TaskBoardMessage::TaskStatusChanged {
                        task_id: task_id.clone(),
                        new_status,
                    }),
                    vec![ExecutionEffect::ScheduleTick],
                ]);
            }
        }

        (*app.task_board_executor).finish_task(&task_id);
        vec![ExecutionEffect::ScheduleTick]
        },
        TaskBoardMessage::PromotePoolTasksTick => {
            if (*app.project_path).is_none() {
                return vec![ExecutionEffect::SchedulePromote];
            }
            batch(vec![
                vec![ExecutionEffect::Wake(WakeKind::Promote)],
            ])
        },
        TaskBoardMessage::AgentSchedulerPoolWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(error = %error, "agent scheduler promote wake failed");
                    return vec![ExecutionEffect::SchedulePromote];
                }
            };
            dispatch_wake(WakeKind::Promote, response)
        },
    TaskBoardMessage::AutoMergeTick => { vec![] }
        TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(error = %error, "agent scheduler merge wake failed");
                    return vec![ExecutionEffect::ScheduleMerge];
                }
            };
            dispatch_wake(WakeKind::Merge, response)
        },
        TaskBoardMessage::ExecuteTask { task_id } => {
            if let Some(task) = (*app.task_board_tasks).iter().find(|t| t.id == task_id).cloned() {
                if let Some(project_path) = &(*app.project_path) {
                    let path = project_path.clone();
                    if (*app.task_board_executor).is_running(&task_id) {
                        if let Some(task_in_list) =
                            (*app.task_board_tasks).iter_mut().find(|t| t.id == task_id)
                        {
                            task_in_list.add_localized_log("任务已在执行或等待工作区，忽略重复启动", vec![]);
                        }
                        app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
                        return vec![];
                    }
                    if let Some(task_in_list) =
                        (*app.task_board_tasks).iter_mut().find(|t| t.id == task_id)
                    {
                        let preparing_log =
                            if should_continue_existing_session_for_manual_retry(&task, true) {
                                "手工重试任务，已发送 Agent Scheduler 重试命令"
                            } else {
                                "已发送 Agent Scheduler 启动命令"
                            };
                        add_agent_scheduler_command_log_once(task_in_list, preparing_log);
                    }
                    app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
                    let command =
                        if should_continue_existing_session_for_manual_retry(&task, true) {
                            AgentSchedulerCommandKindDto::Retry
                        } else {
                            AgentSchedulerCommandKindDto::Start
                        };
                    if let Some(error) = agent_scheduler_lifecycle_error(&app.task_board_settings.workflow_id) {
                        if let Some(task_in_list) =
                            (*app.task_board_tasks).iter_mut().find(|t| t.id == task_id)
                        {
                            task_in_list.mark_execution_failed(error.clone());
                            app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
                        }
                        return error_toast(error);
                    }
                    return command(
                        path,
                        task,
                        (*app.task_board_settings).workflow_id.clone(),
                        command,
                        (*app.task_board_settings).agent_scheduler_dispatch_mode,
                        Some((*app.task_board_settings).max_concurrent.clamp(1, 10)),
                        None,
                    );
                } else if let Some(task_in_list) =
                    (*app.task_board_tasks).iter_mut().find(|t| t.id == task_id)
                {
                    task_in_list.mark_execution_failed("缺少项目路径".to_string());
                    app.effects.push(ExecutionEffect::SyncLogs(app.task_board_tasks.iter().find(|task| task.id == task_id).cloned()));
                }
            }
            vec![]
        }
        _ => vec![],
        }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;
