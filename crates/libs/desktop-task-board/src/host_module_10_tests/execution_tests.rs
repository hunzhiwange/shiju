#[test]
fn merge_stall_timeout_ignores_queued_pr_submitted_task() {
    let mut app = super::Fixture::default();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "queued-merge".to_string();
    task.status = desktop_task_model::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::PrSubmitted);
    assert!(app.task_board_tasks[0].pause_reason.is_none());
}

#[test]
fn merge_stall_timeout_pauses_running_pr_submitted_task() {
    let mut app = super::Fixture::default();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "running-merge".to_string();
    task.status = desktop_task_model::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-merge".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Paused);
    assert!(
        app.task_board_tasks[0]
            .pause_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("合并阶段静默超过 300 秒未完成"))
    );
    assert!(app.task_board_tasks[0].logs.iter().any(|entry| {
        entry.message.contains("合并阶段超时暂停")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
    assert!(app.task_board_executor.running_tasks.is_empty());
}

#[test]
fn merge_stall_timeout_waits_for_runtime_cleanup_without_unlock_task() {
    let mut app = super::Fixture::default();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;
    app.project_path = Some("/tmp/vibewindow-merge-unlock-dispatch-test".to_string());

    let mut task = desktop_task_model::Task::new(1);
    task.id = "running-merge-unlock".to_string();
    task.status = desktop_task_model::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    task.merge_target_branch = Some("main".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-merge-unlock".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Paused);
}

#[test]
fn git_flow_failure_after_success_does_not_auto_retry_task() {
    let mut app = super::Fixture::default();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "git-flow-failure".to_string();
    task.status = desktop_task_model::TaskStatus::Failed;
    task.last_error =
        Some("任务执行成功，但 Git 流程失败: gateway request failed: 400 Bad Request".to_string());
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Failed);
    assert!(
        !app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| entry.message.contains("自动回推到待执行"))
    );
}

#[test]
fn failed_task_scan_keeps_failed_status_without_retry_or_rebuild() {
    let mut app = super::Fixture::default();
    app.task_board_settings.failed_retry_minutes = 1;
    app.task_board_settings.failed_retry_max_attempts = 10;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "failed-stays-failed".to_string();
    task.status = desktop_task_model::TaskStatus::Failed;
    task.last_error = Some("still failing".to_string());
    task.retry_count = 2;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Failed);
    assert_eq!(app.task_board_tasks[0].retry_count, 2);
    assert!(
        !app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| entry.message.contains("自动回推到待执行"))
    );
}

#[test]
fn running_timeout_first_scan_keeps_task_running() {
    let mut app = super::Fixture::default();
    app.task_board_settings.running_timeout_minutes = 1;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "running-timeout".to_string();
    task.status = desktop_task_model::TaskStatus::Running;
    task.execution_started_at_ms = Some(0);
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-timeout".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Running);
    assert!(app.task_board_executor.running_tasks.contains(&"running-timeout".to_string()));
    assert!(app.task_board_tasks[0].logs.iter().any(|entry| entry.message.contains("超时保活")));
}

