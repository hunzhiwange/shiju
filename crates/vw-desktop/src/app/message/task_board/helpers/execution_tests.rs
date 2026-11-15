// 验证任务执行生命周期、日志推进与合并停滞保护。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("execution_tests"));
}

#[test]
fn task_board_does_not_touch_legacy_task_database() {
    let execution_source = include_str!("execution.rs");
    let update_source = include_str!("../update.rs");
    let execution_results_source = include_str!("../update/execution_results.rs");

    for forbidden in [
        "crate::app::task::load_all_tasks(",
        "crate::app::task::update_task(",
        "write_task_execution_result_log(",
        "save_task_execution_result_artifact(",
        "write_task_code_review_result_log(",
        "save_task_code_review_result_artifact(",
        "备份到 SQLite",
        ".agent_scheduler_import_legacy_tasks(",
    ] {
        assert!(
            !execution_source.contains(forbidden)
                && !update_source.contains(forbidden)
                && !execution_results_source.contains(forbidden),
            "task board must not touch the legacy task database through {forbidden}"
        );
    }
}

#[test]
fn merge_stall_timeout_ignores_queued_pr_submitted_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;

    let mut task = crate::app::task::Task::new(1);
    task.id = "queued-merge".to_string();
    task.status = crate::app::task::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::PrSubmitted);
    assert!(app.task_board_tasks[0].pause_reason.is_none());
}

#[test]
fn merge_stall_timeout_pauses_running_pr_submitted_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = crate::app::task::Task::new(1);
    task.id = "running-merge".to_string();
    task.status = crate::app::task::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-merge".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Paused);
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
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 5;
    app.project_path = Some("/tmp/vibewindow-merge-unlock-dispatch-test".to_string());

    let mut task = crate::app::task::Task::new(1);
    task.id = "running-merge-unlock".to_string();
    task.status = crate::app::task::TaskStatus::PrSubmitted;
    task.last_active_at_ms = 0;
    task.merge_target_branch = Some("main".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-merge-unlock".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Paused);
}

#[test]
fn git_flow_failure_after_success_does_not_auto_retry_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = crate::app::task::Task::new(1);
    task.id = "git-flow-failure".to_string();
    task.status = crate::app::task::TaskStatus::Failed;
    task.last_error =
        Some("任务执行成功，但 Git 流程失败: gateway request failed: 400 Bad Request".to_string());
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Failed);
    assert!(
        !app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| entry.message.contains("自动回推到待执行"))
    );
}

#[test]
fn failed_task_scan_keeps_failed_status_without_retry_or_rebuild() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;
    app.task_board_settings.failed_retry_max_attempts = 10;

    let mut task = crate::app::task::Task::new(1);
    task.id = "failed-stays-failed".to_string();
    task.status = crate::app::task::TaskStatus::Failed;
    task.last_error = Some("still failing".to_string());
    task.retry_count = 2;
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert!(changed_tasks.is_empty());
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Failed);
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
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.running_timeout_minutes = 1;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = crate::app::task::Task::new(1);
    task.id = "running-timeout".to_string();
    task.status = crate::app::task::TaskStatus::Running;
    task.execution_started_at_ms = Some(0);
    task.last_active_at_ms = 0;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running-timeout".to_string()];

    let (changed_tasks, recycle_tasks) =
        super::apply_execution_timeouts(&mut app, web_time::Instant::now());

    assert_eq!(changed_tasks.len(), 1);
    assert!(recycle_tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Running);
    assert!(app.task_board_executor.running_tasks.contains(&"running-timeout".to_string()));
    assert!(app.task_board_tasks[0].logs.iter().any(|entry| entry.message.contains("超时保活")));
}

#[test]
fn running_timeout_second_scan_fails_without_recycling() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.running_timeout_minutes = 1;
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = crate::app::task::Task::new(1);
    task.id = "running-timeout".to_string();
    task.status = crate::app::task::TaskStatus::Running;
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
    assert_eq!(app.task_board_tasks[0].status, crate::app::task::TaskStatus::Failed);
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
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut waiting = crate::app::task::Task::new(1);
    waiting.id = "waiting-review-timeout".to_string();
    waiting.status = crate::app::task::TaskStatus::CodeReview;
    waiting.last_error = Some("模型响应超时".to_string());
    waiting.last_active_at_ms = crate::app::time::now_ms();
    app.task_board_tasks = vec![waiting];

    assert!(super::pick_next_code_review_task(&app, &[]).is_none());

    let mut ready = crate::app::task::Task::new(2);
    ready.id = "ready-review-timeout".to_string();
    ready.status = crate::app::task::TaskStatus::CodeReview;
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
    let (mut app, _task) = crate::app::App::new();

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
    let (mut app, _task) = crate::app::App::new();
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
    use crate::app::task::TaskLogStream;

    let (mut app, _task) = crate::app::App::new();
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
    let (mut app, _task) = crate::app::App::new();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.task_board_worktree_action_log_rx = Some(receiver);

    super::poll_worktree_action_logs(&mut app);

    assert!(app.task_board_worktree_action_logs.is_empty());
    assert!(app.task_board_worktree_action_log_rx.is_some());
}

#[test]
fn flushing_without_running_tasks_resets_log_scan_cursor() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_log_scan_cursor = 9;

    super::flush_running_task_logs(&mut app, web_time::Instant::now());

    assert_eq!(app.task_board_log_scan_cursor, 0);
}

