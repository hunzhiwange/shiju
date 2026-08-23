//! Gateway 编排快照与项目归属控制边界。

use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerScopeDto, AgentTaskStatusDto,
    AgentWorkerStatusDto,
};

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::agent_view::{AgentMember, AgentStatus, AgentViewState};
use crate::cli::tui::model::background_task::{
    BackgroundTask, BackgroundTaskKind, BackgroundTaskPanel, BackgroundTaskStatus,
};
use crate::cli::tui::model::schedule_panel::{SchedulePanel, SchedulePanelItem};
use crate::cli::tui::model::workflow_panel::{WorkflowPanel, WorkflowPanelItem};
use crate::cli::tui::model::PermissionMode;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct OrchestrationSnapshot {
    pub(crate) scope: Option<AgentSchedulerScopeDto>,
    pub(crate) workflows: WorkflowPanel,
    pub(crate) schedules: SchedulePanel,
    pub(crate) agents: AgentViewState,
    pub(crate) tasks: BackgroundTaskPanel,
    pub(crate) warnings: Vec<String>,
}

pub(crate) async fn load_orchestration(
    runtime: &GatewayUiRuntime,
    scope: Option<AgentSchedulerScopeDto>,
) -> OrchestrationSnapshot {
    let task_scope = scope.clone();
    let worker_scope = scope.clone();
    let (workflows, schedules, tasks, workers) = tokio::join!(
        runtime.client().workflow_applications_list(),
        runtime.client().cron_jobs_get(),
        async {
            match task_scope.as_ref() {
                Some(scope) => runtime.client().agent_scheduler_status(scope).await,
                None => Err("project scope unavailable".to_string()),
            }
        },
        async {
            match worker_scope.as_ref() {
                Some(scope) => runtime.client().agent_scheduler_workers(scope).await,
                None => Err("project scope unavailable".to_string()),
            }
        },
    );
    let mut snapshot = OrchestrationSnapshot { scope, ..OrchestrationSnapshot::default() };

    match workflows {
        Ok(items) => {
            snapshot.workflows.items = items
                .into_iter()
                .map(|item| WorkflowPanelItem {
                    uuid: item.uuid,
                    name: item.name,
                    description: item.description,
                })
                .collect();
        }
        Err(error) => {
            snapshot.workflows.error = Some(error.clone());
            snapshot.warnings.push(format!("workflows: {error}"));
        }
    }
    match schedules {
        Ok(items) => {
            snapshot.schedules.items = items
                .into_iter()
                .map(|item| SchedulePanelItem {
                    id: item.id,
                    name: item.name.unwrap_or_else(|| "unnamed".to_string()),
                    schedule: if item.expression.is_empty() {
                        item.schedule_kind
                    } else {
                        item.expression
                    },
                    next_run: item.next_run,
                    last_status: item.last_status,
                    enabled: item.enabled,
                    history: Vec::new(),
                    history_error: None,
                })
                .collect();
        }
        Err(error) => {
            snapshot.schedules.error = Some(error.clone());
            snapshot.warnings.push(format!("schedule: {error}"));
        }
    }
    match tasks {
        Ok(response) => snapshot.tasks.replace(
            response
                .tasks
                .into_iter()
                .map(|task| {
                    let status = task.status.unwrap_or(AgentTaskStatusDto::Draft);
                    BackgroundTask {
                        id: task.task_id.as_ref().to_string(),
                        session_id: task.project_id.as_ref().to_string(),
                        kind: BackgroundTaskKind::Agent,
                        title: task
                            .read_model
                            .title
                            .unwrap_or_else(|| task.task_id.as_ref().to_string()),
                        status: task_status(&status),
                        detail: task
                            .read_model
                            .status_label
                            .unwrap_or_else(|| format!("{status:?}")),
                        log: Vec::new(),
                        supports_stop: matches!(
                            status,
                            AgentTaskStatusDto::Queued
                                | AgentTaskStatusDto::Assigned
                                | AgentTaskStatusDto::Dispatched
                                | AgentTaskStatusDto::Acknowledged
                                | AgentTaskStatusDto::Active
                                | AgentTaskStatusDto::Running
                                | AgentTaskStatusDto::Paused
                        ),
                    }
                })
                .collect(),
        ),
        Err(error) => {
            snapshot.tasks.error = Some(error.clone());
            snapshot.warnings.push(format!("tasks: {error}"));
        }
    }
    match workers {
        Ok(response) => {
            snapshot.agents.members = response
                .workers
                .into_iter()
                .map(|worker| AgentMember {
                    id: worker.id.clone(),
                    session_id: worker.room_id.unwrap_or_default(),
                    team_id: worker.team_id,
                    name: worker.worker_role.unwrap_or(worker.id),
                    status: worker_status(worker.status),
                    messages: None,
                    permission_mode: PermissionMode::Default,
                    queued_input: Vec::new(),
                })
                .collect();
        }
        Err(error) => {
            snapshot.agents.error = Some(error.clone());
            snapshot.warnings.push(format!("agents: {error}"));
        }
    }
    snapshot
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_orchestration_blocking(runtime: &GatewayUiRuntime) -> OrchestrationSnapshot {
    let (scope, scope_warning) = match runtime.agent_scheduler_scope_blocking() {
        Ok(scope) => (Some(scope), None),
        Err(error) => (None, Some(format!("project scope: {error}"))),
    };
    let mut snapshot = block_on_gateway(async { Ok(load_orchestration(runtime, scope).await) })
        .unwrap_or_else(|error| OrchestrationSnapshot {
            warnings: vec![format!("orchestration: {error}")],
            ..OrchestrationSnapshot::default()
        });
    if let Some(warning) = scope_warning {
        snapshot.warnings.push(warning);
    }
    snapshot
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn control_owned_task_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &OrchestrationSnapshot,
    task_id: &str,
    command: AgentSchedulerCommandKindDto,
) -> Result<(), String> {
    let task_id = task_id.trim();
    let task = snapshot
        .tasks
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| "task control requires an ID owned by the current project snapshot".to_string())?;
    if !task.supports_stop && matches!(command, AgentSchedulerCommandKindDto::Cancel) {
        return Err("task is not in a cancellable canonical state".to_string());
    }
    let scope = snapshot
        .scope
        .as_ref()
        .ok_or_else(|| "task control requires a resolved project scope".to_string())?;
    let request = AgentSchedulerCommandRequest {
        task_id: task_id.into(),
        command,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("requested from TUI canonical task panel".to_string()),
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    block_on_gateway(async {
        runtime.client().agent_scheduler_command(scope, &request).await.map(|_| ())
    })
}

fn task_status(status: &AgentTaskStatusDto) -> BackgroundTaskStatus {
    match status {
        AgentTaskStatusDto::Completed | AgentTaskStatusDto::Archived => {
            BackgroundTaskStatus::Completed
        }
        AgentTaskStatusDto::Failed | AgentTaskStatusDto::Blocked => BackgroundTaskStatus::Failed,
        AgentTaskStatusDto::Cancelled => BackgroundTaskStatus::Cancelled,
        AgentTaskStatusDto::Draft
        | AgentTaskStatusDto::Queued
        | AgentTaskStatusDto::Assigned
        | AgentTaskStatusDto::Dispatched => BackgroundTaskStatus::Queued,
        _ => BackgroundTaskStatus::Running,
    }
}

fn worker_status(status: AgentWorkerStatusDto) -> AgentStatus {
    match status {
        AgentWorkerStatusDto::Pending => AgentStatus::Loading,
        AgentWorkerStatusDto::Idle => AgentStatus::Ready,
        AgentWorkerStatusDto::Busy | AgentWorkerStatusDto::Running => AgentStatus::Running,
        AgentWorkerStatusDto::Stopped | AgentWorkerStatusDto::Sleeping => AgentStatus::Stopped,
        AgentWorkerStatusDto::Failed | AgentWorkerStatusDto::Unavailable => AgentStatus::Failed,
    }
}