#[test]
fn running_timeout_second_scan_fails_without_recycling() {
    let mut app = super::Fixture::default();
    app.task_board_settings.running_timeout_minutes = 1;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = desktop_task_model::Task::new(1);
    task.id = "running-timeout".to_string();
    task.status = desktop_task_model::TaskStatus::Running;
    task.execution_started_at_ms = Some(0);
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-timeout".to_string()];

    let _ = super::apply_execution_timeouts(&mut app, web_time::Instant::now());
    app.task_board_tasks[0].execution_started_at_ms = Some(0);
    app.task_board_tasks[0].last_active_at_ms = 0;

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, desktop_task_model::TaskStatus::Failed);
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert!(app.task_board_tasks[0].logs.iter().any(|entry| {
        entry.message.contains("任务执行连续超时")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn code_review_model_timeout_retry_waits_before_dispatch() {
    let mut app = super::Fixture::default();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut waiting = desktop_task_model::Task::new(1);
    waiting.id = "waiting-review-timeout".to_string();
    waiting.status = desktop_task_model::TaskStatus::CodeReview;
    waiting.last_error = Some("模型响应超时".to_string());
    waiting.last_active_at_ms = runtime_time::now_ms();
    app.task_board_tasks = vec![waiting];

    assert!(super::pick_next_code_review_task(&app, &[]).is_none());

    let mut ready = desktop_task_model::Task::new(2);
    ready.id = "ready-review-timeout".to_string();
    ready.status = desktop_task_model::TaskStatus::CodeReview;
    ready.last_error = Some("模型响应超时".to_string());
    ready.last_active_at_ms = 0;
    app.task_board_tasks.push(ready);

    assert_eq!(
        super::pick_next_code_review_task(&app, &[]).as_deref(),
        Some("ready-review-timeout")
    );
}

#[test]
fn worktree_action_log_keeps_only_the_newest_bounded_lines() {
    let mut app = super::Fixture::default();

    for index in 0..=super::TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES {
        super::push_worktree_action_log(&mut app, format!("line-{index}"));
    }

    assert_eq!(
        app.task_board_worktree_action_logs.len(),
        super::TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES
    );
    assert_eq!(app.task_board_worktree_action_logs.first().map(String::as_str), Some("line-1"));
    assert_eq!(app.task_board_worktree_action_logs.last().map(String::as_str), Some("line-14"));
}

#[test]
fn clear_and_finish_worktree_action_logs_reset_transient_state() {
    let mut app = super::Fixture::default();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.task_board_worktree_manual_action_kind = Some("cleanup");
    app.task_board_worktree_manual_confirm_kind = Some("cleanup");
    app.task_board_worktree_action_logs = vec!["working".to_string()];
    app.task_board_worktree_action_logs_visible_until_ms = Some(1);
    app.task_board_worktree_action_log_rx = Some(receiver);

    super::clear_worktree_action_logs(&mut app);

    assert!(app.task_board_worktree_manual_action_kind.is_none());
    assert!(app.task_board_worktree_manual_confirm_kind.is_none());
    assert!(app.task_board_worktree_action_logs.is_empty());
    assert!(app.task_board_worktree_action_logs_visible_until_ms.is_none());
    assert!(app.task_board_worktree_action_log_rx.is_none());

    app.task_board_worktree_manual_action_kind = Some("merge");
    app.task_board_worktree_manual_confirm_kind = Some("merge");
    app.task_board_worktree_action_logs.push("done".to_string());
    super::finish_worktree_action_logs(&mut app);

    assert!(app.task_board_worktree_manual_action_kind.is_none());
    assert!(app.task_board_worktree_manual_confirm_kind.is_none());
    assert!(app.task_board_worktree_action_logs_visible_until_ms.is_some());

    app.task_board_worktree_action_logs.clear();
    super::finish_worktree_action_logs(&mut app);
    assert!(app.task_board_worktree_action_logs_visible_until_ms.is_none());
}

#[test]
fn poll_worktree_action_logs_filters_blank_lines_and_clears_disconnected_receiver() {
    use desktop_task_executor::TaskLogStream;

    let mut app = super::Fixture::default();
    let (sender, receiver) = std::sync::mpsc::channel();
    sender.send(TaskLogStream::Stdout("visible".to_string())).expect("log queued");
    sender.send(TaskLogStream::Stderr("   ".to_string())).expect("blank log queued");
    drop(sender);
    app.task_board_worktree_action_log_rx = Some(receiver);

    super::poll_worktree_action_logs(&mut app);

    assert_eq!(app.task_board_worktree_action_logs, vec!["visible"]);
    assert!(app.task_board_worktree_action_log_rx.is_none());
}

#[test]
fn poll_worktree_action_logs_keeps_receiver_when_channel_is_empty() {
    let mut app = super::Fixture::default();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.task_board_worktree_action_log_rx = Some(receiver);

    super::poll_worktree_action_logs(&mut app);

    assert!(app.task_board_worktree_action_logs.is_empty());
    assert!(app.task_board_worktree_action_log_rx.is_some());
}

#[test]
fn flushing_without_running_tasks_resets_log_scan_cursor() {
    let mut app = super::Fixture::default();
    app.task_board_log_scan_cursor = 9;

    super::flush_running_task_logs(&mut app, web_time::Instant::now());

    assert_eq!(app.task_board_log_scan_cursor, 0);
}

#[test]
fn throttled_log_flush_respects_interval_unless_forced() {
    let mut app = super::Fixture::default();
    let mut task = desktop_task_model::Task::new(1);
    task.id = "running".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running".to_string()];
    app.task_board_last_log_flush_at_ms = runtime_time::now_ms();
    app.task_board_log_scan_cursor = 0;

    super::flush_running_task_logs_throttled(&mut app, false, web_time::Instant::now());
    assert_eq!(app.task_board_log_scan_cursor, 0);

    super::flush_running_task_logs_throttled(&mut app, true, web_time::Instant::now());
    assert_eq!(app.task_board_log_scan_cursor, 0);
    assert!(app.task_board_last_log_flush_at_ms > 0);
}

#[test]
fn worktree_snapshot_refresh_interval_tracks_panel_visibility() {
    let mut app = super::Fixture::default();

    app.task_board_worktree_panel_expanded = false;
    assert_eq!(
        super::worktree_snapshot_refresh_interval_ms(&app),
        super::TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS
    );
    app.task_board_worktree_panel_expanded = true;
    assert_eq!(
        super::worktree_snapshot_refresh_interval_ms(&app),
        super::TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS
    );
}

#[test]
fn missing_project_path_clears_stale_worktree_snapshot_state() {
    let mut app = super::Fixture::default();
    app.project_path = None;
    app.task_board_worktree_snapshot_loading = true;
    app.task_board_last_worktree_snapshot_at_ms = 123;

    let _refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert!(app.task_board_worktree_snapshot.is_none());
    assert!(!app.task_board_worktree_snapshot_loading);
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 0);
}

#[test]
fn worktree_snapshot_refresh_skips_loading_and_interval_but_force_schedules() {
    let mut app = super::Fixture::default();
    app.project_path = Some("/tmp/project".to_string());
    app.task_board_worktree_snapshot_loading = true;

    let loading = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);
    assert!(!loading);

    app.task_board_worktree_snapshot_loading = false;
    app.task_board_last_worktree_snapshot_at_ms = runtime_time::now_ms();
    let throttled = super::maybe_schedule_worktree_snapshot_refresh(&mut app, false);
    assert!(!throttled);

    let forced = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);
    assert!(forced);
    assert!(app.task_board_worktree_snapshot_loading);
}

