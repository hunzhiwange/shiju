//! Scheduler DTO projections, validation and public error mapping.
use super::*;
pub fn start_command_is_idempotent_for_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned
            | AgentTaskStatus::Dispatched
            | AgentTaskStatus::Acknowledged
            | AgentTaskStatus::Running
            | AgentTaskStatus::Submitted
    )
}

pub fn normalize_project_wake_max_concurrent(max_concurrent: Option<u32>) -> usize {
    max_concurrent
        .unwrap_or(DEFAULT_PROJECT_WAKE_MAX_CONCURRENT as u32)
        .clamp(1, MAX_PROJECT_WAKE_MAX_CONCURRENT as u32) as usize
}

pub fn execution_slot_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned
            | AgentTaskStatus::Dispatched
            | AgentTaskStatus::Acknowledged
            | AgentTaskStatus::Running
    )
}

pub fn task_board_write_command(command: AgentSchedulerCommandKindDto) -> bool {
    matches!(
        command,
        AgentSchedulerCommandKindDto::Archive
            | AgentSchedulerCommandKindDto::Move
            | AgentSchedulerCommandKindDto::UpdateMetadata
            | AgentSchedulerCommandKindDto::SoftDelete
            | AgentSchedulerCommandKindDto::Restore
            | AgentSchedulerCommandKindDto::ImportLegacyTask
    )
}

pub(super) fn new_task_from_request(
    request: AgentSchedulerCreateTaskRequest,
) -> Result<NewAgentTask, ApiError> {
    let status = task_status_from_dto(request.status.unwrap_or(AgentTaskStatusDto::Queued));
    Ok(NewAgentTask {
        id: TaskId::new(request.task_id).map_err(map_bad_request)?,
        title: non_empty(request.title, "task title")?,
        kind: task_kind_from_dto(request.kind),
        status,
        spec_uri: non_empty(request.spec_uri, "task spec uri")?,
        meta_uri: non_empty(request.meta_uri, "task meta uri")?,
        assigned_to: optional_worker_name(request.assigned_to)?,
        team_id: optional_team_id(request.team_id)?,
        room_id: request.room_id,
        project_room_id: request.project_room_id,
        delegated_to_team: optional_team_id(request.delegated_to_team)?,
        schedule: request.schedule,
        timezone: request.timezone,
        next_scheduled_at: request.next_scheduled_at,
        last_executed_at: request.last_executed_at,
        dag: Vec::new(),
        metadata: request.metadata,
        created_at_ms: request.created_at_ms,
    })
}

pub(super) fn append_task_created_event(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
) -> Result<(), ApiError> {
    store
        .append_event(
            context,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: None,
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind: AgentEventKind::TaskCreated,
                from_status: None,
                to_status: Some(task.status),
                from_status_key: None,
                to_status_key: None,
                message: format!("created agent scheduler task {}", task.id.as_str()),
                payload: json!({
                    "task_id": task.id.as_str(),
                    "title": task.title,
                    "kind": task_kind_to_dto(task.kind),
                    "status": task_status_to_dto(task.status),
                }),
                created_at_ms: task.created_at_ms,
            },
        )
        .map_err(map_scheduler_error)?;
    Ok(())
}

pub(super) fn wake_response_from_outcome(outcome: Option<DispatchOutcome>) -> AgentSchedulerWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run_id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: outcome.outbox_message_id,
        },
        None => AgentSchedulerWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

pub(super) fn recovery_response_from_outcome(outcome: RecoveryOutcome) -> AgentSchedulerRecoveryResponse {
    AgentSchedulerRecoveryResponse {
        graced_run_ids: outcome
            .graced_run_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        recovered_run_ids: outcome
            .recovered_run_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        released_resource_lease_ids: outcome
            .released_resource_lease_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        tainted_resource_lease_ids: outcome
            .tainted_resource_lease_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        drifted_worker_ids: Vec::new(),
        slept_worker_ids: Vec::new(),
        outbox_message_ids: outcome.outbox_message_ids,
    }
}