#[test]
fn throttled_log_flush_respects_interval_unless_forced() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = crate::app::task::Task::new(1);
    task.id = "running".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["running".to_string()];
    app.task_board_last_log_flush_at_ms = crate::app::time::now_ms();
    app.task_board_log_scan_cursor = 0;

    super::flush_running_task_logs_throttled(&mut app, false, web_time::Instant::now());
    assert_eq!(app.task_board_log_scan_cursor, 0);

    super::flush_running_task_logs_throttled(&mut app, true, web_time::Instant::now());
    assert_eq!(app.task_board_log_scan_cursor, 0);
    assert!(app.task_board_last_log_flush_at_ms > 0);
}

#[test]
fn worktree_snapshot_refresh_interval_tracks_panel_visibility() {
    let (mut app, _task) = crate::app::App::new();

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
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.task_board_worktree_snapshot_loading = true;
    app.task_board_last_worktree_snapshot_at_ms = 123;

    let _refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert!(app.task_board_worktree_snapshot.is_none());
    assert!(!app.task_board_worktree_snapshot_loading);
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 0);
}

#[test]
fn non_git_project_schedules_logical_pool_refresh() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/non-git-project".to_string());
    app.project_is_git_repo = false;
    app.task_board_worktree_snapshot_loading = false;
    app.task_board_worktree_snapshot = Some(crate::app::task::WorktreePoolSnapshot {
        repo_root: "/tmp/stale".to_string(),
        pool_root: "/tmp/stale/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 0,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: Vec::new(),
        slots: Vec::new(),
    });
    app.task_board_last_worktree_snapshot_at_ms = 99;

    let refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert_eq!(refresh.units(), 1);
    assert!(app.task_board_worktree_snapshot_loading);
    assert!(app.task_board_worktree_snapshot.is_some());
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 99);

    let maintain = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/non-git-project", 0);
    assert_eq!(maintain.units(), 1);
    assert!(app.task_board_worktree_maintenance_in_flight);

    let recycle = super::recycle_assigned_task_worktree_for_removal(&app, "T-1");
    assert_eq!(recycle.units(), 1);
}

#[test]
fn git_project_still_schedules_worktree_snapshot_refresh() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/git-project".to_string());
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot_loading = false;

    let refresh = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);

    assert!(iced_runtime::task::into_stream(refresh.collect()).is_some());
    assert!(app.task_board_worktree_snapshot_loading);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn worktree_snapshot_refresh_skips_loading_and_interval_but_force_schedules() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".to_string());
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot_loading = true;

    let loading = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);
    assert_eq!(loading.units(), 0);

    app.task_board_worktree_snapshot_loading = false;
    app.task_board_last_worktree_snapshot_at_ms = crate::app::time::now_ms();
    let throttled = super::maybe_schedule_worktree_snapshot_refresh(&mut app, false);
    assert_eq!(throttled.units(), 0);

    let forced = super::maybe_schedule_worktree_snapshot_refresh(&mut app, true);
    assert!(iced_runtime::task::into_stream(forced.collect()).is_some());
    assert!(app.task_board_worktree_snapshot_loading);
}

#[test]
fn worktree_maintenance_respects_in_flight_and_manual_action_guards() {
    let (mut app, _task) = crate::app::App::new();
    app.project_is_git_repo = true;
    app.task_board_settings.max_concurrent = 0;

    app.task_board_worktree_maintenance_in_flight = true;
    let in_flight = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert_eq!(in_flight.units(), 0);

    app.task_board_worktree_maintenance_in_flight = false;
    app.task_board_worktree_manual_action_kind = Some("cleanup");
    let manual = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert_eq!(manual.units(), 0);

    app.task_board_worktree_manual_action_kind = None;
    let scheduled = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/project", 0);
    assert!(iced_runtime::task::into_stream(scheduled.collect()).is_some());
    assert!(app.task_board_worktree_maintenance_in_flight);
}

#[test]
fn persistence_and_removal_helpers_stay_idle_without_local_ownership() {
    let (app, _task) = crate::app::App::new();
    let task = crate::app::task::Task::new(1);

    assert!(super::build_persist_tasks("/project", &[task]).is_empty());

    let recycle = super::recycle_assigned_task_worktree_for_removal(&app, "missing-project");
    assert_eq!(recycle.units(), 0);
}

#[test]
fn worktree_snapshot_from_gateway_preserves_counts_locks_and_slot_state() {
    use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TaskId,
};
    use vw_api_task::{
        TaskPoolWorktreeMergeLockDto, TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto,
        TaskPoolWorktreeSnapshotResponse,
    };

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
    assert_eq!(snapshot.slots[0].state, crate::app::task::WorktreeState::Busy);
    assert_eq!(snapshot.slots[0].leased_task_id.as_deref(), Some("task-a"));
    assert_eq!(snapshot.slots[0].taint_reason.as_deref(), Some("dirty"));
}

#[test]
fn timeout_helpers_clamp_configuration_and_recognize_only_timeout_errors() {
    let (mut app, _task) = crate::app::App::new();

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
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
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

#[test]
fn task_pool_desktop_scheduler_prepares_logical_capacity_without_worktrees() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().into_owned();
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some(path.clone());
    app.project_is_git_repo = false;
    app.task_board_settings.max_concurrent = 7;
    let task = super::schedule_worktree_pool_maintenance(&mut app, &path, 0);
    assert_eq!(task.units(), 1);
    assert!(app.task_board_worktree_maintenance_in_flight);
    assert_eq!(app.task_board_settings.max_concurrent, 7);
    assert!(!directory.path().join(".git").exists());
    assert!(!directory.path().join(".worktrees").exists());
}
