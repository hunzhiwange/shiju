#[test]
fn append_task_log_stream_updates_subtask_execution_status() {
    let mut task = desktop_task_model::Task::new(1);
    let mut first = desktop_task_model::SubTask::new("first".to_string());
    first.id = "SUB-first".to_string();
    let mut second = desktop_task_model::SubTask::new("second".to_string());
    second.id = "SUB-second".to_string();
    task.subtasks = vec![first, second];

    let started = desktop_task_executor::TaskLogStream::SubTaskStarted {
        subtask_id: "SUB-first".to_string(),
        content: "first".to_string(),
        timestamp_ms: 1_000,
    };
    assert!(super::append_task_log_stream(&mut task, &started));
    assert_eq!(task.subtasks[0].status, desktop_task_model::SubTaskStatus::Running);
    assert!(!task.subtasks[0].completed);

    let completed = desktop_task_executor::TaskLogStream::SubTaskCompleted {
        subtask_id: "SUB-first".to_string(),
        timestamp_ms: 2_500,
    };
    assert!(super::append_task_log_stream(&mut task, &completed));
    assert_eq!(task.subtasks[0].status, desktop_task_model::SubTaskStatus::Completed);
    assert!(task.subtasks[0].completed);
    assert_eq!(task.subtasks[0].last_execution_duration_ms, Some(1_500));

    let failed = desktop_task_executor::TaskLogStream::SubTaskFailed {
        subtask_id: "SUB-second".to_string(),
        error: "boom".to_string(),
        timestamp_ms: 4_000,
    };
    assert!(super::append_task_log_stream(&mut task, &failed));
    assert_eq!(task.subtasks[1].status, desktop_task_model::SubTaskStatus::Failed);
    assert!(!task.subtasks[1].completed);
}

#[test]
fn start_command_skips_created_in_flight_task() {
    use vw_api_agent_scheduler::{
        AgentSchedulerCommandKindDto, AgentTaskStatusDto,
    };

    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Start,
        &AgentTaskStatusDto::Queued
    ));
    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Start,
        &AgentTaskStatusDto::Draft
    ));

    for status in [
        AgentTaskStatusDto::Assigned,
        AgentTaskStatusDto::Dispatched,
        AgentTaskStatusDto::Acknowledged,
        AgentTaskStatusDto::Running,
        AgentTaskStatusDto::Submitted,
    ] {
        assert!(!super::should_send_agent_scheduler_command_after_create(
            AgentSchedulerCommandKindDto::Start,
            &status
        ));
    }

    assert!(super::should_send_agent_scheduler_command_after_create(
        AgentSchedulerCommandKindDto::Retry,
        &AgentTaskStatusDto::Running
    ));
}

#[test]
fn retry_backoff_minutes_grows_exponentially_and_caps_at_one_day() {
    assert_eq!(super::retry_backoff_minutes(1, 0), 1);
    assert_eq!(super::retry_backoff_minutes(1, 1), 2);
    assert_eq!(super::retry_backoff_minutes(1, 4), 16);
    assert_eq!(super::retry_backoff_minutes(60, 5), 1440);
    assert_eq!(super::retry_backoff_minutes(0, 1), 2);
}

#[test]
fn gateway_idle_timeout_is_treated_as_model_timeout_retry() {
    assert!(super::is_model_response_timeout_error(
        "gateway 调度失败: gateway stream idle timeout after 60s without response"
    ));
    assert!(super::is_model_response_timeout_error(
        "gateway 响应超时；已在同一会话发送重试但仍失败"
    ));
}

#[test]
fn truncate_for_ui_counts_unicode_characters_and_preserves_short_values() {
    assert_eq!(super::truncate_for_ui("任务日志", 4), "任务日志");
    assert_eq!(super::truncate_for_ui("任务日志", 2), "任务 ...(已截断，共4字符)");
    assert_eq!(super::truncate_for_ui("abc", 0), " ...(已截断，共3字符)");
}