pub fn owned_scheduler_context(
    context: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<AgentSchedulerContext, ApiError> {
    AgentSchedulerContext::new(
        context.storage_scope().map_err(map_scheduler_error)?,
        context.storage_project_path(),
    )
    .map_err(map_scheduler_error)
}

pub(super) fn task_id_from_api(task_id: &vw_api_transport::TaskId) -> Result<TaskId, ApiError> {
    TaskId::new(task_id.as_ref().to_string()).map_err(map_bad_request)
}

pub(super) fn current_timestamp_ms() -> TimestampMs {
    TimestampMs(chrono::Utc::now().timestamp_millis().max(0))
}

pub fn add_ms_or_saturate(timestamp: TimestampMs, delta_ms: i64) -> TimestampMs {
    TimestampMs(timestamp.0.saturating_add(delta_ms).max(timestamp.0))
}

#[cfg(test)]
pub(super) fn workspace_payload(workspace: &AgentSchedulerWorkspaceContextDto) -> Value {
    json!({
        "slot_id": workspace.slot_id,
        "path": workspace.path,
        "branch": workspace.branch,
    })
}

pub(super) fn dispatch_mode_from_dto(dispatch_mode: AgentSchedulerDispatchModeDto) -> DispatchMode {
    match dispatch_mode {
        AgentSchedulerDispatchModeDto::Local => DispatchMode::Local,
        AgentSchedulerDispatchModeDto::Auto => DispatchMode::Auto,
    }
}

pub(super) fn append_command_event(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
    run: Option<&StoredAgentRun>,
    command: AgentSchedulerCommandKindDto,
    from_status: AgentTaskStatus,
    reason: Option<&str>,
    now_ms: TimestampMs,
) -> Result<StoredAgentEvent, ApiError> {
    let message =
        reason.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string).unwrap_or_else(
            || format!("accepted agent scheduler command {}", command_kind_key(command)),
        );
    store
        .append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: run.map(|run| run.id.clone()),
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind: command_event_kind(command),
                from_status: Some(from_status),
                to_status: Some(task.status),
                from_status_key: None,
                to_status_key: None,
                message,
                payload: json!({
                    "command": command_kind_key(command),
                    "task_id": task.id.as_str(),
                    "run_id": run.map(|run| run.id.as_str()),
                }),
                created_at_ms: now_ms,
            },
        )
        .map_err(map_scheduler_error)
}

pub fn command_event_kind(command: AgentSchedulerCommandKindDto) -> AgentEventKind {
    match command {
        AgentSchedulerCommandKindDto::Pause => AgentEventKind::TaskPaused,
        AgentSchedulerCommandKindDto::Resume => AgentEventKind::TaskResumed,
        AgentSchedulerCommandKindDto::Cancel => AgentEventKind::TaskCancelled,
        AgentSchedulerCommandKindDto::Archive => AgentEventKind::TaskArchived,
        _ => AgentEventKind::HumanIntervened,
    }
}

pub(super) fn task_board_task_to_dto_with_latest_run(
    store: &AgentSchedulerStore,
    project_id: &str,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: StoredAgentTask,
) -> Result<AgentSchedulerTaskBoardTaskDto, ApiError> {
    let run =
        store.load_latest_run_for_task(project_path, &task.id).map_err(map_scheduler_error)?;
    task_board_task_to_dto(store, project_id, project_path, task, run.as_ref())
}

pub(super) fn task_board_task_to_dto(
    store: &AgentSchedulerStore,
    project_id: &str,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: StoredAgentTask,
    run: Option<&StoredAgentRun>,
) -> Result<AgentSchedulerTaskBoardTaskDto, ApiError> {
    let metadata = &task.metadata;
    let workspace = run
        .map(|run| run_workspace_to_dto(store, project_path, &task.id, &run.id))
        .transpose()?
        .flatten()
        .or_else(|| workspace_context_from_value(metadata));
    let selected_worktree_slot_id = string_field(metadata, "selected_worktree_slot_id")
        .or_else(|| workspace.as_ref().and_then(|workspace| workspace.slot_id.clone()));
    let selected_worktree_path = string_field(metadata, "selected_worktree_path").or_else(|| {
        workspace
            .as_ref()
            .filter(|workspace| workspace.slot_id.is_some())
            .map(|workspace| workspace.path.clone())
    });
    let selected_worktree_branch = string_field(metadata, "selected_worktree_branch")
        .or_else(|| workspace.as_ref().and_then(|workspace| workspace.branch.clone()));
    let merge_source_branch = string_field(metadata, "merge_source_branch")
        .or_else(|| selected_worktree_branch.clone().filter(|branch| valid_merge_branch(branch)));
    let last_error = run
        .and_then(|run| run.error_message.as_deref().and_then(non_empty_str).map(str::to_string))
        .or_else(|| string_field(metadata, "last_error"));
    let status_key = task_board_status_key(task.status, metadata).to_string();
    let read_model = task_board_task_read_model(&task, metadata, &status_key);

    Ok(AgentSchedulerTaskBoardTaskDto {
        project_id: project_id.into(),
        task_id: task.id.as_str().into(),
        status_key,
        status: Some(task_status_to_dto(task.status)),
        priority: u32_field(metadata, "priority").unwrap_or_default(),
        order_no: i64_field(metadata, "order_no").unwrap_or_default(),
        model: string_field(metadata, "model").unwrap_or_default(),
        agent: string_field(metadata, "agent"),
        acp_agent: string_field(metadata, "acp_agent"),
        reasoning_effort: reasoning_effort_field(metadata)?,
        prompt: string_field(metadata, "prompt").unwrap_or_else(|| task.title.clone()),
        retry_count: run.map_or(0, |run| run.attempt.saturating_sub(1)),
        run_id: run.map(|run| run.id.as_str().to_string()),
        run_stage: run.and_then(|run| run_stage_from_run_status(run.status).map(str::to_string)),
        workspace,
        last_error,
        pause_reason: string_field(metadata, "pause_reason"),
        merge_source_branch,
        merge_target_branch: string_field(metadata, "merge_target_branch"),
        selected_worktree_path,
        selected_worktree_slot_id,
        selected_worktree_branch,
        read_model,
        updated_at_ms: task.updated_at_ms,
    })
}

