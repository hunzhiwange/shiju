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
fn append_task_log_stream_updates_subtask_execution_status() {
    let mut task = crate::app::task::Task::new(1);
    let mut first = crate::app::task::SubTask::new("first".to_string());
    first.id = "SUB-first".to_string();
    let mut second = crate::app::task::SubTask::new("second".to_string());
    second.id = "SUB-second".to_string();
    task.subtasks = vec![first, second];

    let started = crate::app::task::TaskLogStream::SubTaskStarted {
        subtask_id: "SUB-first".to_string(),
        content: "first".to_string(),
        timestamp_ms: 1_000,
    };
    assert!(super::append_task_log_stream(&mut task, &started));
    assert_eq!(task.subtasks[0].status, crate::app::task::SubTaskStatus::Running);
    assert!(!task.subtasks[0].completed);

    let completed = crate::app::task::TaskLogStream::SubTaskCompleted {
        subtask_id: "SUB-first".to_string(),
        timestamp_ms: 2_500,
    };
    assert!(super::append_task_log_stream(&mut task, &completed));
    assert_eq!(task.subtasks[0].status, crate::app::task::SubTaskStatus::Completed);
    assert!(task.subtasks[0].completed);
    assert_eq!(task.subtasks[0].last_execution_duration_ms, Some(1_500));

    let failed = crate::app::task::TaskLogStream::SubTaskFailed {
        subtask_id: "SUB-second".to_string(),
        error: "boom".to_string(),
        timestamp_ms: 4_000,
    };
    assert!(super::append_task_log_stream(&mut task, &failed));
    assert_eq!(task.subtasks[1].status, crate::app::task::SubTaskStatus::Failed);
    assert!(!task.subtasks[1].completed);
}

#[test]
fn start_command_skips_created_in_flight_task() {
    use vw_gateway_client::vw_api_types::agent_scheduler::{
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
fn retry_backoff_minutes_grows_exponentially_and_caps_at_one_day() {
    assert_eq!(super::retry_backoff_minutes(1, 0), 1);
    assert_eq!(super::retry_backoff_minutes(1, 1), 2);
    assert_eq!(super::retry_backoff_minutes(1, 4), 16);
    assert_eq!(super::retry_backoff_minutes(60, 5), 1440);
    assert_eq!(super::retry_backoff_minutes(0, 1), 2);
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
fn truncate_for_ui_counts_unicode_characters_and_preserves_short_values() {
    assert_eq!(super::truncate_for_ui("任务日志", 4), "任务日志");
    assert_eq!(super::truncate_for_ui("任务日志", 2), "任务 ...(已截断，共4字符)");
    assert_eq!(super::truncate_for_ui("abc", 0), " ...(已截断，共3字符)");
}

#[test]
fn format_task_log_stream_for_ui_covers_empty_subtask_and_exit_variants() {
    use crate::app::task::TaskLogStream;

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
fn unmatched_subtask_log_is_rendered_without_mutating_plan_state() {
    let mut task = crate::app::task::Task::new(1);
    let initial_log_count = task.logs.len();
    let log = crate::app::task::TaskLogStream::SubTaskCompleted {
        subtask_id: "missing".to_string(),
        timestamp_ms: 10,
    };

    assert!(!super::append_task_log_stream(&mut task, &log));
    assert_eq!(task.logs.len(), initial_log_count + 1);
    assert_eq!(task.logs.last().map(|entry| entry.message.as_str()), Some("子任务完成: missing"));
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
fn non_git_project_skips_worktree_snapshot_refresh() {
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

    assert_eq!(refresh.units(), 0);
    assert!(!app.task_board_worktree_snapshot_loading);
    assert!(app.task_board_worktree_snapshot.is_none());
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 0);

    let maintain = super::schedule_worktree_pool_maintenance(&mut app, "/tmp/non-git-project", 0);
    assert_eq!(maintain.units(), 0);
    assert!(!app.task_board_worktree_maintenance_in_flight);

    let recycle = super::recycle_assigned_task_worktree_for_removal(&app, "T-1");
    assert_eq!(recycle.units(), 0);
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
    use vw_gateway_client::vw_api_types::id::{ProjectId, TaskId};
    use vw_gateway_client::vw_api_types::task::{
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
fn worktree_recycle_failure_log_explains_gateway_cleanup_ownership() {
    let mut task = crate::app::task::Task::new(1);

    super::record_task_worktree_recycle_after_failure(&mut task, "execution failed");

    assert_eq!(
        task.logs.last().map(|entry| entry.message.as_str()),
        Some("execution failed，等待网关运行时清理 worktree 槽位")
    );
}

#[test]
fn scheduler_dispatch_mode_only_uses_matrix_for_non_local_modes() {
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

    assert!(!super::agent_scheduler_dispatch_mode_uses_matrix(
        AgentSchedulerDispatchModeDto::Local
    ));
    assert!(super::agent_scheduler_dispatch_mode_uses_matrix(
        AgentSchedulerDispatchModeDto::Matrix
    ));
    assert!(super::agent_scheduler_dispatch_mode_uses_matrix(AgentSchedulerDispatchModeDto::Auto));
}

#[test]
fn scheduler_status_mapping_covers_every_builtin_task_status() {
    use crate::app::task::TaskStatus;
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;

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
    use crate::app::task::TaskStatus;
    use vw_gateway_client::vw_api_types::task::TaskPoolWorkflowStatusRoleDto;

    assert_eq!(
        super::agent_scheduler_status_key_for_task_status(TaskStatus::CodeReview, "missing"),
        TaskStatus::CodeReview.to_string_key()
    );
    assert_eq!(
        super::agent_scheduler_status_role_for_task_status(TaskStatus::PrSubmitted, "missing"),
        Some(TaskPoolWorkflowStatusRoleDto::Merging)
    );
    assert_eq!(
        super::agent_scheduler_status_role_for_task_status(TaskStatus::Custom("unknown"), ""),
        None
    );
}

#[test]
fn scheduler_status_metadata_helpers_cover_roles_and_missing_workflow_maps() {
    use crate::app::task::TaskStatus;
    use crate::app::task::workflow::{DEFAULT_TASK_WORKFLOW_ID, StatusRole, task_workflow_by_id};

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

#[test]
fn wake_scheduler_requests_include_pool_queued_and_planning_tasks() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{AgentTaskKindDto, AgentTaskStatusDto};

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
        crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID,
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
    use crate::app::task::{Task, TaskStatus};

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
        crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID
    );
}

#[test]
fn agent_scheduler_request_uses_lifecycle_cache_metadata() {
    use crate::app::task::{Task, TaskStatus, update_runtime_task_lifecycle_cache};
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;
    use vw_gateway_client::vw_api_types::task_lifecycle::{
        TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
    };

    let _cache_lock = crate::app::task::runtime_task_lifecycle_cache_test_lock();
    struct CacheGuard;

    impl Drop for CacheGuard {
        fn drop(&mut self) {
            crate::app::task::reset_runtime_task_lifecycle_cache()
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
    role: vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto,
    next: Option<&str>,
) -> vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusDto {
    use vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto;

    vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusDto {
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