#[test]
fn format_task_log_stream_for_ui_covers_empty_subtask_and_exit_variants() {
    use desktop_task_executor::TaskLogStream;

    assert_eq!(
        super::format_task_log_stream_for_ui(&TaskLogStream::Stdout("  \n".into()), 20),
        None
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(&TaskLogStream::Stderr("abcdef".into()), 3),
        Some("abc ...(已截断，共6字符)".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::SubTaskStarted {
                subtask_id: "sub-1".into(),
                content: "prepare".into(),
                timestamp_ms: 1,
            },
            100,
        ),
        Some("子任务开始: prepare".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::SubTaskCompleted { subtask_id: "sub-1".into(), timestamp_ms: 2 },
            1,
        ),
        Some("子任务完成: sub-1".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::SubTaskFailed {
                subtask_id: "sub-1".into(),
                error: "boom".into(),
                timestamp_ms: 3,
            },
            100,
        ),
        Some("子任务失败: sub-1 boom".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: true, code: Some(0), signal: None },
            1,
        ),
        Some("执行成功 code=Some(0)".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: false, code: None, signal: Some(9) },
            1,
        ),
        Some("执行失败 code=None signal=9".to_string())
    );
    assert_eq!(
        super::format_task_log_stream_for_ui(
            &TaskLogStream::ExitStatus { success: false, code: Some(2), signal: None },
            1,
        ),
        Some("执行失败 code=Some(2)".to_string())
    );
}

#[test]
fn unmatched_subtask_log_is_rendered_without_mutating_plan_state() {
    let mut task = desktop_task_model::Task::new(1);
    let initial_log_count = task.logs.len();
    let log = desktop_task_executor::TaskLogStream::SubTaskCompleted {
        subtask_id: "missing".to_string(),
        timestamp_ms: 10,
    };

    assert!(!super::append_task_log_stream(&mut task, &log));
    assert_eq!(task.logs.len(), initial_log_count + 1);
    assert_eq!(task.logs.last().map(|entry| entry.message.as_str()), Some("子任务完成: missing"));
}

#[test]
fn worktree_recycle_failure_log_explains_gateway_cleanup_ownership() {
    let mut task = desktop_task_model::Task::new(1);

    super::record_task_worktree_recycle_after_failure(&mut task, "execution failed");

    assert_eq!(
        task.logs.last().map(|entry| entry.message.as_str()),
        Some("execution failed，等待网关运行时清理 worktree 槽位")
    );
}

#[test]
fn scheduler_status_mapping_covers_every_builtin_task_status() {
    use desktop_task_model::TaskStatus;
    use vw_api_agent_scheduler::AgentTaskStatusDto;

    for (status, expected) in [
        (TaskStatus::Pool, AgentTaskStatusDto::Draft),
        (TaskStatus::Pending, AgentTaskStatusDto::Queued),
        (TaskStatus::Planning, AgentTaskStatusDto::Queued),
        (TaskStatus::Running, AgentTaskStatusDto::Running),
        (TaskStatus::Failed, AgentTaskStatusDto::Failed),
        (TaskStatus::Paused, AgentTaskStatusDto::Paused),
        (TaskStatus::CodeComplete, AgentTaskStatusDto::Submitted),
        (TaskStatus::CodeReview, AgentTaskStatusDto::Submitted),
        (TaskStatus::PrSubmitted, AgentTaskStatusDto::Submitted),
        (TaskStatus::Completed, AgentTaskStatusDto::Completed),
        (TaskStatus::Archived, AgentTaskStatusDto::Archived),
    ] {
        assert_eq!(super::agent_scheduler_status_for_task_status(status, ""), expected);
    }
    assert_eq!(
        super::agent_scheduler_status_for_task_status(TaskStatus::Custom("unknown"), "missing"),
        AgentTaskStatusDto::Queued
    );
}

#[test]
fn scheduler_status_key_and_role_fall_back_to_default_workflow() {
    use desktop_task_model::TaskStatus;
    use vw_api_task::TaskPoolWorkflowStatusRoleDto;

    assert_eq!(
        super::agent_scheduler_status_key_for_task_status(TaskStatus::CodeReview, "missing"),
        TaskStatus::CodeReview.to_string_key()
    );
    assert_eq!(
        super::agent_scheduler_status_role_for_task_status(TaskStatus::PrSubmitted, "missing"),
        None
    );
    assert_eq!(
        super::agent_scheduler_status_role_for_task_status(
            TaskStatus::PrSubmitted,
            "dev_lifecycle"
        ),
        Some(TaskPoolWorkflowStatusRoleDto::Merging)
    );
    assert_eq!(
        super::agent_scheduler_status_role_for_task_status(TaskStatus::Custom("unknown"), ""),
        None
    );
}