pub(super) fn task_board_task_read_model(
    task: &StoredAgentTask,
    metadata: &Value,
    status_key: &str,
) -> AgentSchedulerTaskBoardTaskReadModelDto {
    AgentSchedulerTaskBoardTaskReadModelDto {
        title: Some(task.title.clone()),
        workflow_id: string_field(metadata, "task_board.workflow_id"),
        status_label: string_field(metadata, "task_board.status_label"),
        status_role: task_board_status_role_from_metadata(metadata),
        stage_id: string_field(metadata, "task_board.stage_id")
            .or_else(|| Some(status_key.to_string())),
        stage_label: string_field(metadata, "task_board.stage_label"),
    }
}

pub fn task_board_status_role_from_metadata(metadata: &Value) -> Option<TaskPoolWorkflowStatusRoleDto> {
    let role = string_field(metadata, "task_board.status_role")?;
    match role.as_str() {
        "backlog" => Some(TaskPoolWorkflowStatusRoleDto::Backlog),
        "queued" => Some(TaskPoolWorkflowStatusRoleDto::Queued),
        "planning" => Some(TaskPoolWorkflowStatusRoleDto::Planning),
        "executing" => Some(TaskPoolWorkflowStatusRoleDto::Executing),
        "failed" => Some(TaskPoolWorkflowStatusRoleDto::Failed),
        "paused" => Some(TaskPoolWorkflowStatusRoleDto::Paused),
        "review_ready" => Some(TaskPoolWorkflowStatusRoleDto::ReviewReady),
        "review" => Some(TaskPoolWorkflowStatusRoleDto::Review),
        "merging" => Some(TaskPoolWorkflowStatusRoleDto::Merging),
        "done" => Some(TaskPoolWorkflowStatusRoleDto::Done),
        "archived" => Some(TaskPoolWorkflowStatusRoleDto::Archived),
        "custom" => Some(TaskPoolWorkflowStatusRoleDto::Custom),
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn workflow_status_role(
    workflow: &TaskPoolWorkflowSnapshotDto,
    status_key: &str,
) -> Option<TaskPoolWorkflowStatusRoleDto> {
    workflow
        .statuses
        .iter()
        .find(|status| status.enabled && status.key == status_key)
        .map(|status| status.role)
}

#[cfg(test)]
pub(super) fn task_metadata_object(value: Value) -> Result<Map<String, Value>, ApiError> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => Err(ApiError::bad_request("task metadata must be a JSON object")),
    }
}

#[cfg(test)]
pub(super) fn set_lifecycle_projection_metadata(
    metadata: &mut Map<String, Value>,
    target: &LifecycleStatusProjection,
) {
    metadata.insert("task_board.status_key".to_string(), json!(target.status_key.clone()));
    metadata.insert(
        "task_board.status_role".to_string(),
        json!(lifecycle_status_role_key(target.status_role)),
    );
}

pub fn task_board_status_key(status: AgentTaskStatus, metadata: &Value) -> String {
    task_board_status_key_from_metadata(status, metadata)
        .unwrap_or_else(|| legacy_task_board_status_key(status).to_string())
}

pub(super) fn task_board_status_key_from_metadata(
    status: AgentTaskStatus,
    metadata: &Value,
) -> Option<String> {
    if let Some(status_key) =
        metadata.get("task_board.status_key").and_then(Value::as_str).and_then(non_empty_str)
    {
        return Some(status_key.to_string());
    }
    let status_key_map = metadata.get("task_board.status_key_map").and_then(Value::as_object)?;
    status_key_map
        .get(status.as_str())
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
}

pub fn legacy_task_board_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

pub fn valid_merge_branch(branch: &str) -> bool {
    let branch = branch.trim();
    !branch.is_empty() && branch != "HEAD"
}

