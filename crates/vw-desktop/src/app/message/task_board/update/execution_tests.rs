#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("execution_tests"));
}

#[test]
fn start_execution_rejects_invalid_source_status() {
    use crate::app::task::{Task, TaskStatus};

    // 来源为 Pool（须先提升到 Pending）时 StartExecution 必须被拒绝，不进入 Running。
    let mut pool_task = Task::new(1);
    assert_eq!(pool_task.status, TaskStatus::Pool);
    assert!(pool_task.start_execution("启动".into()).is_err());
    assert_eq!(pool_task.status, TaskStatus::Pool);
    assert!(pool_task.execution_started_at_ms.is_none());

    // 合法来源 Pending 仍能正常进入 Running。
    let mut pending_task = Task::new(1);
    pending_task.set_status(TaskStatus::Pending);
    assert!(pending_task.start_execution("启动".into()).is_ok());
    assert_eq!(pending_task.status, TaskStatus::Running);
}

#[test]
fn manual_retry_continues_existing_session_only_for_failed_tasks() {
    use crate::app::task::{Task, TaskStatus};

    let mut failed_task = Task::new(1);
    failed_task.status = TaskStatus::Failed;
    assert!(super::should_continue_existing_session_for_manual_retry(&failed_task, true));

    let mut pending_task = Task::new(1);
    pending_task.status = TaskStatus::Pending;
    assert!(!super::should_continue_existing_session_for_manual_retry(&pending_task, true));
    assert!(!super::should_continue_existing_session_for_manual_retry(&failed_task, false));
}

#[test]
fn start_and_stop_execution_toggle_executor_state_and_scan_cursors() {
    let (mut app, _task) = crate::app::App::new();
    app.project_is_git_repo = true;
    app.task_board_executor_running = false;
    app.task_board_last_log_flush_at_ms = 99;
    app.task_board_log_scan_cursor = 3;
    app.task_board_timeout_scan_cursor = 4;
    app.task_board_schedule_scan_cursor = 5;

    let _ = super::update(&mut app, super::TaskBoardMessage::StartExecution);

    assert!(app.task_board_executor_running);
    assert_eq!(app.task_board_last_log_flush_at_ms, 0);
    assert_eq!(app.task_board_log_scan_cursor, 0);
    assert_eq!(app.task_board_timeout_scan_cursor, 0);
    assert_eq!(app.task_board_schedule_scan_cursor, 0);
    assert!(app.task_board_next_scheduler_tick_at_ms > 0);
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);

    let _ = super::update(&mut app, super::TaskBoardMessage::StopExecution);

    assert!(!app.task_board_executor_running);
}

#[test]
fn non_git_project_rejects_task_execution_with_toast() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/non-git-exec".to_string());
    app.project_is_git_repo = false;
    app.task_board_executor_running = false;
    let mut task = Task::new(1);
    task.id = "T-non-git".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let start = super::update(&mut app, super::TaskBoardMessage::StartExecution);
    assert_eq!(start.units(), 1);
    assert!(!app.task_board_executor_running);
    assert!(
        app.active_toast.as_ref().is_some_and(|toast| {
            matches!(toast.kind, crate::app::state::ToastKind::Error)
                && (toast.message.contains("Git") || toast.message.contains("git"))
        }),
        "StartExecution should toast for non-git projects"
    );

    app.active_toast = None;
    let execute = super::update(
        &mut app,
        super::TaskBoardMessage::ExecuteTask { task_id: "T-non-git".to_string() },
    );
    assert_eq!(execute.units(), 1);
    assert!(!app.task_board_executor.is_running("T-non-git"));
    assert_eq!(app.task_board_tasks[0].status, TaskStatus::Pending);
    assert!(
        app.active_toast.as_ref().is_some_and(|toast| {
            matches!(toast.kind, crate::app::state::ToastKind::Error)
                && (toast.message.contains("Git") || toast.message.contains("git"))
        }),
        "ExecuteTask should toast for non-git projects"
    );
}

#[test]
fn execution_and_periodic_ticks_return_without_touching_state_when_disabled() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = false;
    app.project_path = Some("/tmp/vibewindow-disabled-ticks".to_string());

    let _ = super::update(&mut app, super::TaskBoardMessage::ExecutionTick);
    let _ = super::update(&mut app, super::TaskBoardMessage::AutoCodeReviewTick);
    let _ = super::update(&mut app, super::TaskBoardMessage::AutoMergeTick);

    assert!(!app.task_board_executor_running);
    assert_eq!(app.project_path.as_deref(), Some("/tmp/vibewindow-disabled-ticks"));
}

#[test]
fn in_flight_status_tick_filters_project_and_status_then_dedupes_log() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-command-status".to_string());
    let mut task = Task::new(1);
    task.id = "T-merge".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick {
            project_path: "/tmp/other-project".to_string(),
            task_id: "T-merge".to_string(),
        },
    );
    assert_eq!(app.task_board_tasks[0].logs.len(), 1);

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick {
            project_path: "/tmp/vibewindow-command-status".to_string(),
            task_id: "T-merge".to_string(),
        },
    );
    assert_eq!(app.task_board_tasks[0].logs.len(), 1);

    app.task_board_tasks[0].status = TaskStatus::PrSubmitted;
    for _ in 0..2 {
        let _ = super::update(
            &mut app,
            super::TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick {
                project_path: "/tmp/vibewindow-command-status".to_string(),
                task_id: "T-merge".to_string(),
            },
        );
    }

    let matching_logs = app.task_board_tasks[0]
        .logs
        .iter()
        .filter(|entry| entry.message == "Agent Scheduler 合并命令等待返回，正在同步网关状态/日志")
        .count();
    assert_eq!(matching_logs, 1);
}