#[test]
fn scheduler_status_metadata_helpers_cover_roles_and_missing_workflow_maps() {
    use desktop_task_model::TaskStatus;
    use desktop_task_model::workflow::{DEFAULT_TASK_WORKFLOW_ID, StatusRole, task_workflow_by_id};

    let workflow = task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID).expect("default workflow");
    assert_eq!(super::status_role_metadata_key(StatusRole::Backlog), "backlog");
    assert_eq!(super::status_role_metadata_key(StatusRole::Custom), "custom");
    assert_eq!(
        super::status_key_with_role(&workflow, StatusRole::Done).as_deref(),
        Some("completed")
    );
    assert_eq!(
        super::status_key_for_agent_status(
            &workflow,
            TaskStatus::Running,
            &[StatusRole::Executing]
        )
        .as_deref(),
        Some("running")
    );
    assert!(super::agent_scheduler_status_key_map(None, TaskStatus::Running).is_empty());
}

#[test]
fn wake_scheduler_requests_include_pool_queued_and_planning_tasks() {
    use desktop_task_model::{Task, TaskStatus};
    use vw_api_agent_scheduler::{AgentTaskKindDto, AgentTaskStatusDto};

    let mut pending = Task::new(1);
    pending.id = "pending".to_string();
    pending.status = TaskStatus::Pending;
    let mut planning = Task::new(2);
    planning.id = "planning".to_string();
    planning.status = TaskStatus::Planning;
    let mut running = Task::new(3);
    running.id = "running".to_string();
    running.status = TaskStatus::Running;
    let mut pool = Task::new(4);
    pool.id = "pool".to_string();
    pool.status = TaskStatus::Pool;

    let requests = super::agent_scheduler_wake_create_task_requests(
        &[pool, pending, planning, running],
        desktop_task_model::workflow::DEFAULT_TASK_WORKFLOW_ID,
    );

    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].task_id, "pool");
    assert_eq!(requests[0].status, Some(AgentTaskStatusDto::Draft));
    assert_eq!(requests[1].task_id, "pending");
    assert_eq!(requests[2].task_id, "planning");
    assert!(requests.iter().all(|request| request.kind == AgentTaskKindDto::Finite));
    assert!(
        requests[1..].iter().all(|request| { request.status == Some(AgentTaskStatusDto::Queued) })
    );
}

#[test]
fn scheduler_create_request_uses_prompt_title_and_saturates_timestamp() {
    use desktop_task_model::{Task, TaskStatus};

    let mut task = Task::new(7);
    task.id = "request-task".to_string();
    task.status = TaskStatus::Pending;
    task.prompt = format!("\n  {}  \nignored", "界".repeat(130));
    task.created_at_ms = u64::MAX;

    let request = super::agent_scheduler_create_task_request(&task, "");

    assert_eq!(request.title.chars().count(), 120);
    assert!(request.title.chars().all(|character| character == '界'));
    assert_eq!(request.spec_uri, "task-board/request-task/spec.md");
    assert_eq!(request.meta_uri, "task-board/request-task/meta.json");
    assert_eq!(request.created_at_ms.0, i64::MAX);

    task.prompt = " \n\t".to_string();
    let fallback = super::agent_scheduler_create_task_request(&task, "unknown-workflow");
    assert_eq!(fallback.title, "request-task");
    assert_eq!(
        fallback.metadata["task_board.workflow_id"],
        desktop_task_model::workflow::DEFAULT_TASK_WORKFLOW_ID
    );
}