pub(super) fn task_log_to_dto(log: StoredAgentTaskLog) -> AgentSchedulerTaskLogDto {
    AgentSchedulerTaskLogDto {
        task_id: log.task_id.as_str().into(),
        seq_no: log.seq_no,
        timestamp_ms: log.timestamp_ms,
        from_status_key: log
            .from_status_key
            .or_else(|| log.from_status.map(|status| status.as_str().to_string())),
        status_from: log.from_status.map(task_status_to_dto),
        to_status_key: log
            .to_status_key
            .or_else(|| log.to_status.map(|status| status.as_str().to_string())),
        status_to: log.to_status.map(task_status_to_dto),
        message: log.message,
    }
}

pub(super) fn task_artifact_to_dto(artifact: StoredTaskArtifact) -> AgentSchedulerTaskArtifactDto {
    AgentSchedulerTaskArtifactDto {
        task_id: artifact.task_id.as_str().into(),
        run_id: artifact.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        kind: artifact.kind,
        name: artifact.name,
        path: artifact.path,
        payload: artifact.payload,
        created_at_ms: artifact.created_at_ms,
    }
}

pub(super) fn task_board_event_to_dto(
    project_id: &str,
    event: StoredAgentEvent,
) -> Result<AgentSchedulerTaskBoardEventDto, ApiError> {
    let task_id = event
        .task_id
        .as_ref()
        .map(|task_id| task_id.as_str().to_string())
        .or_else(|| {
            task_id_from_payload(&event.payload).map(|task_id| task_id.as_str().to_string())
        })
        .ok_or_else(|| ApiError::internal("agent scheduler task board event requires task id"))?;
    let from_status_key = event
        .from_status_key
        .or_else(|| event.from_status.map(|status| status.as_str().to_string()));
    let to_status_key =
        event.to_status_key.or_else(|| event.to_status.map(|status| status.as_str().to_string()));
    let workspace = workspace_context_from_value(&event.payload);
    let error_message =
        event_error_message(event.kind, event.to_status, &event.message, &event.payload);
    Ok(AgentSchedulerTaskBoardEventDto {
        cursor: event.cursor,
        project_id: project_id.into(),
        task_id: task_id.into(),
        run_id: event.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        run_stage: event.to_status.and_then(run_stage_from_task_status).map(str::to_string),
        kind: event_kind_to_dto(event.kind),
        from_status_key,
        from_status: event.from_status.map(task_status_to_dto),
        to_status_key,
        to_status: event.to_status.map(task_status_to_dto),
        message: event.message,
        error_message,
        workspace,
        payload: event.payload,
        created_at_ms: event.created_at_ms,
    })
}

pub(super) fn project_wake_response_from_local_outcome(
    outcome: Option<LocalAgentRunnerOutcome>,
) -> AgentSchedulerProjectWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerProjectWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run.id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: None,
        },
        None => AgentSchedulerProjectWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

#[cfg(test)]
pub(super) fn empty_project_wake_response() -> AgentSchedulerProjectWakeResponse {
    AgentSchedulerProjectWakeResponse {
        dispatched: false,
        task_id: None,
        run_id: None,
        worker_id: None,
        outbox_message_id: None,
    }
}

pub(super) fn project_wake_response_from_dispatch_outcome(
    outcome: Option<DispatchOutcome>,
) -> AgentSchedulerProjectWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerProjectWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run_id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: outcome.outbox_message_id,
        },
        None => AgentSchedulerProjectWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

pub(super) fn task_graph_task_to_dto(
    task: &TaskGraphTaskSnapshot,
    graph: &TaskGraph,
) -> Result<AgentSchedulerTaskGraphTaskDto, ApiError> {
    let readiness = graph.readiness_for(&task.id).map_err(map_scheduler_error)?;
    let (readiness, waiting_task_ids, blocking_task_ids) = match readiness {
        TaskGraphReadiness::Ready => {
            (AgentSchedulerTaskGraphReadinessDto::Ready, Vec::new(), Vec::new())
        }
        TaskGraphReadiness::WaitingDependencies { task_ids } => (
            AgentSchedulerTaskGraphReadinessDto::WaitingDependencies,
            task_ids.into_iter().map(|task_id| task_id.as_str().to_string()).collect(),
            Vec::new(),
        ),
        TaskGraphReadiness::BlockedByDependency { task_ids } => (
            AgentSchedulerTaskGraphReadinessDto::BlockedByDependency,
            Vec::new(),
            task_ids.into_iter().map(|task_id| task_id.as_str().to_string()).collect(),
        ),
        TaskGraphReadiness::NotQueued => {
            (AgentSchedulerTaskGraphReadinessDto::NotQueued, Vec::new(), Vec::new())
        }
    };
    Ok(AgentSchedulerTaskGraphTaskDto {
        task_id: task.id.as_str().to_string(),
        status: task_status_to_dto(task.status),
        readiness,
        waiting_task_ids,
        blocking_task_ids,
    })
}

