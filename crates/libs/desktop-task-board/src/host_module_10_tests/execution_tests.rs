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

#[test]
fn flushing_without_running_tasks_resets_log_scan_cursor() {
    let mut app = super::Fixture {
        task_board_log_scan_cursor: 9,
        ..Default::default()
    };

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
fn missing_project_path_clears_stale_worktree_snapshot_state() {
    let mut app = super::Fixture {
        project_path: None,
        task_board_worktree_snapshot_loading: true,
        task_board_last_worktree_snapshot_at_ms: 123,
        ..Default::default()
    };

    let _refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert!(app.task_board_worktree_snapshot.is_none());
    assert!(!app.task_board_worktree_snapshot_loading);
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 0);
}

#[test]
fn worktree_snapshot_refresh_skips_loading_and_interval_but_force_schedules() {
    let mut app = super::Fixture {
        project_path: Some("/tmp/project".to_string()),
        task_board_worktree_snapshot_loading: true,
        ..Default::default()
    };

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
fn worktree_maintenance_skips_when_maintenance_or_recycle_is_in_flight() {
    let mut app = super::Fixture::default();
    app.task_board_settings.max_concurrent = 0;

    app.task_board_worktree_maintenance_in_flight = true;
    let in_flight = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(!in_flight);

    app.task_board_worktree_maintenance_in_flight = false;
    app.task_board_worktree_slot_recycle_in_flight = true;
    let recycling = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(!recycling);

    app.task_board_worktree_slot_recycle_in_flight = false;
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
fn timeout_helpers_clamp_configuration() {
    let mut app = super::Fixture::default();

    assert_eq!(super::minutes_to_ms(0), 60_000);
    assert_eq!(super::minutes_to_ms(u32::MAX), u64::from(u32::MAX) * 60_000);
    app.task_board_settings.failed_retry_max_attempts = 99;
    assert_eq!(super::failed_retry_max_attempts(&app.task_board_settings), 10);
    app.task_board_settings.failed_retry_max_attempts = 0;
    assert_eq!(super::failed_retry_max_attempts(&app.task_board_settings), 0);
}
