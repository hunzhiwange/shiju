//! Gateway 编排快照与项目归属控制边界。

use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerScopeDto, AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskGraphResponse, AgentSchedulerTaskLogsResponse, AgentTaskStatusDto,
};

use super::cron::CronJobSnapshot;
use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::agent_view::AgentViewState;
use crate::cli::tui::model::background_task::{
    BackgroundTask, BackgroundTaskKind, BackgroundTaskPanel, BackgroundTaskStatus,
    CanonicalTaskStatus, TaskArtifactSummary, TaskControlAction, TaskDetailView,
    TaskGraphNodeSummary, TaskLogSummary,
};
use crate::cli::tui::model::integration::sanitize_integration_text;
use crate::cli::tui::model::schedule_panel::{SchedulePanel, SchedulePanelItem};
use crate::cli::tui::model::workflow_panel::{WorkflowPanel, WorkflowPanelItem};

const MAX_TASK_ID_CHARS: usize = 160;
const MAX_TASK_LABEL_CHARS: usize = 160;
const MAX_TASK_DETAIL_CHARS: usize = 512;
const MAX_TASK_DEPENDENCIES: usize = 12;
const MAX_TASK_LOGS: usize = 20;
const MAX_TASK_ARTIFACTS: usize = 20;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct OrchestrationSnapshot {
    pub(crate) scope: Option<AgentSchedulerScopeDto>,
    pub(crate) workflows: WorkflowPanel,
    pub(crate) schedules: SchedulePanel,
    pub(crate) cron_jobs: Vec<CronJobSnapshot>,
    pub(crate) agents: AgentViewState,
    pub(crate) tasks: BackgroundTaskPanel,
    pub(crate) warnings: Vec<String>,
}

/// Workflow 详情中允许进入 TUI transcript 的净化字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDetailSummary {
    name: String,
    description: String,
    updated_at_ms: u64,
    node_count: usize,
    edge_count: usize,
}

impl WorkflowDetailSummary {
    pub(crate) fn render_text(&self) -> String {
        let description = if self.description.trim().is_empty() {
            "（无描述）"
        } else {
            self.description.as_str()
        };
        format!(
            "Workflow 详情\n名称：{}\n描述：{description}\n更新时间：{}\n结构：{} 个节点 · {} 条连接",
            self.name, self.updated_at_ms, self.node_count, self.edge_count,
        )
    }
}