pub(super) fn task_to_dto(task: StoredAgentTask) -> AgentTaskDto {
    AgentTaskDto {
        id: task.id.as_str().to_string(),
        title: task.title,
        kind: task_kind_to_dto(task.kind),
        status: task_status_to_dto(task.status),
        assigned_to: task.assigned_to.as_ref().map(AssigneeId::as_str).map(str::to_string),
        team_id: task.team_id.as_ref().map(TeamId::as_str).map(str::to_string),
        room_id: task.room_id,
        project_room_id: task.project_room_id,
        delegated_to_team: task.delegated_to_team.as_ref().map(TeamId::as_str).map(str::to_string),
        schedule: task.schedule,
        timezone: task.timezone,
        next_scheduled_at: task.next_scheduled_at,
        last_executed_at: task.last_executed_at,
        dag: task.dag.into_iter().map(dag_node_to_dto).collect(),
        created_at_ms: task.created_at_ms,
        updated_at_ms: task.updated_at_ms,
        metadata: task.metadata,
    }
}

pub(super) fn dag_node_to_dto(node: AgentTaskDagNode) -> TaskDependencyNodeDto {
    TaskDependencyNodeDto {
        task_id: node.task_id.as_str().to_string(),
        title: node.title,
        depends_on: node.depends_on.iter().map(|task_id| task_id.as_str().to_string()).collect(),
        assigned_to: node.assigned_to.as_ref().map(AssigneeId::as_str).map(str::to_string),
        status: task_status_to_dto(node.status),
    }
}

pub(super) fn event_to_dto(event: StoredAgentEvent) -> AgentEventDto {
    AgentEventDto {
        id: event.cursor.to_string(),
        task_id: event.task_id.as_ref().map(TaskId::as_str).map(str::to_string),
        run_id: event.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        kind: event_kind_to_dto(event.kind),
        message: event.message,
        payload: event.payload,
        created_at_ms: event.created_at_ms,
    }
}

#[cfg(test)]
pub(super) fn run_to_dto(run: StoredAgentRun) -> AgentRunDto {
    AgentRunDto {
        id: run.id.as_str().to_string(),
        task_id: run.task_id.as_str().to_string(),
        status: run_status_to_dto(run.status),
        lease_expires_at_ms: run.lease_expires_at_ms,
        attempt: run.attempt,
        started_at_ms: run.started_at_ms,
        finished_at_ms: run.finished_at_ms,
        last_heartbeat_at_ms: run.last_heartbeat_at_ms,
        error_message: run.error_message,
    }
}

pub(super) fn outbox_to_dto(message: StoredOutboxMessage) -> AgentOutboxMessageDto {
    AgentOutboxMessageDto {
        id: message.id,
        target_kind: message.channel,
        target_id: message.target,
        recipient_id: message.recipient_id,
        body: message.body,
        status: outbox_status_to_dto(message.status),
        idempotency_key: message.idempotency_key,
        attempts: message.attempts,
        next_attempt_at_ms: message.next_attempt_at_ms,
        delivered_at_ms: message.delivered_at_ms,
        last_error: message.last_error,
        payload: message.payload,
        created_at_ms: message.created_at_ms,
    }
}

#[cfg(test)]
pub(super) fn run_status_to_dto(status: AgentRunStatus) -> AgentRunStatusDto {
    match status {
        AgentRunStatus::Pending => AgentRunStatusDto::Pending,
        AgentRunStatus::Leased => AgentRunStatusDto::Leased,
        AgentRunStatus::Running => AgentRunStatusDto::Running,
        AgentRunStatus::Succeeded => AgentRunStatusDto::Succeeded,
        AgentRunStatus::Failed => AgentRunStatusDto::Failed,
        AgentRunStatus::Cancelled => AgentRunStatusDto::Cancelled,
        AgentRunStatus::TimedOut => AgentRunStatusDto::TimedOut,
        AgentRunStatus::Stale => AgentRunStatusDto::Stale,
    }
}

pub(super) fn task_kind_from_dto(kind: AgentTaskKindDto) -> AgentTaskKind {
    match kind {
        AgentTaskKindDto::Finite => AgentTaskKind::Finite,
        AgentTaskKindDto::Infinite => AgentTaskKind::Infinite,
        AgentTaskKindDto::DependencyGraph => AgentTaskKind::DependencyGraph,
    }
}

pub(super) fn task_kind_to_dto(kind: AgentTaskKind) -> AgentTaskKindDto {
    match kind {
        AgentTaskKind::Finite => AgentTaskKindDto::Finite,
        AgentTaskKind::Infinite => AgentTaskKindDto::Infinite,
        AgentTaskKind::DependencyGraph => AgentTaskKindDto::DependencyGraph,
    }
}