#[test]
fn simulate_step_finishes_missing_running_task_without_status_change() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = true;
    app.task_board_executor.start_task("missing-task");

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::SimulateStep { task_id: "missing-task".to_string() },
    );

    assert!(!app.task_board_executor.is_running("missing-task"));
}

#[test]
fn auto_merge_tick_requires_project_even_when_executor_is_running() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = true;
    app.project_path = None;

    let _ = super::update(&mut app, super::TaskBoardMessage::AutoMergeTick);

    assert!(app.task_board_executor_running);
    assert!(app.project_path.is_none());
}

#[test]
fn execute_task_without_project_marks_the_task_failed() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    let mut task = Task::new(1);
    task.id = "missing-project".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::ExecuteTask { task_id: "missing-project".to_string() },
    );

    let task = &app.task_board_tasks[0];
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("缺少项目路径"));
}

#[test]
fn manual_execute_deduplicates_agent_scheduler_start_log() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-agent-scheduler-log-dedupe".to_string());
    app.project_is_git_repo = true;

    let mut task = Task::new(1);
    task.id = "T-start-dedupe".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::ExecuteTask { task_id: "T-start-dedupe".to_string() },
    );
    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::ExecuteTask { task_id: "T-start-dedupe".to_string() },
    );

    let count = app.task_board_tasks[0]
        .logs
        .iter()
        .filter(|entry| entry.message == "已发送 Agent Scheduler 启动命令")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn agent_scheduler_wake_response_does_not_reserve_desktop_running_slot() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerWakeResponse;

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-execution-reserve-test".to_string());
    app.task_board_executor_running = true;
    app.task_board_settings.max_concurrent = 1;
    app.task_board_worktree_maintenance_in_flight = true;

    let mut first = Task::new(1);
    first.id = "T-first".to_string();
    first.status = TaskStatus::Pending;
    first.priority = 1;
    let mut second = Task::new(2);
    second.id = "T-second".to_string();
    second.status = TaskStatus::Pending;
    second.priority = 2;
    app.task_board_tasks = vec![first, second];

    let response = AgentSchedulerWakeResponse {
        dispatched: true,
        task_id: Some("T-first".to_string()),
        run_id: Some("run-1".to_string()),
        worker_id: Some("gateway-local".to_string()),
        outbox_message_id: None,
    };
    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(response.clone())),
    );
    assert!(app.task_board_executor.running_tasks.is_empty());

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(response)),
    );
    assert!(
        app.task_board_executor.running_tasks.is_empty(),
        "Agent Scheduler wake is owned by the gateway and must not reserve desktop slots"
    );
}

#[test]
fn wake_agent_scheduler_create_requests_include_pool_and_filter_terminal_tasks() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{AgentTaskKindDto, AgentTaskStatusDto};

    let mut pending = Task::new(9);
    pending.id = "T-pending-register".to_string();
    pending.status = TaskStatus::Pending;
    pending.prompt = "Register this task".to_string();
    pending.order = 4;
    pending.model = "gpt-helper".to_string();
    pending.agent = Some("planner".to_string());
    pending.acp_agent = Some("codex".to_string());

    let mut pool = Task::new(1);
    pool.id = "T-pool".to_string();
    pool.status = TaskStatus::Pool;
    let mut completed = Task::new(1);
    completed.id = "T-completed".to_string();
    completed.status = TaskStatus::Completed;
    let mut archived = Task::new(1);
    archived.id = "T-archived".to_string();
    archived.status = TaskStatus::Archived;

    let requests = super::agent_scheduler_wake_create_task_requests(
        &[pool, pending, completed, archived],
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID,
    );

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].task_id, "T-pool");
    assert_eq!(requests[0].status, Some(AgentTaskStatusDto::Draft));
    let request = &requests[1];
    assert_eq!(request.task_id, "T-pending-register");
    assert_eq!(request.kind, AgentTaskKindDto::Finite);
    assert_eq!(request.status, Some(AgentTaskStatusDto::Queued));
    assert_eq!(request.metadata["priority"], 9);
    assert_eq!(request.metadata["order_no"], 4);
    assert_eq!(request.metadata["model"], "gpt-helper");
    assert_eq!(request.metadata["agent"], "planner");
    assert_eq!(request.metadata["acp_agent"], "codex");
    assert_eq!(request.metadata["prompt"], "Register this task");
    assert_eq!(
        request.metadata["task_board.workflow_id"],
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID
    );
    assert_eq!(request.metadata["task_board.status_key"], "pending");
    assert_eq!(request.metadata["task_board.status_role"], "queued");
    assert_eq!(request.metadata["task_board.status_key_map"]["running"], "running");
    assert_eq!(request.metadata["task_board.status_key_map"]["assigned"], "pending");
    assert_eq!(request.metadata["task_board.status_key_map"]["dispatched"], "pending");
    assert_eq!(request.metadata["task_board.status_key_map"]["acknowledged"], "pending");
    assert_eq!(request.metadata["task_board.status_key_map"]["submitted"], "code_complete");
}

#[test]
fn manual_execute_ignores_task_already_waiting_for_worktree() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-manual-duplicate-test".to_string());
    app.project_is_git_repo = true;

    let mut task = Task::new(1);
    task.id = "T-reserved".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-reserved");

    let _ = super::update(
        &mut app,
        super::TaskBoardMessage::ExecuteTask { task_id: "T-reserved".to_string() },
    );

    assert_eq!(app.task_board_executor.running_tasks, vec!["T-reserved".to_string()]);
    assert!(
        app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| { entry.message.contains("任务已在执行或等待工作区") })
    );
}