pub(crate) async fn load_orchestration(
    runtime: &GatewayUiRuntime,
    scope: Option<AgentSchedulerScopeDto>,
) -> OrchestrationSnapshot {
    let task_scope = scope.clone();
    let (workflows, schedules, tasks) = tokio::join!(
        runtime.client().workflow_applications_list(),
        runtime.client().cron_jobs_get(),
        async {
            match task_scope.as_ref() {
                Some(scope) => runtime.client().agent_scheduler_status(scope).await,
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
            for item in items {
                snapshot.cron_jobs.push(CronJobSnapshot::from_dto(&item));
                snapshot.schedules.items.push(SchedulePanelItem {
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
                });
            }
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
                        canonical_status: canonical_task_status(&status),
                        detail: task
                            .read_model
                            .status_label
                            .unwrap_or_else(|| format!("{status:?}")),
                        log: Vec::new(),
                    }
                })
                .collect(),
        ),
        Err(error) => {
            snapshot.tasks.error = Some(error.clone());
            snapshot.warnings.push(format!("tasks: {error}"));
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
pub(crate) fn load_owned_workflow_detail_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &OrchestrationSnapshot,
    uuid: &str,
) -> Result<WorkflowDetailSummary, String> {
    let uuid = uuid.trim();
    if !snapshot.workflows.items.iter().any(|item| item.uuid == uuid) {
        return Err(
            "workflow detail requires a UUID from the current Workflow snapshot".to_string()
        );
    }
    let record = block_on_gateway(async { runtime.client().workflow_application_get(uuid).await })?;
    if record.uuid != uuid {
        return Err("Gateway Workflow detail UUID does not match the selected snapshot item".to_string());
    }
    let (node_count, edge_count) = workflow_structure_counts(record.workflow_yaml.as_str());
    Ok(WorkflowDetailSummary {
        name: record.name,
        description: record.description,
        updated_at_ms: record.updated_at_ms,
        node_count,
        edge_count,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn control_owned_task_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &OrchestrationSnapshot,
    task_id: &str,
    action: TaskControlAction,
) -> Result<(), String> {
    let task_id = task_id.trim();
    let task = snapshot
        .tasks
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| "task control requires an ID owned by the current project snapshot".to_string())?;
    let scope = snapshot
        .scope
        .as_ref()
        .ok_or_else(|| "task control requires a resolved project scope".to_string())?;
    if task.session_id != scope.project_scope().project_id().as_ref() {
        return Err("task control snapshot does not match the resolved project scope".to_string());
    }
    if !task.canonical_status.allows(action) {
        return Err(format!(
            "task canonical state does not allow {}",
            action.command_name()
        ));
    }
    let request = AgentSchedulerCommandRequest {
        task_id: task_id.into(),
        command: task_control_command(action),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("requested from TUI canonical task panel".to_string()),
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    block_on_gateway(async {
        runtime.client().agent_scheduler_command(scope, &request).await.map(|_| ())
    })
}

fn task_control_command(action: TaskControlAction) -> AgentSchedulerCommandKindDto {
    match action {
        TaskControlAction::Start => AgentSchedulerCommandKindDto::Start,
        TaskControlAction::Retry => AgentSchedulerCommandKindDto::Retry,
        TaskControlAction::Pause => AgentSchedulerCommandKindDto::Pause,
        TaskControlAction::Resume => AgentSchedulerCommandKindDto::Resume,
        TaskControlAction::Cancel => AgentSchedulerCommandKindDto::Cancel,
        TaskControlAction::Archive => AgentSchedulerCommandKindDto::Archive,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_owned_task_detail_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &OrchestrationSnapshot,
    task_id: &str,
) -> Result<TaskDetailView, String> {
    let task_id = task_id.trim();
    let task = snapshot
        .tasks
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| {
            "task detail requires an ID from the current project snapshot".to_string()
        })?;
    let scope = snapshot
        .scope
        .as_ref()
        .ok_or_else(|| "task detail requires a resolved project scope".to_string())?;
    if task.session_id != scope.project_scope().project_id().as_ref() {
        return Err("task detail snapshot does not match the resolved project scope".to_string());
    }
    let (graph, logs, artifacts) = block_on_gateway(async {
        Ok(tokio::join!(
            runtime.client().agent_scheduler_task_graph(scope),
            runtime.client().agent_scheduler_logs(scope, task_id),
            runtime.client().agent_scheduler_artifacts(scope, task_id),
        ))
    })?;
    Ok(TaskDetailView {
        task_id: bounded_task_text(task_id, MAX_TASK_ID_CHARS),
        graph: summarize_task_graph(scope, task_id, graph),
        logs: summarize_task_logs(task_id, logs),
        artifacts: summarize_task_artifacts(task_id, artifacts),
    })
}

fn summarize_task_graph(
    scope: &AgentSchedulerScopeDto,
    task_id: &str,
    response: Result<AgentSchedulerTaskGraphResponse, String>,
) -> Result<TaskGraphNodeSummary, String> {
    let response = response.map_err(|error| bounded_task_text(&error, MAX_TASK_DETAIL_CHARS))?;
    if response.project_id.as_ref() != scope.project_scope().project_id().as_ref() {
        return Err("Gateway task graph project does not match the selected scope".to_string());
    }
    let node = response
        .tasks
        .into_iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| "Gateway task graph does not contain the selected task".to_string())?;
    Ok(TaskGraphNodeSummary {
        status: bounded_task_text(&format!("{:?}", node.status), MAX_TASK_LABEL_CHARS),
        readiness: bounded_task_text(&format!("{:?}", node.readiness), MAX_TASK_LABEL_CHARS),
        waiting_task_ids: node
            .waiting_task_ids
            .into_iter()
            .take(MAX_TASK_DEPENDENCIES)
            .map(|id| bounded_task_text(&id, MAX_TASK_ID_CHARS))
            .collect(),
        blocking_task_ids: node
            .blocking_task_ids
            .into_iter()
            .take(MAX_TASK_DEPENDENCIES)
            .map(|id| bounded_task_text(&id, MAX_TASK_ID_CHARS))
            .collect(),
    })
}

fn summarize_task_logs(
    task_id: &str,
    response: Result<AgentSchedulerTaskLogsResponse, String>,
) -> Result<Vec<TaskLogSummary>, String> {
    let response = response.map_err(|error| bounded_task_text(&error, MAX_TASK_DETAIL_CHARS))?;
    if response.task_id.as_ref() != task_id
        || response.logs.iter().any(|log| log.task_id.as_ref() != task_id)
    {
        return Err("Gateway task logs contain a different task ID".to_string());
    }
    let skip = response.logs.len().saturating_sub(MAX_TASK_LOGS);
    Ok(response
        .logs
        .into_iter()
        .skip(skip)
        .map(|log| {
            let from = log
                .from_status_key
                .or_else(|| log.status_from.map(|status| format!("{status:?}")))
                .unwrap_or_else(|| "—".to_string());
            let to = log
                .to_status_key
                .or_else(|| log.status_to.map(|status| format!("{status:?}")))
                .unwrap_or_else(|| "—".to_string());
            TaskLogSummary {
                seq_no: log.seq_no,
                timestamp_ms: log.timestamp_ms.0,
                transition: bounded_task_text(
                    &format!("{from} → {to}"),
                    MAX_TASK_LABEL_CHARS,
                ),
                message: bounded_task_text(&log.message, MAX_TASK_DETAIL_CHARS),
            }
        })
        .collect())
}

fn summarize_task_artifacts(
    task_id: &str,
    response: Result<AgentSchedulerTaskArtifactsResponse, String>,
) -> Result<Vec<TaskArtifactSummary>, String> {
    let response = response.map_err(|error| bounded_task_text(&error, MAX_TASK_DETAIL_CHARS))?;
    if response.task_id.as_ref() != task_id
        || response.artifacts.iter().any(|artifact| artifact.task_id.as_ref() != task_id)
    {
        return Err("Gateway task artifacts contain a different task ID".to_string());
    }
    Ok(response
        .artifacts
        .into_iter()
        .take(MAX_TASK_ARTIFACTS)
        .map(|artifact| TaskArtifactSummary {
            run_id: artifact
                .run_id
                .as_deref()
                .map(|value| bounded_task_text(value, MAX_TASK_ID_CHARS)),
            kind: bounded_task_text(&artifact.kind, MAX_TASK_LABEL_CHARS),
            name: bounded_task_text(&artifact.name, MAX_TASK_LABEL_CHARS),
            path: artifact
                .path
                .as_deref()
                .map(|value| bounded_task_text(value, MAX_TASK_DETAIL_CHARS)),
            created_at_ms: artifact.created_at_ms.0,
        })
        .collect())
}

fn bounded_task_text(value: &str, max_chars: usize) -> String {
    let sanitized = sanitize_integration_text(&redact_bearer_value(value));
    if sanitized.chars().count() <= max_chars {
        return sanitized;
    }
    if max_chars <= 3 {
        return sanitized.chars().take(max_chars).collect();
    }
    format!(
        "{}...",
        sanitized.chars().take(max_chars.saturating_sub(3)).collect::<String>()
    )
}

fn redact_bearer_value(value: &str) -> String {
    let mut redact_next = false;
    value
        .split_whitespace()
        .map(|word| {
            if redact_next {
                redact_next = false;
                return "[redacted]";
            }
            let normalized = word.to_ascii_lowercase();
            if normalized == "bearer"
                || (normalized.contains("authorization=") && normalized.ends_with("bearer"))
            {
                redact_next = true;
            }
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
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

fn canonical_task_status(status: &AgentTaskStatusDto) -> CanonicalTaskStatus {
    match status {
        AgentTaskStatusDto::Draft => CanonicalTaskStatus::Draft,
        AgentTaskStatusDto::Queued => CanonicalTaskStatus::Queued,
        AgentTaskStatusDto::Assigned => CanonicalTaskStatus::Assigned,
        AgentTaskStatusDto::Dispatched => CanonicalTaskStatus::Dispatched,
        AgentTaskStatusDto::Acknowledged => CanonicalTaskStatus::Acknowledged,
        AgentTaskStatusDto::Active => CanonicalTaskStatus::Active,
        AgentTaskStatusDto::Running => CanonicalTaskStatus::Running,
        AgentTaskStatusDto::Submitted => CanonicalTaskStatus::Submitted,
        AgentTaskStatusDto::Revision => CanonicalTaskStatus::Revision,
        AgentTaskStatusDto::Blocked => CanonicalTaskStatus::Blocked,
        AgentTaskStatusDto::Paused => CanonicalTaskStatus::Paused,
        AgentTaskStatusDto::Completed => CanonicalTaskStatus::Completed,
        AgentTaskStatusDto::Failed => CanonicalTaskStatus::Failed,
        AgentTaskStatusDto::Cancelled => CanonicalTaskStatus::Cancelled,
        AgentTaskStatusDto::Archived => CanonicalTaskStatus::Archived,
    }
}

fn workflow_structure_counts(source: &str) -> (usize, usize) {
    let mut section = None;
    let mut section_indent = 0;
    let mut item_indent = None;
    let mut node_count = 0;
    let mut edge_count = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len().saturating_sub(line.trim_start().len());
        if let Some(active) = section {
            if indent <= section_indent {
                section = None;
                item_indent = None;
            } else if trimmed.starts_with("- ") {
                let list_indent = *item_indent.get_or_insert(indent);
                if indent == list_indent {
                    match active {
                        "nodes" => node_count += 1,
                        "edges" => edge_count += 1,
                        _ => {}
                    }
                }
            }
        }
        match trimmed {
            "nodes:" => {
                section = Some("nodes");
                section_indent = indent;
                item_indent = None;
            }
            "edges:" => {
                section = Some("edges");
                section_indent = indent;
                item_indent = None;
            }
            _ => {}
        }
    }
    (node_count, edge_count)
}