pub(super) fn task_status_from_dto(status: AgentTaskStatusDto) -> AgentTaskStatus {
    match status {
        AgentTaskStatusDto::Draft => AgentTaskStatus::Draft,
        AgentTaskStatusDto::Queued => AgentTaskStatus::Queued,
        AgentTaskStatusDto::Assigned => AgentTaskStatus::Assigned,
        AgentTaskStatusDto::Dispatched => AgentTaskStatus::Dispatched,
        AgentTaskStatusDto::Acknowledged => AgentTaskStatus::Acknowledged,
        AgentTaskStatusDto::Active => AgentTaskStatus::Active,
        AgentTaskStatusDto::Running => AgentTaskStatus::Running,
        AgentTaskStatusDto::Submitted => AgentTaskStatus::Submitted,
        AgentTaskStatusDto::Revision => AgentTaskStatus::Revision,
        AgentTaskStatusDto::Blocked => AgentTaskStatus::Blocked,
        AgentTaskStatusDto::Paused => AgentTaskStatus::Paused,
        AgentTaskStatusDto::Completed => AgentTaskStatus::Completed,
        AgentTaskStatusDto::Failed => AgentTaskStatus::Failed,
        AgentTaskStatusDto::Cancelled => AgentTaskStatus::Cancelled,
        AgentTaskStatusDto::Archived => AgentTaskStatus::Archived,
    }
}

pub(super) fn task_status_to_dto(status: AgentTaskStatus) -> AgentTaskStatusDto {
    match status {
        AgentTaskStatus::Draft => AgentTaskStatusDto::Draft,
        AgentTaskStatus::Queued => AgentTaskStatusDto::Queued,
        AgentTaskStatus::Assigned => AgentTaskStatusDto::Assigned,
        AgentTaskStatus::Dispatched => AgentTaskStatusDto::Dispatched,
        AgentTaskStatus::Acknowledged => AgentTaskStatusDto::Acknowledged,
        AgentTaskStatus::Active => AgentTaskStatusDto::Active,
        AgentTaskStatus::Running => AgentTaskStatusDto::Running,
        AgentTaskStatus::Submitted => AgentTaskStatusDto::Submitted,
        AgentTaskStatus::Revision => AgentTaskStatusDto::Revision,
        AgentTaskStatus::Blocked => AgentTaskStatusDto::Blocked,
        AgentTaskStatus::Paused => AgentTaskStatusDto::Paused,
        AgentTaskStatus::Completed => AgentTaskStatusDto::Completed,
        AgentTaskStatus::Failed => AgentTaskStatusDto::Failed,
        AgentTaskStatus::Cancelled => AgentTaskStatusDto::Cancelled,
        AgentTaskStatus::Archived => AgentTaskStatusDto::Archived,
    }
}

#[cfg(test)]
pub(super) fn capability_kind_from_dto(kind: AgentCapabilityKindDto) -> AgentCapabilityKind {
    match kind {
        AgentCapabilityKindDto::Skill => AgentCapabilityKind::Skill,
        AgentCapabilityKindDto::RemoteSkill => AgentCapabilityKind::RemoteSkill,
        AgentCapabilityKindDto::McpServer => AgentCapabilityKind::McpServer,
        AgentCapabilityKindDto::Model => AgentCapabilityKind::Model,
        AgentCapabilityKindDto::Runtime => AgentCapabilityKind::Runtime,
        AgentCapabilityKindDto::Tool => AgentCapabilityKind::Tool,
    }
}

#[cfg(test)]
pub(super) fn capability_kind_to_dto(kind: AgentCapabilityKind) -> AgentCapabilityKindDto {
    match kind {
        AgentCapabilityKind::Skill => AgentCapabilityKindDto::Skill,
        AgentCapabilityKind::RemoteSkill => AgentCapabilityKindDto::RemoteSkill,
        AgentCapabilityKind::McpServer => AgentCapabilityKindDto::McpServer,
        AgentCapabilityKind::Model => AgentCapabilityKindDto::Model,
        AgentCapabilityKind::Runtime => AgentCapabilityKindDto::Runtime,
        AgentCapabilityKind::Tool => AgentCapabilityKindDto::Tool,
    }
}