#[test]
fn agent_scheduler_request_uses_lifecycle_cache_metadata() {
    use desktop_task_model::{Task, TaskStatus, update_runtime_task_lifecycle_cache};
    use vw_api_agent_scheduler::AgentTaskStatusDto;
    use vw_api_task_lifecycle::{
        TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
    };

    let _cache_lock = desktop_task_model::runtime_task_lifecycle_cache_test_lock();
    struct CacheGuard;

    impl Drop for CacheGuard {
        fn drop(&mut self) {
            desktop_task_model::reset_runtime_task_lifecycle_cache()
                .expect("runtime lifecycle cache should reset");
        }
    }

    let _guard = CacheGuard;
    update_runtime_task_lifecycle_cache(vec![TaskLifecycleWorkflowDto {
        id: "scheduler_ops".to_string(),
        label: "Scheduler Ops".to_string(),
        built_in: false,
        statuses: vec![
            lifecycle_status(
                "ops_backlog",
                "Ops Backlog",
                TaskLifecycleStatusRoleDto::Backlog,
                Some("ops_ready"),
            ),
            lifecycle_status(
                "ops_ready",
                "Ops Ready",
                TaskLifecycleStatusRoleDto::Queued,
                Some("ops_active"),
            ),
            lifecycle_status(
                "ops_active",
                "Ops Active",
                TaskLifecycleStatusRoleDto::Executing,
                Some("ops_review"),
            ),
            lifecycle_status(
                "ops_review",
                "Ops Review",
                TaskLifecycleStatusRoleDto::ReviewReady,
                Some("ops_done"),
            ),
            lifecycle_status(
                "ops_blocked",
                "Ops Blocked",
                TaskLifecycleStatusRoleDto::Failed,
                Some("ops_ready"),
            ),
            lifecycle_status("ops_done", "Ops Done", TaskLifecycleStatusRoleDto::Done, None),
        ],
    }])
    .expect("runtime lifecycle cache should update");

    let mut task = Task::new(42);
    task.id = "task-cache-metadata".to_string();
    task.status = TaskStatus::Custom("ops_active");
    task.prompt = "Use cache metadata".to_string();

    let request = super::agent_scheduler_create_task_request(&task, "scheduler_ops");
    let metadata = request.metadata.as_object().expect("metadata should be an object");
    let status_key_map = metadata["task_board.status_key_map"]
        .as_object()
        .expect("status key map should be an object");

    assert_eq!(request.status, Some(AgentTaskStatusDto::Running));
    assert_eq!(metadata["task_board.workflow_id"], "scheduler_ops");
    assert_eq!(metadata["task_board.status_key"], "ops_active");
    assert_eq!(metadata["task_board.status_role"], "executing");
    assert_eq!(status_key_map["draft"], "ops_backlog");
    assert_eq!(status_key_map["queued"], "ops_ready");
    assert_eq!(status_key_map["assigned"], "ops_ready");
    assert_eq!(status_key_map["dispatched"], "ops_ready");
    assert_eq!(status_key_map["acknowledged"], "ops_ready");
    assert_eq!(status_key_map["running"], "ops_active");
    assert_eq!(status_key_map["submitted"], "ops_review");
    assert_eq!(status_key_map["failed"], "ops_blocked");
    assert_eq!(status_key_map["completed"], "ops_done");
}

fn lifecycle_status(
    key: &str,
    label: &str,
    role: vw_api_task_lifecycle::TaskLifecycleStatusRoleDto,
    next: Option<&str>,
) -> vw_api_task_lifecycle::TaskLifecycleStatusDto {
    use vw_api_task_lifecycle::TaskLifecycleStatusRoleDto;

    vw_api_task_lifecycle::TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal: next.is_none(),
        auto_execution_candidate: !matches!(
            role,
            TaskLifecycleStatusRoleDto::Done | TaskLifecycleStatusRoleDto::Archived
        ),
        occupies_execution_slot: matches!(
            role,
            TaskLifecycleStatusRoleDto::Planning | TaskLifecycleStatusRoleDto::Executing
        ),
        is_scheduling_backlog: matches!(
            role,
            TaskLifecycleStatusRoleDto::Backlog
                | TaskLifecycleStatusRoleDto::Queued
                | TaskLifecycleStatusRoleDto::Planning
        ),
        next: next.map(str::to_string),
        transitions: Vec::new(),
    }
}