#[test]
fn worktree_maintenance_respects_in_flight_and_manual_action_guards() {
    let mut app = super::Fixture::default();
    app.task_board_settings.max_concurrent = 0;

    app.task_board_worktree_maintenance_in_flight = true;
    let in_flight = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(!in_flight);

    app.task_board_worktree_maintenance_in_flight = false;
    app.task_board_worktree_manual_action_kind = Some("cleanup");
    let manual = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(!manual);

    app.task_board_worktree_manual_action_kind = None;
    let scheduled = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(scheduled);
    assert!(app.task_board_worktree_maintenance_in_flight);
}

#[test]
fn worktree_snapshot_from_gateway_preserves_counts_locks_and_slot_state() {
    use vw_api_task::{
        TaskPoolWorktreeMergeLockDto, TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto,
        TaskPoolWorktreeSnapshotResponse,
    };
    use {vw_api_transport::id::ProjectId, vw_api_transport::id::TaskId};

    let snapshot = super::worktree_snapshot_from_gateway(TaskPoolWorktreeSnapshotResponse {
        project_id: ProjectId::from("project-1"),
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 1,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: vec![TaskPoolWorktreeMergeLockDto {
            target_branch: "main".to_string(),
            task_id: TaskId("task-a".to_string()),
            age_ms: 250,
        }],
        slots: vec![TaskPoolWorktreeSlotDto {
            agent_id: None,
            run_id: None,
            id: "slot-a".to_string(),
            path: "/repo/.worktrees/a".to_string(),
            branch: "feature/a".to_string(),
            state: TaskPoolWorktreeSlotStateDto::Busy,
            leased_task_id: Some(TaskId("task-a".to_string())),
            taint_reason: Some("dirty".to_string()),
        }],
    });

    assert_eq!(snapshot.repo_root, "/repo");
    assert_eq!(snapshot.idle_count, 1);
    assert_eq!(snapshot.busy_count, 1);
    assert_eq!(snapshot.merge_target_locks, vec![("main".to_string(), "task-a".to_string())]);
    assert_eq!(snapshot.slots[0].id, "slot-a");
    assert_eq!(
        snapshot.slots[0].state,
        desktop_task_executor::executor::state::WorktreeState::Busy
    );
    assert_eq!(snapshot.slots[0].leased_task_id.as_deref(), Some("task-a"));
    assert_eq!(snapshot.slots[0].taint_reason.as_deref(), Some("dirty"));
}

#[test]
fn timeout_helpers_clamp_configuration_and_recognize_only_timeout_errors() {
    let mut app = super::Fixture::default();

    assert_eq!(super::minutes_to_ms(0), 60_000);
    assert_eq!(super::minutes_to_ms(u32::MAX), u64::from(u32::MAX) * 60_000);
    app.task_board_settings.failed_retry_max_attempts = 99;
    assert_eq!(super::failed_retry_max_attempts(&app.task_board_settings), 10);
    app.task_board_settings.failed_retry_max_attempts = 0;
    assert_eq!(super::failed_retry_max_attempts(&app.task_board_settings), 0);
    assert!(super::is_model_response_timeout_error("LLM RESPONSE TIMED OUT"));
    assert!(super::is_model_response_timeout_error(
        "request timed out while waiting for the model"
    ));
    assert!(!super::is_model_response_timeout_error("connection refused"));
}

#[test]
fn code_review_selection_honors_exclusions_and_priority_order() {
    use desktop_task_model::{Task, TaskStatus};

    let mut app = super::Fixture::default();
    let mut later = Task::new(5);
    later.id = "later".to_string();
    later.status = TaskStatus::CodeReview;
    later.order = 1;
    let mut first = Task::new(1);
    first.id = "first".to_string();
    first.status = TaskStatus::CodeReview;
    first.order = 8;
    let mut deleted = Task::new(0);
    deleted.id = "deleted".to_string();
    deleted.status = TaskStatus::CodeReview;
    deleted.deleted = true;
    app.task_board_tasks = vec![later, first, deleted];

    assert_eq!(super::pick_next_code_review_task(&app, &[]).as_deref(), Some("first"));
    assert_eq!(
        super::pick_next_code_review_task(&app, &["first".to_string()]).as_deref(),
        Some("later")
    );
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