pub(super) fn event_kind_to_dto(kind: AgentEventKind) -> AgentEventKindDto {
    match kind {
        AgentEventKind::TaskCreated => AgentEventKindDto::TaskCreated,
        AgentEventKind::ImportedLegacyTask => AgentEventKindDto::ImportedLegacyTask,
        AgentEventKind::TaskAssigned => AgentEventKindDto::TaskAssigned,
        AgentEventKind::WorkerSelected => AgentEventKindDto::WorkerSelected,
        AgentEventKind::WorkerAcknowledged => AgentEventKindDto::WorkerAcknowledged,
        AgentEventKind::RunStarted => AgentEventKindDto::RunStarted,
        AgentEventKind::RunSubmitted => AgentEventKindDto::RunSubmitted,
        AgentEventKind::RunCompleted => AgentEventKindDto::RunCompleted,
        AgentEventKind::RunFailed => AgentEventKindDto::RunFailed,
        AgentEventKind::RunRecovered => AgentEventKindDto::RunRecovered,
        AgentEventKind::TaskBlocked => AgentEventKindDto::TaskBlocked,
        AgentEventKind::TaskPaused => AgentEventKindDto::TaskPaused,
        AgentEventKind::TaskResumed => AgentEventKindDto::TaskResumed,
        AgentEventKind::TaskCancelled => AgentEventKindDto::TaskCancelled,
        AgentEventKind::TaskArchived => AgentEventKindDto::TaskArchived,
        AgentEventKind::InfiniteScheduled => AgentEventKindDto::InfiniteScheduled,
        AgentEventKind::InfiniteExecuted => AgentEventKindDto::InfiniteExecuted,
        AgentEventKind::TeamNodeQueued => AgentEventKindDto::TeamNodeQueued,
        AgentEventKind::TeamNodeDispatched => AgentEventKindDto::TeamNodeDispatched,
        AgentEventKind::TeamNodeCompleted => AgentEventKindDto::TeamNodeCompleted,
        AgentEventKind::ResourceLeased => AgentEventKindDto::ResourceLeased,
        AgentEventKind::ResourceReleased => AgentEventKindDto::ResourceReleased,
        AgentEventKind::OutboxQueued => AgentEventKindDto::OutboxQueued,
        AgentEventKind::OutboxDelivered => AgentEventKindDto::OutboxDelivered,
        AgentEventKind::OutboxFailed => AgentEventKindDto::OutboxFailed,
        AgentEventKind::HumanIntervened => AgentEventKindDto::HumanIntervened,
    }
}

pub(super) fn outbox_status_to_dto(status: OutboxMessageStatus) -> AgentOutboxMessageStatusDto {
    match status {
        OutboxMessageStatus::Pending => AgentOutboxMessageStatusDto::Pending,
        OutboxMessageStatus::Claimed => AgentOutboxMessageStatusDto::Sending,
        OutboxMessageStatus::Failed => AgentOutboxMessageStatusDto::Failed,
        OutboxMessageStatus::Delivered => AgentOutboxMessageStatusDto::Delivered,
    }
}

pub(super) fn run_workspace_to_dto(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
    run_id: &RunId,
) -> Result<Option<AgentSchedulerWorkspaceContextDto>, ApiError> {
    let artifacts =
        store.list_task_artifacts(project_path, task_id).map_err(map_scheduler_error)?;
    if let Some(artifact) = artifacts.iter().rev().find(|artifact| {
        artifact.kind == "execution_workspace" && artifact.run_id.as_ref() == Some(run_id)
    }) {
        return Ok(workspace_context_from_value(&artifact.payload));
    }
    store
        .load_run_workspace_context(project_path, run_id)
        .map_err(map_scheduler_error)
        .map(|workspace| workspace.map(workspace_to_dto))
}

pub(super) fn workspace_to_dto(
    workspace: StoredAgentRunWorkspaceContext,
) -> AgentSchedulerWorkspaceContextDto {
    AgentSchedulerWorkspaceContextDto {
        slot_id: Some(workspace.workspace.slot_id),
        path: workspace.workspace.path,
        branch: Some(workspace.workspace.branch),
    }
}

pub fn workspace_context_from_value(value: &Value) -> Option<AgentSchedulerWorkspaceContextDto> {
    let workspace = value.get("workspace").unwrap_or(value);
    let slot_id = string_value(workspace, "slot_id")
        .or_else(|| string_value(value, "selected_worktree_slot_id"));
    let path = string_value(workspace, "path")
        .or_else(|| string_value(value, "selected_worktree_path"))?;
    let branch = string_value(workspace, "branch")
        .or_else(|| string_value(value, "selected_worktree_branch"));
    Some(AgentSchedulerWorkspaceContextDto { slot_id, path, branch })
}

pub(super) fn run_stage_from_run_status(status: AgentRunStatus) -> Option<&'static str> {
    match status {
        AgentRunStatus::Pending => Some("planning"),
        AgentRunStatus::Leased | AgentRunStatus::Running => Some("running"),
        AgentRunStatus::Succeeded => Some("succeeded"),
        AgentRunStatus::Failed | AgentRunStatus::TimedOut | AgentRunStatus::Stale => Some("failed"),
        AgentRunStatus::Cancelled => Some("canceled"),
    }
}

pub(super) fn run_stage_from_task_status(status: AgentTaskStatus) -> Option<&'static str> {
    match status {
        AgentTaskStatus::Assigned
        | AgentTaskStatus::Dispatched
        | AgentTaskStatus::Acknowledged
        | AgentTaskStatus::Active
        | AgentTaskStatus::Running
        | AgentTaskStatus::Submitted
        | AgentTaskStatus::Revision => Some("running"),
        AgentTaskStatus::Completed => Some("succeeded"),
        AgentTaskStatus::Blocked | AgentTaskStatus::Paused | AgentTaskStatus::Failed => {
            Some("failed")
        }
        AgentTaskStatus::Cancelled | AgentTaskStatus::Archived => Some("canceled"),
        AgentTaskStatus::Draft | AgentTaskStatus::Queued => None,
    }
}

pub fn event_error_message(
    kind: AgentEventKind,
    to_status: Option<AgentTaskStatus>,
    message: &str,
    payload: &Value,
) -> Option<String> {
    string_value(payload, "error_message").or_else(|| string_value(payload, "error")).or_else(
        || {
            let is_error_event =
                kind == AgentEventKind::RunFailed || to_status == Some(AgentTaskStatus::Failed);
            (is_error_event && !message.trim().is_empty()).then(|| message.to_string())
        },
    )
}

pub(super) fn task_id_from_payload(payload: &Value) -> Option<TaskId> {
    payload
        .get("task_id")
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .and_then(|value| TaskId::new(value).ok())
}

pub fn command_kind_key(command: AgentSchedulerCommandKindDto) -> &'static str {
    match command {
        AgentSchedulerCommandKindDto::Start => "start",
        AgentSchedulerCommandKindDto::Retry => "retry",
        AgentSchedulerCommandKindDto::Pause => "pause",
        AgentSchedulerCommandKindDto::Resume => "resume",
        AgentSchedulerCommandKindDto::Cancel => "cancel",
        AgentSchedulerCommandKindDto::Archive => "archive",
        AgentSchedulerCommandKindDto::Promote => "promote",
        AgentSchedulerCommandKindDto::Merge => "merge",
        AgentSchedulerCommandKindDto::Move => "move",
        AgentSchedulerCommandKindDto::UpdateMetadata => "update_metadata",
        AgentSchedulerCommandKindDto::SoftDelete => "soft_delete",
        AgentSchedulerCommandKindDto::Restore => "restore",
        AgentSchedulerCommandKindDto::ImportLegacyTask => "import_legacy_task",
    }
}

pub fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

pub(super) fn reasoning_effort_field(value: &Value) -> Result<Option<ReasoningEffortDto>, ApiError> {
    value
        .get("reasoning_effort")
        .cloned()
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value).map_err(|error| {
                ApiError::bad_request(format!(
                    "invalid task metadata field reasoning_effort: {error}"
                ))
            })
        })
        .transpose()
}

pub(super) fn string_value(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

pub fn string_array_field(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter_map(non_empty_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

pub fn u32_field(value: &Value, key: &str) -> Option<u32> {
    value.get(key).and_then(Value::as_u64).and_then(|value| u32::try_from(value).ok())
}

pub fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

pub fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub fn optional_text(value: Option<String>) -> Option<String> {
    value.as_deref().and_then(non_empty_str).map(str::to_string)
}

pub(super) fn optional_worker_name(value: Option<String>) -> Result<Option<AssigneeId>, ApiError> {
    value.map(AssigneeId::new).transpose().map_err(map_bad_request)
}

pub(super) fn optional_team_id(value: Option<String>) -> Result<Option<TeamId>, ApiError> {
    value.map(TeamId::new).transpose().map_err(map_bad_request)
}

pub fn parse_project_path(value: &str) -> Result<PathBuf, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("project_path must not be empty"));
    }
    Ok(PathBuf::from(trimmed))
}

pub(super) fn non_empty(value: String, field_name: &str) -> Result<String, ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::bad_request(format!("{field_name} must not be empty")));
    }
    Ok(value)
}

pub fn map_bad_request(error: anyhow::Error) -> ApiError {
    ApiError::bad_request(error.to_string())
}

pub(super) fn map_scheduler_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(error.to_string())
}

pub fn map_scheduler_command_error(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("unsupported agent task transition") {
        return ApiError::bad_request(message);
    }
    ApiError::internal(message)
}

pub fn map_task_board_error(error: anyhow::Error) -> ApiError {
    ApiError::bad_request(error.to_string())
}
