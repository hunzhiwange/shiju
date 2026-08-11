#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("update_tests"));
}

#[test]
fn project_match_requires_the_same_present_path() {
    let (mut app, _task) = crate::app::App::new();

    assert!(!super::task_board_project_matches(&app, "/workspace"));
    app.project_path = Some("/workspace".to_string());
    assert!(super::task_board_project_matches(&app, "/workspace"));
    assert!(!super::task_board_project_matches(&app, "/other"));
}

#[test]
fn staff_binding_and_default_detection_normalize_blank_and_named_values() {
    assert_eq!(super::normalize_staff_binding(None), None);
    assert_eq!(super::normalize_staff_binding(Some(" \n ")), None);
    assert_eq!(
        super::normalize_staff_binding(Some("  specialist  ")),
        Some("specialist".to_string())
    );

    assert!(super::task_agent_uses_default(None));
    assert!(super::task_agent_uses_default(Some("  main  ")));
    assert!(!super::task_agent_uses_default(Some("specialist")));
    assert!(super::task_acp_agent_uses_default(None));
    assert!(super::task_acp_agent_uses_default(Some("  ")));
    assert!(!super::task_acp_agent_uses_default(Some("codex")));
}

#[test]
fn capability_summary_helpers_trim_deduplicate_and_fallback() {
    let mut summary = crate::app::task::TaskBoardStaffCapabilitySummary::default();
    assert_eq!(super::summary_description_or_contract(&summary, "fallback"), "fallback");
    summary.description = "  explicit description  ".to_string();
    assert_eq!(
        super::summary_description_or_contract(&summary, "fallback"),
        "explicit description"
    );
    assert_eq!(
        super::normalize_capability_summary_fields(vec![
            " first ".to_string(),
            "".to_string(),
            "first".to_string(),
            "second".to_string(),
        ]),
        vec!["first".to_string(), "second".to_string()]
    );
    assert_eq!(super::generated_workflow_field_label("task_summary"), "task summary");
}

#[test]
fn staff_start_request_recognizes_localized_and_ascii_actions() {
    for input in ["请执行任务", "现在开始", "START task", "run task"] {
        assert!(super::staff_input_requests_start(input), "input={input}");
    }
    assert!(!super::staff_input_requests_start("只查看任务"));
}

fn ensure_task_board_project_path(app: &mut crate::app::App) -> String {
    if let Some(project_path) = &app.project_path {
        return project_path.clone();
    }

    let project_path = "/tmp/vibewindow-task-board-update-test".to_string();
    app.project_path = Some(project_path.clone());
    project_path
}

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[test]
fn create_task_uses_first_status_for_selected_workflow() {
    use crate::app::task::TaskStatus;

    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();

    assert_eq!(super::initial_task_status_for_app(&app), TaskStatus::Custom("campaign_ideas"));
}

#[test]
fn task_pool_status_mapping_covers_builtin_and_custom_statuses() {
    use crate::app::task::TaskStatus;
    use vw_gateway_client::vw_api_types::task::TaskPoolStatus;

    let cases = [
        (TaskStatus::Pool, TaskPoolStatus::Pool),
        (TaskStatus::Pending, TaskPoolStatus::Pending),
        (TaskStatus::Planning, TaskPoolStatus::Planning),
        (TaskStatus::Running, TaskPoolStatus::Running),
        (TaskStatus::Failed, TaskPoolStatus::Failed),
        (TaskStatus::Paused, TaskPoolStatus::Paused),
        (TaskStatus::CodeComplete, TaskPoolStatus::CodeComplete),
        (TaskStatus::CodeReview, TaskPoolStatus::CodeReview),
        (TaskStatus::PrSubmitted, TaskPoolStatus::PrSubmitted),
        (TaskStatus::Completed, TaskPoolStatus::Completed),
        (TaskStatus::Archived, TaskPoolStatus::Archived),
        (TaskStatus::Custom("campaign_ideas"), TaskPoolStatus::Pool),
    ];

    for (status, expected) in cases {
        assert_eq!(super::task_pool_status_for_gateway(status), expected);
    }
}

#[test]
fn scheduler_status_key_mapping_trims_aliases_and_preserves_custom_keys() {
    use crate::app::task::TaskStatus;

    let cases = [
        (" draft ", Some(TaskStatus::Pool)),
        ("queued", Some(TaskStatus::Pending)),
        ("active", Some(TaskStatus::Pending)),
        ("assigned", Some(TaskStatus::Pending)),
        ("dispatched", Some(TaskStatus::Pending)),
        ("acknowledged", Some(TaskStatus::Pending)),
        ("submitted", Some(TaskStatus::CodeReview)),
        ("revision", Some(TaskStatus::CodeReview)),
        ("blocked", Some(TaskStatus::Paused)),
        ("cancelled", Some(TaskStatus::Archived)),
        ("campaign_ideas", Some(TaskStatus::Custom("campaign_ideas"))),
        ("   ", None),
    ];

    for (status_key, expected) in cases {
        assert_eq!(super::task_status_from_agent_scheduler_key(status_key), expected);
    }
}

#[test]
fn scheduler_status_mapping_distinguishes_submitted_snapshots() {
    use crate::app::task::TaskStatus;
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;

    let cases = [
        (AgentTaskStatusDto::Draft, TaskStatus::Pool),
        (AgentTaskStatusDto::Queued, TaskStatus::Pending),
        (AgentTaskStatusDto::Assigned, TaskStatus::Pending),
        (AgentTaskStatusDto::Dispatched, TaskStatus::Pending),
        (AgentTaskStatusDto::Acknowledged, TaskStatus::Pending),
        (AgentTaskStatusDto::Active, TaskStatus::Pending),
        (AgentTaskStatusDto::Running, TaskStatus::Running),
        (AgentTaskStatusDto::Submitted, TaskStatus::CodeReview),
        (AgentTaskStatusDto::Revision, TaskStatus::CodeReview),
        (AgentTaskStatusDto::Blocked, TaskStatus::Paused),
        (AgentTaskStatusDto::Paused, TaskStatus::Paused),
        (AgentTaskStatusDto::Completed, TaskStatus::Completed),
        (AgentTaskStatusDto::Failed, TaskStatus::Failed),
        (AgentTaskStatusDto::Cancelled, TaskStatus::Archived),
        (AgentTaskStatusDto::Archived, TaskStatus::Archived),
    ];

    for (status, expected) in cases {
        assert_eq!(super::task_status_from_agent_scheduler_status(status), expected);
    }
    assert_eq!(
        super::task_status_from_agent_scheduler_snapshot_status(AgentTaskStatusDto::Submitted),
        TaskStatus::CodeComplete
    );
    assert_eq!(
        super::task_status_from_agent_scheduler_snapshot_status(AgentTaskStatusDto::Revision),
        TaskStatus::CodeReview
    );
}

#[test]
fn scheduler_projection_helpers_cover_stale_pending_and_running_stage() {
    use crate::app::task::TaskStatus;

    assert!(super::status_key_is_stale_pending_projection(
        TaskStatus::Pending,
        TaskStatus::Running
    ));
    assert!(!super::status_key_is_stale_pending_projection(
        TaskStatus::Pending,
        TaskStatus::Pending
    ));
    assert!(!super::status_key_is_stale_pending_projection(
        TaskStatus::Running,
        TaskStatus::Completed
    ));

    assert!(super::run_stage_is_running(Some(" running ")));
    assert!(!super::run_stage_is_running(Some("Running")));
    assert!(!super::run_stage_is_running(Some("queued")));
    assert!(!super::run_stage_is_running(None));
}

#[test]
fn gateway_numeric_fields_clamp_invalid_values() {
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    assert_eq!(super::timestamp_ms_to_u64(TimestampMs(-1)), 0);
    assert_eq!(super::timestamp_ms_to_u64(TimestampMs(42)), 42);
    assert_eq!(super::runtime_order_no_to_task_order(-1), 0);
    assert_eq!(super::runtime_order_no_to_task_order(42), 42);
    assert_eq!(super::runtime_order_no_to_task_order(i64::MAX), u32::MAX);
}

#[test]
fn workflow_variable_fields_are_sanitized_deduplicated_and_reserved_inputs_are_rejected() {
    let mut fields = Vec::new();

    super::push_unique_workflow_field(&mut fields, " release notes ");
    super::push_unique_workflow_field(&mut fields, "release_notes");
    super::push_unique_workflow_field(&mut fields, "42%");
    super::push_unique_workflow_field(&mut fields, "workflow_uuid");
    super::push_unique_workflow_field(&mut fields, "application_uuid");
    super::push_unique_workflow_field(&mut fields, "---");

    assert_eq!(fields, vec!["release_notes", "field_42"]);
    assert_eq!(super::sanitize_workflow_variable_name(" a/b.c "), "a_b_c");
    assert!(super::is_reserved_workflow_runtime_input(" workflow_uuid "));
    assert!(!super::is_reserved_workflow_runtime_input("workflow_id"));
}

#[test]
fn optional_ids_and_member_toggles_normalize_boundary_inputs() {
    assert_eq!(
        super::normalize_optional_member_id(Some(" staff-planner ".to_string())),
        Some("staff-planner".to_string())
    );
    assert_eq!(super::normalize_optional_member_id(Some("  ".to_string())), None);
    assert_eq!(super::normalize_optional_workflow_uuid(None), None);
    assert_eq!(
        super::normalize_optional_capability_id(Some(" review ".to_string())),
        Some("review".to_string())
    );

    let mut member_ids = vec!["staff-reviewer".to_string()];
    assert!(!super::toggle_member_id(&mut member_ids, "  "));
    assert!(super::toggle_member_id(&mut member_ids, " staff-planner "));
    assert_eq!(member_ids, vec!["staff-reviewer", "staff-planner"]);
    assert!(!super::toggle_member_id(&mut member_ids, "staff-reviewer"));
    assert_eq!(member_ids, vec!["staff-planner"]);
}

#[test]
fn staff_command_member_input_replaces_partial_mentions_and_preserves_actions() {
    assert_eq!(super::staff_command_input_with_member("", " Planner "), "@Planner 执行 ");
    assert_eq!(super::staff_command_input_with_member("请 @Pla", "Reviewer"), "请 @Reviewer 执行 ");
    assert_eq!(
        super::staff_command_input_with_member("@Pla start T-507", "Planner"),
        "@Planner start T-507"
    );
    assert_eq!(
        super::staff_command_input_with_member("前缀 @Pla T-507", "Planner"),
        "前缀 @Planner 执行 T-507"
    );
    assert!(super::staff_command_tail_starts_with_action(" RUN T-507"));
    assert!(!super::staff_command_tail_starts_with_action("T-507"));
}

#[tokio::test(flavor = "multi_thread")]
async fn scheduler_tick_does_not_emit_run_staff_auto_schedule() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = crate::app::state::TaskBoardViewMode::Staff;
    app.task_board_next_scheduler_tick_at_ms = 0;

    let task = super::schedule_scheduler_tick_with_deadline(&mut app);
    let messages = collect_task_messages(task).await;

    assert!(
        messages.iter().all(|message| {
            !matches!(
                message,
                crate::app::Message::TaskBoard(
                    super::TaskBoardMessage::StaffAutoSchedulePlanLoaded { .. }
                        | super::TaskBoardMessage::SubmitStaffCommandSilently(_)
                )
            )
        }),
        "scheduler tick emitted staff auto schedule messages: {messages:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn drop_on_status_does_not_write_local_store_when_gateway_is_unavailable() {
    use crate::app::Message;
    use crate::app::task::{Task, TaskStatus};
    use iced_runtime::Action;
    use iced_runtime::futures::futures::StreamExt;

    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

    async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
        let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
            return Vec::new();
        };
        while let Some(action) = stream.next().await {
            if let Action::Output(messages) = action {
                return messages;
            }
        }
        Vec::new()
    }

    let temp = tempfile::TempDir::new().expect("temp dir");
    let project = temp.path().to_string_lossy().to_string();
    let mut task = Task::new(1);
    task.id = "T-gateway-unavailable".to_string();
    task.status = TaskStatus::Pool;

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.clone());
    app.task_board_tasks = vec![task.clone()];

    app.task_board_dragging = Some((task.id.clone(), TaskStatus::Pool));
    let drop_task = super::update(
        &mut app,
        super::TaskBoardMessage::DropOnStatus {
            to_status: TaskStatus::Completed,
            insert_index: None,
        },
    );
    for message in collect_task_messages(drop_task).await {
        if let Message::TaskBoard(task_message) = message {
            let _ = super::update(&mut app, task_message);
        }
    }

    let preserved = app
        .task_board_tasks
        .iter()
        .find(|candidate| candidate.id == task.id)
        .expect("task preserved");
    assert_eq!(preserved.status, TaskStatus::Pool);
    assert!(
        !app.notifications.is_empty(),
        "gateway failure should surface an error without local writes: {:?}",
        app.notifications
    );
}

#[test]
fn execution_messages_are_delegated_to_execution_module() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_next_scheduler_tick_at_ms = 0;
    app.task_board_next_auto_review_tick_at_ms = 0;
    app.task_board_next_auto_merge_tick_at_ms = 0;

    let _task = super::update(&mut app, super::TaskBoardMessage::StartExecution);

    assert!(app.task_board_executor_running);
    assert!(app.task_board_next_scheduler_tick_at_ms > 0);
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);
    assert_eq!(app.task_board_next_auto_merge_tick_at_ms, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn move_task_status_completed_pending_wakes_running_executor() {
    use crate::app::Message;
    use crate::app::task::TaskStatus;
    use iced_runtime::Action;
    use iced_runtime::futures::futures::StreamExt;

    async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
        let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
            return Vec::new();
        };
        while let Some(action) = stream.next().await {
            if let Action::Output(messages) = action {
                return messages;
            }
        }
        Vec::new()
    }

    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = true;

    let task = super::update(
        &mut app,
        super::TaskBoardMessage::MoveTaskStatusCompleted {
            target_status: TaskStatus::Pending,
            result: Ok(()),
        },
    );

    let messages = collect_task_messages(task).await;
    assert!(
        messages.iter().any(|message| matches!(
            message,
            Message::TaskBoard(super::TaskBoardMessage::LoadTasks)
        ))
    );
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::TaskBoard(super::TaskBoardMessage::ExecutionTick)
    )));
}

#[test]
fn settings_dispatch_mode_message_updates_task_board_setting() {
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

    let (mut app, _task) = crate::app::App::new();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SetAgentSchedulerDispatchMode(
            AgentSchedulerDispatchModeDto::Matrix,
        ),
    );
    assert_eq!(
        app.task_board_settings.agent_scheduler_dispatch_mode,
        AgentSchedulerDispatchModeDto::Matrix
    );

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SetAgentSchedulerDispatchMode(AgentSchedulerDispatchModeDto::Auto),
    );
    assert_eq!(
        app.task_board_settings.agent_scheduler_dispatch_mode,
        AgentSchedulerDispatchModeDto::Auto
    );
}

#[test]
fn project_workflow_switch_refreshes_staff_projection() {
    use crate::app::state::TaskBoardViewMode;
    use crate::app::task::{TaskBoardSettings, TaskBoardStaffConfig};

    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let mut settings = TaskBoardSettings::new();
    settings.workflow_id = "marketing_campaign".to_string();
    let _task = super::update(&mut app, super::TaskBoardMessage::SettingsUpdated(settings));

    let marketing_statuses = app
        .task_board_staff_projection
        .lanes
        .iter()
        .map(|lane| lane.status.to_string_key())
        .collect::<Vec<_>>();
    assert_eq!(
        marketing_statuses,
        vec![
            "campaign_ideas",
            "campaign_queued",
            "campaign_planning",
            "campaign_running",
            "campaign_blocked",
            "campaign_paused",
            "campaign_review_ready",
            "campaign_reviewing",
            "campaign_closing",
            "campaign_done",
            "campaign_archived",
        ]
    );

    let mut settings = app.task_board_settings.clone();
    settings.workflow_id = crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string();
    let _task = super::update(&mut app, super::TaskBoardMessage::SettingsUpdated(settings));

    let dev_statuses = app
        .task_board_staff_projection
        .lanes
        .iter()
        .map(|lane| lane.status.to_string_key())
        .collect::<Vec<_>>();
    assert_eq!(
        dev_statuses,
        vec![
            "pool",
            "pending",
            "planning",
            "running",
            "failed",
            "paused",
            "code_complete",
            "code_review",
            "pr_submitted",
            "completed",
            "archived",
        ]
    );
}

#[test]
fn agent_scheduler_execution_wake_does_not_consume_merge_action() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerWakeResponse;

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-merge-dispatch-test".to_string());
    app.task_board_executor_running = true;
    app.task_board_settings.max_concurrent = 3;

    let mut task = Task::new(1);
    task.id = "T-merge-only".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.merge_source_branch = Some("vw/task/T-merge-only".to_string());
    task.merge_target_branch = Some("main".to_string());
    app.task_board_tasks = vec![task];

    let response = AgentSchedulerWakeResponse {
        dispatched: true,
        task_id: Some("T-merge-only".to_string()),
        run_id: Some("run-merge".to_string()),
        worker_id: Some("gateway-local".to_string()),
        outbox_message_id: None,
    };
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(response)),
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-only")
        .expect("merge task should remain in task board");
    assert!(!app.task_board_executor.running_tasks.contains(&"T-merge-only".to_string()));
    assert!(app.task_board_manual_merge_queue.is_empty());
    assert!(
        task.logs.iter().all(|entry| !entry.message.contains("Agent Scheduler 合并命令")),
        "Agent Scheduler wake must not enqueue local merge commands: {:?}",
        task.logs
    );
}

#[test]
fn agent_scheduler_auto_merge_wake_syncs_without_local_queue() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerWakeResponse;

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-auto-merge-dispatch-test".to_string());
    app.task_board_executor_running = true;
    app.task_board_settings.max_concurrent = 1;

    let mut pending = Task::new(1);
    pending.id = "T-pending".to_string();
    pending.status = TaskStatus::Pending;
    pending.priority = 1;

    let mut first_merge = Task::new(2);
    first_merge.id = "T-merge-first".to_string();
    first_merge.status = TaskStatus::PrSubmitted;
    first_merge.priority = 2;
    first_merge.order = 1;
    first_merge.merge_source_branch = Some("vw/task/T-merge-first".to_string());
    first_merge.merge_target_branch = Some("main".to_string());

    let mut second_merge = Task::new(3);
    second_merge.id = "T-merge-second".to_string();
    second_merge.status = TaskStatus::PrSubmitted;
    second_merge.priority = 3;
    second_merge.order = 2;
    second_merge.merge_source_branch = Some("vw/task/T-merge-second".to_string());
    second_merge.merge_target_branch = Some("main".to_string());

    app.task_board_tasks = vec![pending, first_merge, second_merge];

    let response = AgentSchedulerWakeResponse {
        dispatched: true,
        task_id: Some("T-merge-first".to_string()),
        run_id: Some("run-merge-first".to_string()),
        worker_id: Some("gateway-local".to_string()),
        outbox_message_id: None,
    };

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted(Ok(response)),
    );

    assert!(!app.task_board_executor.running_tasks.contains(&"T-merge-first".to_string()));
    assert!(app.task_board_manual_merge_queue.is_empty());
    assert_eq!(app.task_board_worktree_desired_capacity, None);
    assert!(
        !app.task_board_executor.running_tasks.contains(&"T-pending".to_string()),
        "auto merge should not dispatch pending tasks through the merge tick"
    );
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-first")
        .expect("merge task should remain in task board");
    assert!(task.logs.iter().all(|entry| !entry.message.contains("自动合并轮询已入队")));
    assert!(task.logs.iter().all(|entry| !entry.message.contains("Agent Scheduler 合并命令")));
}

#[test]
fn execution_tick_defers_pending_selection_to_gateway_schedule() {
    use crate::app::task::{Task, TaskSplitMode, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-non-git-worktree-prealloc".to_string());
    app.task_board_executor_running = true;
    app.task_board_settings.max_concurrent = 1;

    let mut task = Task::new(1);
    task.id = "T-worktree-prealloc-failure".to_string();
    task.status = TaskStatus::Pending;
    task.task_split_mode = TaskSplitMode::Disabled;
    task.merge_target_branch = Some("main".to_string());
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::ExecutionTick);

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-worktree-prealloc-failure")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.logs.iter().all(|entry| !entry.message.contains("转入代码完成")));
    assert!(
        !app.task_board_executor.running_tasks.contains(&"T-worktree-prealloc-failure".to_string())
    );
}

#[test]
fn worktree_preallocation_failure_pauses_instead_of_code_complete() {
    use crate::app::task::{Task, TaskStatus};

    let mut task = Task::new(1);
    task.id = "T-worktree-prealloc-failure".to_string();
    task.status = TaskStatus::Planning;
    task.execution_started_at_ms = Some(0);

    super::mark_task_paused_after_worktree_preallocation_failure(
        &mut task,
        "missing worktree branch".to_string(),
    );

    assert_eq!(task.status, TaskStatus::Paused);
    assert_eq!(
        task.pause_reason.as_deref(),
        Some("Git/worktree 预分配失败: missing worktree branch")
    );
    assert!(task.logs.iter().any(|entry| entry.message.contains("执行前预分配失败")));
    assert!(task.logs.iter().all(|entry| !entry.message.contains("转入代码完成")));
}

#[test]
fn auto_code_review_tick_leaves_code_complete_for_stage_capabilities() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-code-complete-advance".to_string());
    app.task_board_executor_running = true;

    let mut task = Task::new(1);
    task.id = "T-code-complete-advance".to_string();
    task.status = TaskStatus::CodeComplete;
    task.merge_source_branch = Some("vw/task/T-code-complete-advance".to_string());
    task.selected_worktree_path = Some("/tmp/vibewindow-missing-worktree".to_string());
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::AutoCodeReviewTick);

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-code-complete-advance")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert!(task.logs.iter().all(|entry| !entry.message.contains("自动审核已跳过")));
    assert!(task.logs.iter().all(|entry| !entry.message.contains("Agent Scheduler 合并命令")));
}

#[test]
fn agent_scheduler_merge_ready_persisted_sends_merge_command_log() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-merge-ready-persisted".to_string();
    app.project_path = Some(project_path.clone());

    let mut task = Task::new(1);
    task.id = "T-merge-ready-persisted".to_string();
    task.status = TaskStatus::PrSubmitted;
    app.task_board_tasks = vec![task.clone()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerMergeReadyPersisted {
            project_path,
            task,
            result: Ok(()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-ready-persisted")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::PrSubmitted);
    assert!(task.logs.iter().any(|entry| entry.message == "已发送 Agent Scheduler 合并命令"));
}

#[test]
fn agent_scheduler_command_in_flight_tick_logs_gateway_sync() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-command-in-flight".to_string();
    app.project_path = Some(project_path.clone());

    let mut task = Task::new(1);
    task.id = "T-command-in-flight".to_string();
    task.status = TaskStatus::PrSubmitted;
    app.task_board_tasks = vec![task];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick {
            project_path,
            task_id: "T-command-in-flight".to_string(),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-command-in-flight")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::PrSubmitted);
    assert!(task.logs.iter().any(|entry| {
        entry.message == "Agent Scheduler 合并命令等待返回，正在同步网关状态/日志"
    }));
}

#[test]
fn auto_code_review_tick_leaves_code_complete_without_merge_metadata() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-code-complete-missing-merge".to_string());
    app.task_board_executor_running = true;

    let mut task = Task::new(1);
    task.id = "T-code-complete-missing-merge".to_string();
    task.status = TaskStatus::CodeComplete;
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::AutoCodeReviewTick);

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-code-complete-missing-merge")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert_eq!(task.pause_reason, None);
    assert!(task.logs.iter().all(|entry| !entry.message.contains("自动审核已跳过")));
    assert!(task.logs.iter().all(|entry| !entry.message.contains("Agent Scheduler 合并命令")));
}

#[test]
fn task_updated_wakes_late_stage_pipeline() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.task_board_next_auto_review_tick_at_ms = 0;
    app.task_board_next_auto_merge_tick_at_ms = 0;

    let mut review_task = Task::new(1);
    review_task.id = "T-review-wakeup".to_string();
    review_task.status = TaskStatus::CodeReview;
    let _task = super::update(&mut app, super::TaskBoardMessage::TaskUpdated(review_task));
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);
    assert_eq!(app.task_board_next_auto_merge_tick_at_ms, 0);

    let mut merge_task = Task::new(2);
    merge_task.id = "T-merge-wakeup".to_string();
    merge_task.status = TaskStatus::PrSubmitted;
    let _task = super::update(&mut app, super::TaskBoardMessage::TaskUpdated(merge_task));
    assert!(app.task_board_next_auto_merge_tick_at_ms > 0);
}

#[test]
fn toggle_board_without_project_keeps_current_screen() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.screen = crate::app::Screen::Home;

    let _task = super::update(&mut app, super::TaskBoardMessage::ToggleBoard);

    assert_eq!(app.screen, crate::app::Screen::Home);
    assert!(!app.show_task_board);
}

#[test]
fn close_board_without_project_returns_home() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.screen = crate::app::Screen::TaskBoard;
    app.show_task_board = true;

    let _task = super::update(&mut app, super::TaskBoardMessage::CloseBoard);

    assert_eq!(app.screen, crate::app::Screen::Home);
    assert!(!app.show_task_board);
}

#[test]
fn loaded_tasks_preserve_running_subtask_state() {
    use crate::app::task::{SubTask, SubTaskStatus, Task, TaskStatus};

    let mut current = Task::new(1);
    current.id = "T-running".to_string();
    current.status = TaskStatus::Running;
    let mut current_subtask = SubTask::new("live".to_string());
    current_subtask.id = "SUB-live".to_string();
    current_subtask.start_execution();
    current.subtasks = vec![current_subtask];

    let mut loaded = current.clone();
    loaded.subtasks[0].status = SubTaskStatus::Pending;
    loaded.subtasks[0].completed = false;
    loaded.subtasks[0].execution_started_at_ms = None;

    let merged = super::merge_loaded_tasks_preserving_running_state(
        vec![loaded],
        &[current],
        &["T-running".to_string()],
    );

    assert_eq!(merged[0].subtasks[0].status, SubTaskStatus::Running);
    assert!(merged[0].subtasks[0].execution_started_at_ms.is_some());
}

#[test]
fn stale_project_tasks_loaded_does_not_replace_current_board() {
    use crate::app::task::Task;

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/current-project".to_string());
    let mut current = Task::new(1);
    current.id = "T-current".to_string();
    let mut stale = Task::new(1);
    stale.id = "T-stale".to_string();
    app.task_board_tasks = vec![current];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TasksLoaded {
            project_path: "/tmp/old-project".to_string(),
            tasks: vec![stale],
        },
    );

    assert_eq!(app.task_board_tasks.len(), 1);
    assert_eq!(app.task_board_tasks[0].id, "T-current");
}

#[test]
fn stale_project_execution_result_does_not_mutate_same_id_current_task() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/current-project".to_string());
    let mut task = Task::new(1);
    task.id = "T-colliding".to_string();
    task.status = TaskStatus::Running;
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-colliding");

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path: "/tmp/old-project".to_string(),
            task_id: "T-colliding".to_string(),
            result: Ok("done".to_string()),
        },
    );

    assert_eq!(app.task_board_tasks[0].status, TaskStatus::Running);
    assert!(app.task_board_executor.running_tasks.contains(&"T-colliding".to_string()));
}

#[test]
fn loaded_tasks_does_not_rebootstrap_running_executor() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = true;
    app.task_board_next_auto_promote_tick_at_ms = 12_345;

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TasksLoaded { project_path, tasks: Vec::new() },
    );

    assert!(app.task_board_executor_running);
    assert_eq!(app.task_board_next_auto_promote_tick_at_ms, 12_345);
}

#[test]
fn stop_execution_stops_executor_without_legacy_switches() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_executor_running = true;

    let _task = super::update(&mut app, super::TaskBoardMessage::StopExecution);

    assert!(!app.task_board_executor_running);
}

#[test]
fn bulk_task_split_updates_board_detail_and_editing_draft_immediately() {
    use crate::app::task::{Task, TaskSplitMode, TaskStatus};

    let (mut app, _task) = crate::app::App::new();

    let mut first = Task::new(1);
    first.id = "T-split-1".to_string();
    first.status = TaskStatus::Pending;
    first.task_split_mode = TaskSplitMode::Disabled;

    let mut second = Task::new(2);
    second.id = "T-split-2".to_string();
    second.status = TaskStatus::Pending;
    second.task_split_mode = TaskSplitMode::Disabled;

    app.task_board_tasks = vec![first.clone(), second];
    app.task_board_selected_tasks.insert("T-split-1".to_string());
    app.task_board_editing_task_id = Some("T-split-1".to_string());
    app.task_board_viewing_logs = Some(first);
    app.task_board_draft.task_split_mode = TaskSplitMode::Disabled;

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::BulkSetTaskSplitModeInStatus {
            status: TaskStatus::Pending,
            task_split_mode: TaskSplitMode::Enabled,
        },
    );

    let updated = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-split-1")
        .expect("selected task should remain");
    assert_eq!(updated.task_split_mode, TaskSplitMode::Enabled);
    assert_eq!(
        app.task_board_viewing_logs.as_ref().map(|task| task.task_split_mode),
        Some(TaskSplitMode::Enabled)
    );
    assert_eq!(app.task_board_draft.task_split_mode, TaskSplitMode::Enabled);
    assert!(app.task_board_selected_tasks.is_empty());
}

#[test]
fn task_execution_late_success_after_timeout_is_applied() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();

    let mut task = Task::new(1);
    task.id = "T-late-success".to_string();
    task.status = TaskStatus::Failed;
    task.last_error = Some("Running超过 1 分钟仍未完成，已自动标记失败".to_string());
    app.task_board_tasks = vec![task];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-late-success".to_string(),
            result: Ok("done".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-late-success")
        .expect("task should remain in task board");
    assert_ne!(task.status, TaskStatus::Failed);
    assert!(task.logs.iter().any(|entry| entry.message.contains("执行结果晚于超时返回")));
    assert!(task.logs.iter().any(|entry| entry.message.contains("执行完成")));
    assert!(
        !task.logs.iter().any(|entry| entry.message.contains("忽略本次执行结果")),
        "late success should not be ignored"
    );
}

#[test]
fn task_execution_success_waits_in_code_complete_for_tick() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();

    let mut task = Task::new(1);
    task.id = "T-success-code-complete".to_string();
    task.status = TaskStatus::Running;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-success-code-complete".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-success-code-complete".to_string(),
            result: Ok("done".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-success-code-complete")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert!(task.logs.iter().any(|entry| entry.message.contains("等待代码完成调度")));
    assert!(app.task_board_executor.running_tasks.is_empty());
}

#[test]
fn task_execution_failure_preserves_worktree_context() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-exec-failure-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-exec-failure".to_string();
    task.status = TaskStatus::Running;
    task.selected_worktree_path = Some("/tmp/vibewindow-task-worktree".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-exec-failure".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-exec-failure".to_string(),
            result: Err("network timeout".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-exec-failure")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.selected_worktree_path.as_deref(), Some("/tmp/vibewindow-task-worktree"));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("任务执行失败")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}

#[test]
fn task_execution_model_response_timeout_fails_without_pending_retry() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = Task::new(1);
    task.id = "T-model-timeout".to_string();
    task.status = TaskStatus::Running;
    task.execution_started_at_ms = Some(0);
    task.selected_worktree_path = Some("/tmp/vibewindow-timeout-worktree".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-model-timeout".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-model-timeout".to_string(),
            result: Err("模型响应超时".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-model-timeout")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("模型响应超时"));
    assert!(task.execution_started_at_ms.is_none());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert_eq!(task.selected_worktree_path.as_deref(), Some("/tmp/vibewindow-timeout-worktree"));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("模型响应超时")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
    assert!(task.logs.iter().any(|entry| entry.message.contains("状态变更: 执行中 → 执行失败")));
}

#[test]
fn task_execution_gateway_idle_timeout_fails_without_pending_retry() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = Task::new(1);
    task.id = "T-gateway-idle-timeout".to_string();
    task.status = TaskStatus::Running;
    task.execution_started_at_ms = Some(0);
    task.selected_worktree_path = Some("/tmp/vibewindow-gateway-timeout-worktree".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-gateway-idle-timeout".to_string()];

    let error = "gateway 调度失败: gateway stream idle timeout after 60s without response";
    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-gateway-idle-timeout".to_string(),
            result: Err(error.to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-gateway-idle-timeout")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some(error));
    assert!(task.execution_started_at_ms.is_none());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert_eq!(
        task.selected_worktree_path.as_deref(),
        Some("/tmp/vibewindow-gateway-timeout-worktree")
    );
    assert!(!task.logs.iter().any(|entry| entry.message.contains("自动重试")));
    assert!(task.logs.iter().any(|entry| entry.message.contains("状态变更: 执行中 → 执行失败")));
}

#[test]
fn task_planning_model_response_timeout_fails_without_pending_retry() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-planning-timeout-test".to_string());
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = Task::new(1);
    task.id = "T-planning-timeout".to_string();
    task.status = TaskStatus::Planning;
    task.execution_started_at_ms = Some(0);
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-planning-timeout".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskPlanningCompleted {
            project_path,
            task_id: "T-planning-timeout".to_string(),
            result: Err("模型响应超时".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-planning-timeout")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.last_error.as_deref(), Some("任务拆分模型响应超时: 模型响应超时"));
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert!(task.logs.iter().any(|entry| entry.message.contains("任务拆分模型响应超时")));
    assert!(!task.logs.iter().any(|entry| entry.message.contains("自动重试")));
    assert!(task.logs.iter().any(|entry| entry.message.contains("状态变更: 任务拆分 → 执行失败")));
}

#[test]
fn planning_completed_logs_create_task_plans_prompt_batches_and_starts_execution() {
    use crate::app::task::{Task, TaskPlanGenerationOutcome, TaskPlanSubTask, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-planning-success-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-planning-success".to_string();
    task.status = TaskStatus::Planning;
    task.execution_started_at_ms = Some(0);
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-planning-success".to_string()];

    let plan_dir = "/tmp/vibewindow-planning-success-test/.plans/T-planning-success".to_string();
    let prompt_md_path = format!("{plan_dir}/prompt.md");
    let outcome = TaskPlanGenerationOutcome {
        plan_dir: plan_dir.clone(),
        prompt_md_path: prompt_md_path.clone(),
        raw_output: "create-task-plans 原始输出".to_string(),
        subtasks: vec![
            TaskPlanSubTask {
                title: "任务一".to_string(),
                boundary: "边界一".to_string(),
                acceptance_criteria: vec!["验收一".to_string()],
                target_files: vec!["src/a.rs".to_string()],
                batch: 1,
                depends_on_batches: vec![],
                execution_prompt: "执行提示一".to_string(),
            },
            TaskPlanSubTask {
                title: "任务二".to_string(),
                boundary: "边界二".to_string(),
                acceptance_criteria: vec!["验收二".to_string()],
                target_files: vec!["src/b.rs".to_string()],
                batch: 2,
                depends_on_batches: vec![1],
                execution_prompt: "执行提示二".to_string(),
            },
        ],
    };

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskPlanningCompleted {
            project_path,
            task_id: "T-planning-success".to_string(),
            result: Ok(outcome),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-planning-success")
        .expect("task should remain in task board");

    assert_eq!(task.subtasks.len(), 2);
    assert_eq!(task.subtasks[0].batch, 1);
    assert_eq!(task.subtasks[1].depends_on_batches, vec![1]);
    assert_eq!(task.status, TaskStatus::Running);
    assert!(app.task_board_executor.running_tasks.contains(&"T-planning-success".to_string()));

    assert!(
        task.logs.iter().any(|entry| entry.message.contains(&plan_dir)),
        "planning success log should include the create-task-plans plan directory"
    );
    assert!(
        task.logs.iter().any(|entry| entry.message.contains("prompt.md")),
        "planning success log should reference prompt.md"
    );
    assert!(
        task.logs.iter().any(|entry| entry.message.contains("批次")),
        "planning success log should include a batch summary"
    );
    assert!(task.logs.iter().any(|entry| entry.message.contains("2 个子任务")));
}

#[test]
fn planning_completed_missing_prompt_md_error_does_not_start_execution() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-planning-missing-prompt-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-planning-missing-prompt".to_string();
    task.status = TaskStatus::Planning;
    task.execution_started_at_ms = Some(0);
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-planning-missing-prompt".to_string()];

    let error = "无法读取任务拆分结果 /tmp/.plans/T/prompt.md: No such file or directory";
    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskPlanningCompleted {
            project_path,
            task_id: "T-planning-missing-prompt".to_string(),
            result: Err(error.to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-planning-missing-prompt")
        .expect("task should remain in task board");

    assert_eq!(task.status, TaskStatus::Failed);
    assert!(task.subtasks.is_empty());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert!(
        task.logs.iter().any(|entry| entry.message.contains("任务拆分")),
        "planning failure log should attribute the failure to the task split artifact"
    );
}

#[test]
fn task_code_review_model_response_timeout_pauses_without_waiting_retry() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.failed_retry_minutes = 1;

    let mut task = Task::new(1);
    task.id = "T-review-timeout".to_string();
    task.status = TaskStatus::CodeReview;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-review-timeout".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskCodeReviewCompleted {
            project_path,
            task_id: "T-review-timeout".to_string(),
            result: Err("模型响应超时".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-review-timeout")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Paused);
    assert!(task.last_error.is_none());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert!(
        task.pause_reason.as_deref().is_some_and(|reason| reason.contains("代码审核模型响应超时"))
    );
    assert!(!task.logs.iter().any(|entry| entry.message.contains("自动重试")));
}

#[test]
fn task_merge_failure_collects_reason_and_preserves_worktree_context() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-merge-failure-test".to_string());
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = Task::new(1);
    task.id = "T-merge-failure".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.merge_source_branch = Some("vw/task/T-merge-failure".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path = Some("/tmp/vibewindow-merge-worktree".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-merge-failure".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path,
            task_id: "T-merge-failure".to_string(),
            result: Err("conflict on src/lib.rs".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-failure")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Paused);
    assert_eq!(task.selected_worktree_path.as_deref(), Some("/tmp/vibewindow-merge-worktree"));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("合并失败详情")
            && entry.message.contains("source=vw/task/T-merge-failure")
            && entry.message.contains("target=main")
            && entry.message.contains("worktree=/tmp/vibewindow-merge-worktree")
            && entry.message.contains("conflict on src/lib.rs")
    }));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("合并失败")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}

#[test]
fn task_board_matrix_publish_status_change_uses_safe_payload_and_failure_is_nonfatal() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::task::{TaskPoolMatrixEventKindDto, TaskPoolStatus};

    let (mut app, _task) = crate::app::App::new();
    app.channels_settings.matrix = Some(vw_config_types::channels::MatrixConfig {
        homeserver: "https://matrix.test".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@bot:matrix.test".to_string()),
        device_id: Some("DEVICE".to_string()),
        room_id: "!room:matrix.test".to_string(),
        allowed_users: vec!["@alice:matrix.test".to_string()],
        mention_only: false,
    });

    let mut task = Task::new(1);
    task.id = "T-matrix-safe".to_string();
    task.status = TaskStatus::Pending;
    task.description = "sensitive description".to_string();
    task.prompt = "sensitive prompt".to_string();
    let from_status = task.status;
    task.set_status(TaskStatus::Running);

    let request = super::build_task_pool_matrix_publish_request(&app, &task, Some(from_status))
        .expect("matrix config should produce a publish request");
    assert_eq!(request.event.version, 1);
    assert_eq!(request.event.task_id, "T-matrix-safe");
    assert_eq!(request.event.actor.matrix_user_id, "@bot:matrix.test");
    assert_eq!(
        request.event.event_id,
        format!("desktop:T-matrix-safe:running:{}", task.updated_at_ms)
    );
    assert_eq!(request.event.occurred_at_ms, task.updated_at_ms);
    match &request.event.kind {
        TaskPoolMatrixEventKindDto::TaskStatusChanged { from_status, to_status } => {
            assert_eq!(from_status.as_ref(), Some(&TaskPoolStatus::Pending));
            assert_eq!(to_status, &TaskPoolStatus::Running);
        }
        _ => panic!("expected task_status_changed event"),
    }

    let serialized = serde_json::to_string(&request).expect("request should serialize");
    assert!(!serialized.contains("sensitive prompt"));
    assert!(!serialized.contains("sensitive description"));
    assert!(!serialized.contains("matrix-token"));

    app.task_board_tasks = vec![task];
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskPoolMatrixEventPublished {
            task_id: "T-matrix-safe".to_string(),
            result: Err("gateway down".to_string()),
        },
    );

    assert_eq!(app.task_board_tasks[0].status, TaskStatus::Running);
    let toast = app.active_toast.as_ref().expect("failure should show a toast");
    assert!(toast.message.contains("Matrix"));
    assert!(toast.message.contains("gateway down"));
}

#[test]
fn task_board_matrix_publish_skips_placeholder_room_id() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.channels_settings.matrix = Some(vw_config_types::channels::MatrixConfig {
        homeserver: "https://matrix.test".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@bot:matrix.test".to_string()),
        device_id: Some("DEVICE".to_string()),
        room_id: "!room-id:localhost".to_string(),
        allowed_users: vec!["@alice:matrix.test".to_string()],
        mention_only: false,
    });

    let mut task = Task::new(1);
    task.id = "T-matrix-placeholder".to_string();
    task.status = TaskStatus::Pending;
    let from_status = task.status;
    task.set_status(TaskStatus::Running);

    assert!(
        super::build_task_pool_matrix_publish_request(&app, &task, Some(from_status)).is_none()
    );
}

#[test]
fn task_board_matrix_instance_publish_uses_staff_matrix_binding() {
    use crate::app::task::{Task, TaskStatus};
    use vw_config_types::channels::{MatrixConfig, MatrixInstanceConfig};
    use vw_config_types::staff::StaffChannelBindingConfig;

    let (mut app, _task) = crate::app::App::new();
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://legacy.matrix.test".to_string(),
        access_token: "legacy-token".to_string(),
        user_id: Some("@legacy:matrix.test".to_string()),
        device_id: Some("LEGACY".to_string()),
        room_id: "!legacy:matrix.test".to_string(),
        allowed_users: vec!["@alice:matrix.test".to_string()],
        mention_only: false,
    });
    app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
        id: "planner".to_string(),
        config: MatrixConfig {
            homeserver: "https://planner.matrix.test".to_string(),
            access_token: "planner-token".to_string(),
            user_id: Some("@planner-bot:matrix.test".to_string()),
            device_id: Some("PLANNER".to_string()),
            room_id: "!planner:matrix.test".to_string(),
            allowed_users: vec!["@alice:matrix.test".to_string()],
            mention_only: false,
        },
    });

    let planner = app
        .task_board_staff_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner staff member");
    planner.current_task_ids = vec!["T-matrix-staff".to_string()];
    planner.channel_bindings = vec![StaffChannelBindingConfig {
        channel_type: "matrix".to_string(),
        instance_id: "planner".to_string(),
    }];

    let mut task = Task::new(1);
    task.id = "T-matrix-staff".to_string();
    task.status = TaskStatus::Planning;
    let from_status = task.status;
    task.set_status(TaskStatus::Running);

    let request = super::build_task_pool_matrix_publish_request(&app, &task, Some(from_status))
        .expect("staff matrix binding should produce a publish request");

    assert_eq!(request.matrix_instance_id.as_deref(), Some("planner"));
    assert_eq!(request.event.actor.matrix_user_id, "@planner-bot:matrix.test");
}

#[test]
fn task_board_matrix_inbound_events_refresh_gateway_snapshot_without_local_writes() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::task::{
        TaskPoolMatrixActorDto, TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto,
        TaskPoolMatrixEventsResponse, TaskPoolStatus,
    };

    fn actor() -> TaskPoolMatrixActorDto {
        TaskPoolMatrixActorDto {
            matrix_user_id: "@peer:matrix.test".to_string(),
            display_name: Some("Peer".to_string()),
        }
    }

    fn event(
        event_id: &str,
        task_id: &str,
        kind: TaskPoolMatrixEventKindDto,
    ) -> TaskPoolMatrixEventDto {
        TaskPoolMatrixEventDto {
            version: 1,
            event_id: event_id.to_string(),
            task_id: task_id.to_string(),
            actor: actor(),
            occurred_at_ms: 42,
            kind,
        }
    }

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/matrix-refresh-project".to_string());

    let mut claimed = Task::new(1);
    claimed.id = "T-matrix-claimed".to_string();
    claimed.status = TaskStatus::Pending;
    let claimed_id = claimed.id.clone();

    let mut noted = Task::new(2);
    noted.id = "T-matrix-noted".to_string();
    noted.status = TaskStatus::Running;
    let noted_id = noted.id.clone();

    let mut blocked = Task::new(3);
    blocked.id = "T-matrix-blocked".to_string();
    blocked.status = TaskStatus::Running;
    let blocked_id = blocked.id.clone();

    let mut advanced = Task::new(4);
    advanced.id = "T-matrix-advanced".to_string();
    advanced.status = TaskStatus::Pending;
    let advanced_id = advanced.id.clone();

    let mut illegal = Task::new(5);
    illegal.id = "T-matrix-illegal".to_string();
    illegal.status = TaskStatus::Running;
    let illegal_id = illegal.id.clone();

    let mut completed = Task::new(6);
    completed.id = "T-matrix-completed".to_string();
    completed.status = TaskStatus::Completed;
    let completed_id = completed.id.clone();

    app.task_board_tasks = vec![claimed, noted, blocked, advanced, illegal, completed];

    let response = TaskPoolMatrixEventsResponse {
        cursor: 9,
        events: vec![
            event(
                "$claim",
                &claimed_id,
                TaskPoolMatrixEventKindDto::TaskClaimed { assignee: "alice".to_string() },
            ),
            event(
                "$note",
                &noted_id,
                TaskPoolMatrixEventKindDto::TaskNote { message: "ready for review".to_string() },
            ),
            event(
                "$block",
                &blocked_id,
                TaskPoolMatrixEventKindDto::TaskBlocked { reason: "waiting for API".to_string() },
            ),
            event(
                "$advance",
                &advanced_id,
                TaskPoolMatrixEventKindDto::TaskStatusChanged {
                    from_status: Some(TaskPoolStatus::Pending),
                    to_status: TaskPoolStatus::Planning,
                },
            ),
            event(
                "$claim",
                &claimed_id,
                TaskPoolMatrixEventKindDto::TaskClaimed { assignee: "bob".to_string() },
            ),
            event(
                "$illegal",
                &illegal_id,
                TaskPoolMatrixEventKindDto::TaskStatusChanged {
                    from_status: Some(TaskPoolStatus::Running),
                    to_status: TaskPoolStatus::Pending,
                },
            ),
            event(
                "$completed",
                &completed_id,
                TaskPoolMatrixEventKindDto::TaskStatusChanged {
                    from_status: Some(TaskPoolStatus::Completed),
                    to_status: TaskPoolStatus::Running,
                },
            ),
            event(
                "$missing",
                "T-missing",
                TaskPoolMatrixEventKindDto::TaskNote {
                    message: "missing task should not create anything".to_string(),
                },
            ),
            event(
                "$too-long",
                &noted_id,
                TaskPoolMatrixEventKindDto::TaskNote { message: "x".repeat(1025) },
            ),
        ],
    };

    let _task =
        super::update(&mut app, super::TaskBoardMessage::MatrixTaskPoolEventsLoaded(Ok(response)));

    assert_eq!(app.task_board_matrix_event_cursor, 9);
    assert_eq!(app.task_board_tasks.len(), 6);

    let claimed =
        app.task_board_tasks.iter().find(|task| task.id == claimed_id).expect("claimed task");
    assert_eq!(claimed.assignee, "VibeWindow");
    assert_eq!(claimed.logs.len(), 1);

    let noted = app.task_board_tasks.iter().find(|task| task.id == noted_id).expect("noted task");
    assert_eq!(noted.status, TaskStatus::Running);
    assert_eq!(noted.logs.len(), 1);

    let blocked =
        app.task_board_tasks.iter().find(|task| task.id == blocked_id).expect("blocked task");
    assert_eq!(blocked.status, TaskStatus::Running);
    assert!(blocked.pause_reason.is_none());

    let advanced =
        app.task_board_tasks.iter().find(|task| task.id == advanced_id).expect("advanced task");
    assert_eq!(advanced.status, TaskStatus::Pending);

    let illegal =
        app.task_board_tasks.iter().find(|task| task.id == illegal_id).expect("illegal task");
    assert_eq!(illegal.status, TaskStatus::Running);

    let completed =
        app.task_board_tasks.iter().find(|task| task.id == completed_id).expect("completed task");
    assert_eq!(completed.status, TaskStatus::Completed);

    assert!(app.task_board_processed_matrix_event_ids.contains(&"$claim".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$note".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$block".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$advance".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$illegal".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$completed".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$missing".to_string()));
    assert!(app.task_board_processed_matrix_event_ids.contains(&"$too-long".to_string()));
}

#[test]
fn task_board_applies_agent_scheduler_custom_status_key() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.workflow_id = "content_publishing".to_string();
    let mut task = Task::new(1);
    task.id = "T-content".to_string();
    task.status = TaskStatus::parse_key("content_queued").expect("content queued status");
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-content");

    let response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/content".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/content".into(),
            task_id: "T-content".into(),
            status_key: "content_writing".to_string(),
            status: Some(AgentTaskStatusDto::Queued),
            priority: 5,
            order_no: 2,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "write the launch article".to_string(),
            retry_count: 0,
            run_id: Some("run-content".to_string()),
            run_stage: Some("running".to_string()),
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_716_000_000_000),
        }],
    };

    let updated = super::apply_agent_scheduler_status(&mut app, response);

    assert_eq!(updated.len(), 1);
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-content")
        .expect("runtime task should remain on board");
    assert_eq!(task.status, TaskStatus::Custom("content_writing"));
    assert_eq!(task.status.label_for_workflow_id("content_publishing"), "写作中");
    assert!(!app.task_board_executor.running_tasks.contains(&"T-content".to_string()));
}

#[test]
fn agent_scheduler_registers_active_workflow_pool_and_queued_statuses() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{AgentTaskKindDto, AgentTaskStatusDto};

    let mut queued = Task::new(7);
    queued.id = "T-campaign-queued".to_string();
    queued.status = TaskStatus::parse_key("campaign_queued").expect("campaign queued status");
    queued.prompt = "Prepare the campaign launch".to_string();
    queued.order = 3;
    queued.model = "gpt-campaign".to_string();

    let mut backlog = Task::new(1);
    backlog.id = "T-campaign-backlog".to_string();
    backlog.status = TaskStatus::parse_key("campaign_ideas").expect("campaign backlog status");

    let requests = super::super::helpers::agent_scheduler_wake_create_task_requests(
        &[backlog, queued],
        "marketing_campaign",
    );

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].task_id, "T-campaign-backlog");
    assert_eq!(requests[0].status, Some(AgentTaskStatusDto::Draft));
    let request = &requests[1];
    assert_eq!(request.task_id, "T-campaign-queued");
    assert_eq!(request.kind, AgentTaskKindDto::Finite);
    assert_eq!(request.status, Some(AgentTaskStatusDto::Queued));
    assert_eq!(request.metadata["task_board.workflow_id"], "marketing_campaign");
    assert_eq!(request.metadata["task_board.status_key"], "campaign_queued");
    assert_eq!(request.metadata["task_board.status_role"], "queued");
    assert_eq!(request.metadata["task_board.status_key_map"]["queued"], "campaign_queued");
    assert_eq!(request.metadata["task_board.status_key_map"]["running"], "campaign_running");
    assert_eq!(request.metadata["task_board.status_key_map"]["submitted"], "campaign_review_ready");
    assert_eq!(request.metadata["prompt"], "Prepare the campaign launch");
}

#[test]
fn agent_scheduler_status_sync_moves_pending_task_out_of_pending() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto,
        AgentSchedulerTaskLogDto, AgentSchedulerTaskLogsResponse, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-wake-closed-loop".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-wake-closed-loop");

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/wake".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/wake".into(),
            task_id: "T-wake-closed-loop".into(),
            status_key: "code_complete".to_string(),
            status: Some(AgentTaskStatusDto::Submitted),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "run queued task through wake".to_string(),
            retry_count: 0,
            run_id: Some("run-wake-closed-loop".to_string()),
            run_stage: None,
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_902_000),
        }],
    };
    let logs_response = AgentSchedulerTaskLogsResponse {
        task_id: "T-wake-closed-loop".into(),
        logs: vec![AgentSchedulerTaskLogDto {
            task_id: "T-wake-closed-loop".into(),
            seq_no: 1,
            timestamp_ms: TimestampMs(1_783_000_902_100),
            from_status_key: Some("running".to_string()),
            status_from: Some(AgentTaskStatusDto::Running),
            to_status_key: Some("submitted".to_string()),
            status_to: Some(AgentTaskStatusDto::Submitted),
            message: "local execution submitted result".to_string(),
        }],
    };

    let status_updates = super::apply_agent_scheduler_status(&mut app, status_response);
    let log_updates = super::apply_agent_scheduler_logs(&mut app, vec![logs_response]);

    assert_eq!(status_updates.len(), 1);
    assert_eq!(log_updates.len(), 1);
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-wake-closed-loop")
        .expect("wake task should remain on board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert_ne!(task.status, TaskStatus::Pending);
    assert!(!app.task_board_executor.running_tasks.contains(&"T-wake-closed-loop".to_string()));
    assert!(task.logs.iter().any(|entry| {
        entry.message == "local execution submitted result"
            && entry.status_from == Some(TaskStatus::Running)
            && entry.status_to == Some(TaskStatus::CodeReview)
    }));
}

#[test]
fn agent_scheduler_status_sync_uses_agent_status_when_status_key_is_stale_pending() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-stale-pending-sync".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-stale-pending-sync");

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/stale-pending".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/stale-pending".into(),
            task_id: "T-stale-pending-sync".into(),
            status_key: "pending".to_string(),
            status: Some(AgentTaskStatusDto::Submitted),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "sync submitted task with stale pending key".to_string(),
            retry_count: 0,
            run_id: Some("run-stale-pending-sync".to_string()),
            run_stage: Some("succeeded".to_string()),
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_902_000),
        }],
    };

    let status_updates = super::apply_agent_scheduler_status(&mut app, status_response);

    assert_eq!(status_updates.len(), 1);
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-stale-pending-sync")
        .expect("stale pending task should remain on board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert_ne!(task.status, TaskStatus::Pending);
    assert!(task.logs.iter().any(|entry| entry.message == "Agent Scheduler 阶段: succeeded"));
    assert!(!app.task_board_executor.running_tasks.contains(&"T-stale-pending-sync".to_string()));
}

#[test]
fn agent_scheduler_status_sync_projects_dispatched_task_to_pending() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-matrix-dispatched-sync".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/matrix-dispatched".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/matrix-dispatched".into(),
            task_id: "T-matrix-dispatched-sync".into(),
            status_key: "dispatched".to_string(),
            status: Some(AgentTaskStatusDto::Dispatched),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "dispatch through Matrix".to_string(),
            retry_count: 0,
            run_id: Some("run-matrix-dispatched-sync".to_string()),
            run_stage: None,
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_903_000),
        }],
    };

    let updated = super::apply_agent_scheduler_status(&mut app, status_response);

    assert_eq!(updated.len(), 1);
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-matrix-dispatched-sync")
        .expect("dispatched task should remain on board");
    assert_eq!(task.status, TaskStatus::Pending);
}

#[test]
fn agent_scheduler_status_sync_projects_running_run_stage_to_running() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-matrix-running-stage-sync".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/matrix-running-stage".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/matrix-running-stage".into(),
            task_id: "T-matrix-running-stage-sync".into(),
            status_key: "pending".to_string(),
            status: Some(AgentTaskStatusDto::Dispatched),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "dispatch through Matrix".to_string(),
            retry_count: 0,
            run_id: Some("run-matrix-running-stage-sync".to_string()),
            run_stage: Some("running".to_string()),
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_903_000),
        }],
    };

    let updated = super::apply_agent_scheduler_status(&mut app, status_response);

    assert_eq!(updated.len(), 1);
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-matrix-running-stage-sync")
        .expect("running stage task should remain on board");
    assert_eq!(task.status, TaskStatus::Running);
    assert!(task.logs.iter().any(|entry| entry.message == "Agent Scheduler 阶段: running"));
}

#[test]
fn agent_scheduler_status_sync_gateway_unavailable_does_not_show_warning_toast() {
    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-scheduler-unavailable".to_string();
    app.project_path = Some(project_path.clone());

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerStatusLoaded {
            project_path,
            result: Err(
                "error sending request for url (http://127.0.0.1:42617/v1/projects/resolve)"
                    .to_string(),
            ),
        },
    );

    assert!(app.active_toast.is_none());
}

#[test]
fn worktree_snapshot_failure_records_attempt_time_for_retry_throttling() {
    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-worktree-snapshot-unavailable".to_string();
    app.project_path = Some(project_path.clone());
    app.task_board_worktree_snapshot_loading = true;
    app.task_board_last_worktree_snapshot_at_ms = 0;

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::WorktreeSnapshotLoaded {
            project_path,
            result: Err(
                "error sending request for url (http://127.0.0.1:42617/v1/projects/resolve)"
                    .to_string(),
            ),
        },
    );

    assert!(!app.task_board_worktree_snapshot_loading);
    assert!(app.task_board_last_worktree_snapshot_at_ms > 0);
}

#[test]
fn agent_scheduler_status_sync_moves_submitted_task_to_code_complete() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-code-complete-sync".to_string();
    app.project_path = Some(project_path.clone());
    app.task_board_executor_running = true;
    app.task_board_next_auto_review_tick_at_ms = 0;
    app.task_board_next_auto_merge_tick_at_ms = 0;

    let mut task = Task::new(1);
    task.id = "T-code-complete-sync".to_string();
    task.status = TaskStatus::Running;
    app.task_board_tasks = vec![task];
    app.task_board_executor.start_task("T-code-complete-sync");

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/code-complete".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/code-complete".into(),
            task_id: "T-code-complete-sync".into(),
            status_key: "code_complete".to_string(),
            status: Some(AgentTaskStatusDto::Submitted),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "sync submitted task".to_string(),
            retry_count: 0,
            run_id: Some("run-code-complete-sync".to_string()),
            run_stage: None,
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
            selected_worktree_slot_id: None,
            selected_worktree_branch: None,
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_902_000),
        }],
    };

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerStatusLoaded {
            project_path,
            result: Ok(status_response),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-code-complete-sync")
        .expect("submitted task should remain on board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert!(!app.task_board_executor.running_tasks.contains(&"T-code-complete-sync".to_string()));
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);
    assert_eq!(app.task_board_next_auto_merge_tick_at_ms, 0);
}

#[test]
fn agent_scheduler_status_sync_uses_gateway_stage_for_submitted_snapshot() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-merge-stage-sync".to_string();
    app.project_path = Some(project_path.clone());
    app.task_board_executor_running = true;
    app.task_board_next_auto_review_tick_at_ms = 0;
    app.task_board_next_auto_merge_tick_at_ms = 0;

    let mut task = Task::new(1);
    task.id = "T-merge-stage-sync".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.merge_source_branch = Some("vw/task/T-merge-stage-sync".to_string());
    task.selected_worktree_path = Some("/tmp/vibewindow-merge-stage-sync/slot".to_string());
    app.task_board_tasks = vec![task];

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/merge-stage".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/merge-stage".into(),
            task_id: "T-merge-stage-sync".into(),
            status_key: "code_complete".to_string(),
            status: Some(AgentTaskStatusDto::Submitted),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "sync submitted task while merge command is in flight".to_string(),
            retry_count: 0,
            run_id: Some("run-merge-stage-sync".to_string()),
            run_stage: Some("succeeded".to_string()),
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: Some("/tmp/vibewindow-merge-stage-sync/slot".to_string()),
            selected_worktree_slot_id: Some("slot".to_string()),
            selected_worktree_branch: Some("vw/task/T-merge-stage-sync".to_string()),
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_902_000),
        }],
    };

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerStatusLoaded {
            project_path,
            result: Ok(status_response),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-stage-sync")
        .expect("merge task should remain on board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert_eq!(task.merge_source_branch, None);
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);
    assert_eq!(app.task_board_next_auto_merge_tick_at_ms, 0);
}

#[test]
fn agent_scheduler_status_sync_recovers_local_target_branch_pause() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let project_path = "/tmp/vibewindow-code-complete-paused-sync".to_string();
    app.project_path = Some(project_path.clone());
    app.task_board_executor_running = true;
    app.task_board_next_auto_review_tick_at_ms = 0;
    app.task_board_next_auto_merge_tick_at_ms = 0;

    let mut task = Task::new(1);
    task.id = "T-code-complete-target-sync".to_string();
    task.status = TaskStatus::CodeComplete;
    task.merge_source_branch = Some("vw/task/T-code-complete-target-sync".to_string());
    task.selected_worktree_path =
        Some("/tmp/vibewindow-code-complete-paused-sync/slot".to_string());
    task.mark_paused("缺少有效 target branch，无法自动合并".to_string());
    app.task_board_tasks = vec![task];

    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/code-complete".into(),
        read_model: Default::default(),
        tasks: vec![AgentSchedulerTaskBoardTaskDto {
            project_id: "project/code-complete".into(),
            task_id: "T-code-complete-target-sync".into(),
            status_key: "code_complete".to_string(),
            status: Some(AgentTaskStatusDto::Submitted),
            priority: 4,
            order_no: 9,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "sync locally target-paused submitted task".to_string(),
            retry_count: 0,
            run_id: Some("run-code-complete-paused-sync".to_string()),
            run_stage: None,
            workspace: None,
            last_error: None,
            pause_reason: None,
            merge_source_branch: Some("vw/task/T-code-complete-target-sync".to_string()),
            merge_target_branch: None,
            selected_worktree_path: Some(
                "/tmp/vibewindow-code-complete-paused-sync/slot".to_string(),
            ),
            selected_worktree_slot_id: Some("slot".to_string()),
            selected_worktree_branch: Some("vw/task/T-code-complete-target-sync".to_string()),
            read_model: Default::default(),
            updated_at_ms: TimestampMs(1_783_000_902_000),
        }],
    };

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::AgentSchedulerStatusLoaded {
            project_path,
            result: Ok(status_response),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-code-complete-target-sync")
        .expect("target-paused task should remain on board");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert_eq!(task.pause_reason, None);
    assert!(app.task_board_next_auto_review_tick_at_ms > 0);
    assert_eq!(app.task_board_next_auto_merge_tick_at_ms, 0);
}

#[test]
fn task_board_applies_agent_scheduler_logs_without_duplicates() {
    use crate::app::task::{Task, TaskStatus};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskLogDto, AgentSchedulerTaskLogsResponse, AgentTaskStatusDto,
    };
    use vw_gateway_client::vw_api_types::common::TimestampMs;

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-runtime-logs".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task.clone()];
    app.task_board_detail_tab = super::TaskBoardDetailTab::Logs;
    super::set_viewing_logs(&mut app, Some(task));

    let response = AgentSchedulerTaskLogsResponse {
        task_id: "T-runtime-logs".into(),
        logs: vec![
            AgentSchedulerTaskLogDto {
                task_id: "T-runtime-logs".into(),
                seq_no: 1,
                timestamp_ms: TimestampMs(10),
                from_status_key: Some("pending".to_string()),
                status_from: Some(AgentTaskStatusDto::Queued),
                to_status_key: Some("running".to_string()),
                status_to: Some(AgentTaskStatusDto::Running),
                message: "Agent Scheduler 领取任务".to_string(),
            },
            AgentSchedulerTaskLogDto {
                task_id: "T-runtime-logs".into(),
                seq_no: 2,
                timestamp_ms: TimestampMs(11),
                from_status_key: None,
                status_from: None,
                to_status_key: None,
                status_to: None,
                message: "执行输出 hello".to_string(),
            },
        ],
    };

    let updated = super::apply_agent_scheduler_logs(&mut app, vec![response.clone()]);
    let duplicate_update = super::apply_agent_scheduler_logs(&mut app, vec![response]);

    assert_eq!(updated.len(), 1);
    assert!(duplicate_update.is_empty());
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-runtime-logs")
        .expect("runtime logs task");
    assert!(task.logs.iter().any(|entry| {
        entry.message == "Agent Scheduler 领取任务"
            && entry.status_from == Some(TaskStatus::Pending)
            && entry.status_to == Some(TaskStatus::Running)
    }));
    assert_eq!(task.logs.iter().filter(|entry| entry.message == "执行输出 hello").count(), 1);
    let editor_text = app.task_board_logs_editor.text();
    assert!(
        editor_text.contains("Agent Scheduler 领取任务") && editor_text.contains("执行输出 hello"),
        "viewing task logs editor should include gateway logs: {editor_text}"
    );
}

#[test]
fn matrix_publish_request_rejects_same_status_and_incomplete_targets() {
    use crate::app::task::{Task, TaskStatus};
    use vw_config_types::channels::MatrixConfig;

    fn config(user_id: Option<&str>) -> MatrixConfig {
        MatrixConfig {
            homeserver: "https://matrix.test".to_string(),
            access_token: "matrix-token".to_string(),
            user_id: user_id.map(ToString::to_string),
            device_id: Some("DEVICE".to_string()),
            room_id: "!room:matrix.test".to_string(),
            allowed_users: vec!["@alice:matrix.test".to_string()],
            mention_only: false,
        }
    }

    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-matrix-boundary".to_string();
    task.status = TaskStatus::Pending;

    app.channels_settings.matrix = Some(config(Some("@bot:matrix.test")));
    assert!(
        super::build_task_pool_matrix_publish_request(&app, &task, Some(TaskStatus::Pending))
            .is_none()
    );

    app.channels_settings.matrix = Some(config(None));
    task.set_status(TaskStatus::Running);
    assert!(
        super::build_task_pool_matrix_publish_request(&app, &task, Some(TaskStatus::Pending))
            .is_none()
    );

    app.channels_settings.matrix = None;
    assert!(
        super::build_task_pool_matrix_publish_request(&app, &task, Some(TaskStatus::Pending))
            .is_none()
    );
}

#[test]
fn matrix_publish_target_uses_default_staff_binding_and_rejects_missing_named_instance() {
    use crate::app::task::{Task, TaskStatus};
    use vw_config_types::channels::MatrixConfig;
    use vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID;
    use vw_config_types::staff::StaffChannelBindingConfig;

    let (mut app, _task) = crate::app::App::new();
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.test".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@default-bot:matrix.test".to_string()),
        device_id: Some("DEVICE".to_string()),
        room_id: "!room:matrix.test".to_string(),
        allowed_users: vec!["@alice:matrix.test".to_string()],
        mention_only: false,
    });

    let planner = app
        .task_board_staff_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner staff member");
    planner.current_task_ids = vec!["T-default-matrix".to_string()];
    planner.channel_bindings = vec![StaffChannelBindingConfig {
        channel_type: " matrix ".to_string(),
        instance_id: format!(" {DEFAULT_CHANNEL_INSTANCE_ID} "),
    }];

    let mut task = Task::new(1);
    task.id = "T-default-matrix".to_string();
    task.status = TaskStatus::Pending;
    task.set_status(TaskStatus::Running);

    let request =
        super::build_task_pool_matrix_publish_request(&app, &task, Some(TaskStatus::Pending))
            .expect("default staff Matrix binding should use default Matrix config");
    assert_eq!(request.matrix_instance_id.as_deref(), Some(DEFAULT_CHANNEL_INSTANCE_ID));
    assert_eq!(request.event.actor.matrix_user_id, "@default-bot:matrix.test");

    let planner = app
        .task_board_staff_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner staff member");
    planner.channel_bindings[0].instance_id = "missing-instance".to_string();
    assert!(
        super::build_task_pool_matrix_publish_request(&app, &task, Some(TaskStatus::Pending))
            .is_none()
    );
}

#[test]
fn matrix_processed_event_memory_deduplicates_and_bounds_cached_ids() {
    let (mut app, _task) = crate::app::App::new();

    super::remember_task_board_matrix_event(&mut app, "");
    assert!(app.task_board_processed_matrix_event_ids.is_empty());

    for index in 0..(super::TASK_BOARD_MATRIX_PROCESSED_EVENT_LIMIT + 2) {
        super::remember_task_board_matrix_event(&mut app, &format!("$event-{index}"));
    }
    assert_eq!(
        app.task_board_processed_matrix_event_ids.len(),
        super::TASK_BOARD_MATRIX_PROCESSED_EVENT_LIMIT
    );
    assert!(!super::task_board_matrix_event_is_processed(&app, "$event-0"));
    assert!(!super::task_board_matrix_event_is_processed(&app, "$event-1"));
    assert!(super::task_board_matrix_event_is_processed(&app, "$event-2"));

    let before = app.task_board_processed_matrix_event_ids.len();
    super::remember_task_board_matrix_event(&mut app, "$event-2");
    assert_eq!(app.task_board_processed_matrix_event_ids.len(), before);
}

#[test]
fn staff_config_loaded_records_success_and_retarget_errors_per_workflow() {
    use crate::app::task::TaskBoardStaffConfig;

    let (mut app, _task) = crate::app::App::new();
    super::apply_staff_config_loaded(
        &mut app,
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID,
        Ok(TaskBoardStaffConfig::default()),
    );
    assert_eq!(
        app.task_board_staff_config_workflow_id.as_deref(),
        Some(crate::app::task::DEFAULT_TASK_WORKFLOW_ID)
    );
    assert!(app.task_board_staff_config_load_error.is_none());
    assert!(!app.task_board_staff_projection.lanes.is_empty());

    super::apply_staff_config_loaded(
        &mut app,
        "missing-workflow",
        Ok(TaskBoardStaffConfig::default()),
    );
    assert_eq!(app.task_board_staff_config_workflow_id.as_deref(), Some("missing-workflow"));
    assert!(
        app.task_board_staff_config_load_error
            .as_deref()
            .is_some_and(|error| error.contains("missing-workflow"))
    );

    super::apply_staff_config_loaded(
        &mut app,
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID,
        Err("gateway unavailable".to_string()),
    );
    assert_eq!(
        app.task_board_staff_config_workflow_id.as_deref(),
        Some(crate::app::task::DEFAULT_TASK_WORKFLOW_ID)
    );
    assert_eq!(app.task_board_staff_config_load_error.as_deref(), Some("gateway unavailable"));
}

#[test]
fn staff_stage_action_drafts_round_trip_bindings_and_prune_to_workflow_statuses() {
    use crate::app::task::{
        DEFAULT_TASK_WORKFLOW_ID, TaskBoardStaffConfig, TaskBoardStageAction,
        TaskBoardStageAssignment, TaskBoardStageWorkflowBinding, TaskStatus, task_workflow_by_id,
    };
    use serde_json::json;

    let workflow = task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID).expect("default workflow");
    let mut config = TaskBoardStaffConfig::default();
    config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: "code".to_string(),
        workflow_uuid: None,
        input_parameters: [("branch".to_string(), json!("feature"))].into(),
    }];
    config.stage_workflow_bindings = vec![
        TaskBoardStageWorkflowBinding {
            status: TaskStatus::Planning,
            workflow_uuid: " wf-plan ".to_string(),
        },
        TaskBoardStageWorkflowBinding {
            status: TaskStatus::Pending,
            workflow_uuid: "ignored-existing-action".to_string(),
        },
        TaskBoardStageWorkflowBinding {
            status: TaskStatus::Running,
            workflow_uuid: "  ".to_string(),
        },
    ];

    let mut actions = super::stage_action_draft_from_config(&config);
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|action| {
        action.status == TaskStatus::Planning
            && action.capability_id == super::SAVED_WORKFLOW_CAPABILITY_ID
            && action.workflow_uuid.as_deref() == Some("wf-plan")
    }));

    super::set_stage_action_parameter(
        &mut actions,
        TaskStatus::Pending,
        " prompt ".to_string(),
        " summarize ".to_string(),
    );
    assert_eq!(
        actions
            .iter()
            .find(|action| action.status == TaskStatus::Pending)
            .and_then(|action| action.input_parameters.get("prompt")),
        Some(&json!("summarize"))
    );

    super::set_stage_action_capability(
        &mut actions,
        TaskStatus::Pending,
        Some(" review ".to_string()),
        false,
    );
    let pending = actions.iter().find(|action| action.status == TaskStatus::Pending).unwrap();
    assert_eq!(pending.capability_id, "review");
    assert!(pending.workflow_uuid.is_none());
    assert!(pending.input_parameters.is_empty());

    super::set_stage_action_workflow(
        &mut actions,
        TaskStatus::CodeReview,
        Some(" wf-review ".to_string()),
    );
    let bindings = super::stage_workflow_bindings_for_stage_actions(&actions);
    assert!(bindings.iter().any(|binding| {
        binding.status == TaskStatus::Planning && binding.workflow_uuid == "wf-plan"
    }));
    assert!(bindings.iter().any(|binding| {
        binding.status == TaskStatus::CodeReview && binding.workflow_uuid == "wf-review"
    }));

    actions.push(TaskBoardStageAction {
        status: TaskStatus::Custom("outside"),
        capability_id: "outside".to_string(),
        workflow_uuid: None,
        input_parameters: Default::default(),
    });
    super::prune_stage_action_draft(&mut actions, &workflow);
    assert!(actions.iter().all(|action| action.status != TaskStatus::Custom("outside")));
    assert_eq!(actions[0].status, TaskStatus::Pending);

    let mut assignments = vec![
        TaskBoardStageAssignment {
            status: TaskStatus::CodeReview,
            primary_member_id: Some("staff-reviewer".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        TaskBoardStageAssignment {
            status: TaskStatus::Pending,
            primary_member_id: None,
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        TaskBoardStageAssignment {
            status: TaskStatus::Custom("outside"),
            primary_member_id: Some("staff-outsider".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
    ];
    super::prune_stage_assignment_draft(&mut assignments, &workflow);
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].status, TaskStatus::CodeReview);
}

#[test]
fn staff_command_gateway_round_trip_preserves_each_command_variant() {
    use crate::app::task::TaskBoardStaffCommand;
    use vw_gateway_client::vw_api_types::staff::StaffCommandDto;

    let commands = vec![
        TaskBoardStaffCommand::AssignTask {
            command_id: "cmd-assign".to_string(),
            task_id: "T-1".to_string(),
            member_id: "staff-planner".to_string(),
        },
        TaskBoardStaffCommand::TakeOverTask {
            command_id: "cmd-take-over".to_string(),
            task_id: "T-2".to_string(),
            member_id: "staff-reviewer".to_string(),
        },
        TaskBoardStaffCommand::PauseTask {
            command_id: "cmd-pause".to_string(),
            task_id: "T-3".to_string(),
            member_id: "staff-manager".to_string(),
        },
        TaskBoardStaffCommand::ResumeTask {
            command_id: "cmd-resume".to_string(),
            task_id: "T-4".to_string(),
            member_id: "staff-manager".to_string(),
        },
        TaskBoardStaffCommand::StartTask {
            command_id: "cmd-start".to_string(),
            task_id: "T-5".to_string(),
            member_id: "staff-executor".to_string(),
        },
        TaskBoardStaffCommand::OpenTaskLogs {
            command_id: "cmd-logs".to_string(),
            task_id: "T-6".to_string(),
        },
        TaskBoardStaffCommand::FilterByMember {
            command_id: "cmd-filter".to_string(),
            member_id: "staff-planner".to_string(),
        },
    ];

    for command in commands {
        let dto = super::staff_command_to_gateway(&command);
        assert_eq!(super::staff_command_from_gateway(dto), command);
    }

    assert!(matches!(
        super::staff_command_to_gateway(&TaskBoardStaffCommand::OpenTaskLogs {
            command_id: "cmd-logs".to_string(),
            task_id: "T-6".to_string(),
        }),
        StaffCommandDto::OpenTaskLogs { command_id, task_id }
            if command_id == "cmd-logs" && task_id == "T-6"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
mod task_board_worktree_gateway {
    use crate::app::Message;
    use crate::app::state::TaskBoardViewMode;
    use crate::app::task::{
        Task, TaskBoardStaffCommand, TaskBoardStaffConfig, TaskBoardStageAssignment, TaskStatus,
        WorktreePoolSnapshot, WorktreeSlotSnapshot, WorktreeState,
    };
    use iced_runtime::Action;
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::ffi::{OsStr, OsString};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerDispatchModeDto, AgentSchedulerWakeResponse,
    };
    use vw_gateway_client::vw_api_types::id::ProjectId;

    use super::super::TaskBoardMessage;

    #[derive(Debug, Clone)]
    struct RecordedRequest {
        method: String,
        path: String,
        body: Value,
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    struct TestGateway {
        _home: tempfile::TempDir,
        _home_guard: EnvGuard,
        _config_guard: EnvGuard,
        rx: mpsc::Receiver<RecordedRequest>,
        handle: std::thread::JoinHandle<()>,
    }

    struct TestGatewayResponse {
        status: u16,
        body: Value,
    }

    impl TestGatewayResponse {
        fn ok(body: Value) -> Self {
            Self { status: 200, body }
        }

        fn with_status(status: u16, body: Value) -> Self {
            Self { status, body }
        }
    }

    impl TestGateway {
        fn start(responses: Vec<Value>) -> Self {
            Self::start_http(responses.into_iter().map(TestGatewayResponse::ok).collect())
        }

        fn start_http(responses: Vec<TestGatewayResponse>) -> Self {
            let home = tempfile::tempdir().expect("temp home");
            let home_guard = EnvGuard::set("HOME", home.path());
            let config_guard = EnvGuard::remove("VIBEWINDOW_CONFIG");
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway stub");
            let port = listener.local_addr().expect("local addr").port();
            write_gateway_bootstrap_config(home.path(), port);

            let (tx, rx) = mpsc::channel();
            let handle = std::thread::spawn(move || {
                let mut responses = std::collections::VecDeque::from(responses);
                listener.set_nonblocking(true).expect("nonblocking gateway stub");
                let started_at = Instant::now();
                let mut last_activity_at = Instant::now();
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).expect("blocking gateway stream");
                            let request = read_request(&mut stream);
                            let response = if is_task_pool_request(&request) {
                                responses
                                    .pop_front()
                                    .unwrap_or_else(|| TestGatewayResponse::ok(json!({})))
                            } else {
                                TestGatewayResponse::ok(json!({}))
                            };
                            tx.send(request).expect("record request");
                            write_response(
                                &mut stream,
                                response.status,
                                &response.body.to_string(),
                            );
                            last_activity_at = Instant::now();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if responses.is_empty()
                                && last_activity_at.elapsed() >= Duration::from_millis(150)
                            {
                                break;
                            }
                            if started_at.elapsed() >= Duration::from_secs(1)
                                && last_activity_at.elapsed() >= Duration::from_secs(1)
                            {
                                break;
                            }
                            if started_at.elapsed() >= Duration::from_secs(5) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("accept request: {error}"),
                    }
                }
            });

            Self { _home: home, _home_guard: home_guard, _config_guard: config_guard, rx, handle }
        }

        fn take_requests(self) -> Vec<RecordedRequest> {
            self.handle.join().expect("gateway stub thread");
            let mut requests = Vec::new();
            while let Ok(request) = self.rx.try_recv() {
                requests.push(request);
            }
            requests
        }
    }

    fn write_gateway_bootstrap_config(home: &std::path::Path, port: u16) {
        let mut config = GatewayClientSystemSettingsConfig::default();
        config.set_servers(
            vec![GatewayClientServerConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                bearer_token: "test-token".to_string(),
                ..GatewayClientServerConfig::default()
            }],
            "test".to_string(),
        );
        let path =
            vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json");
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        std::fs::write(path, serde_json::to_string(&config).expect("serialize config"))
            .expect("write config");
    }

    fn is_task_pool_request(request: &RecordedRequest) -> bool {
        request.path.starts_with("/v1/projects/resolve")
            || request.path.starts_with("/v1/agent-scheduler")
            || request.path.starts_with("/v1/staff")
            || request.path.starts_with("/v1/task-pool/worktrees")
    }

    fn read_request(stream: &mut TcpStream) -> RecordedRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        let mut header_end = None;
        while header_end.is_none() {
            let read = stream.read(&mut buffer).expect("read request");
            assert!(read > 0, "connection closed before headers");
            bytes.extend_from_slice(&buffer[..read]);
            header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
        }

        let header_end = header_end.expect("headers");
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
        let mut lines = headers.lines();
        let request_line = lines.next().expect("request line");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().expect("method").to_string();
        let path = parts.next().expect("path").to_string();
        let content_length = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let mut body = bytes[(header_end + 4)..].to_vec();
        while body.len() < content_length {
            let read = stream.read(&mut buffer).expect("read body");
            assert!(read > 0, "connection closed before body");
            body.extend_from_slice(&buffer[..read]);
        }
        body.truncate(content_length);
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).expect("request json body")
        };

        RecordedRequest { method, path, body }
    }

    fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
        let response = format!(
            "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("write response");
    }

    fn project_response(project_path: &str) -> Value {
        json!({
            "project": {
                "id": "project-gateway",
                "name": "Gateway Project",
                "directory": project_path,
                "display_path": project_path,
                "status": "ready",
                "created_at_ms": 1,
                "updated_at_ms": 2,
                "git": {
                    "is_repo": true,
                    "has_uncommitted_changes": false
                }
            }
        })
    }

    fn snapshot_response(project_path: &str) -> Value {
        json!({
            "project_id": "project-gateway",
            "repo_root": project_path,
            "pool_root": format!("{project_path}/.vibe-window/task-worktrees"),
            "base_branch": "main",
            "idle_count": 1,
            "busy_count": 1,
            "tainted_count": 0,
            "recycling_count": 0,
            "dead_count": 0,
            "merge_target_locks": [
                {"target_branch": "main", "task_id": "T-lock"}
            ],
            "slots": [
                {
                    "id": "slot-1",
                    "path": format!("{project_path}/slot-1"),
                    "branch": "vw/slot-1",
                    "state": "idle"
                },
                {
                    "id": "slot-2",
                    "path": format!("{project_path}/slot-2"),
                    "branch": "vw/slot-2",
                    "state": "busy",
                    "leased_task_id": "T-busy"
                }
            ]
        })
    }

    fn runtime_status_response(project_path: &str, task_id: &str, status: &str) -> Value {
        let agent_status = match status {
            "pool" => "draft",
            "pending" => "queued",
            "planning" | "running" => "running",
            "code_complete" | "code_review" | "pr_submitted" => "submitted",
            "paused" => "paused",
            "failed" => "failed",
            "completed" => "completed",
            "archived" => "archived",
            value => value,
        };
        json!({
        "project_id": "project-gateway",
        "tasks": [{
            "project_id": "project-gateway",
            "task_id": task_id,
            "status_key": status,
            "status": agent_status,
            "priority": 7,
            "order_no": 3,
            "model": "gpt-runtime",
        "prompt": "Runtime-owned task prompt",
        "retry_count": 2,
        "run_id": "run-gateway",
        "run_stage": "running",
        "workspace": {
            "slot_id": "slot-runtime",
            "path": format!("{project_path}/slot-runtime"),
            "branch": "vw/task/runtime"
        },
        "selected_worktree_path": format!("{project_path}/slot-runtime"),
        "selected_worktree_slot_id": "slot-runtime",
        "selected_worktree_branch": "vw/task/runtime",
        "merge_source_branch": "vw/task/runtime",
        "merge_target_branch": "main",
        "last_error": "runtime retry note",
        "updated_at_ms": 44
        }]
        })
    }

    fn runtime_status_response_with_workspace(
        project_path: &str,
        task_id: &str,
        status: &str,
        slot_id: &str,
        branch: &str,
    ) -> Value {
        let mut response = runtime_status_response(project_path, task_id, status);
        let path = format!("{project_path}/{slot_id}");
        response["tasks"][0]["workspace"]["slot_id"] = json!(slot_id);
        response["tasks"][0]["workspace"]["path"] = json!(path);
        response["tasks"][0]["workspace"]["branch"] = json!(branch);
        response["tasks"][0]["selected_worktree_path"] = json!(format!("{project_path}/{slot_id}"));
        response["tasks"][0]["selected_worktree_slot_id"] = json!(slot_id);
        response["tasks"][0]["selected_worktree_branch"] = json!(branch);
        response["tasks"][0]["merge_source_branch"] = json!(branch);
        response
    }

    fn runtime_logs_response(
        task_id: &str,
        from_status_key: &str,
        to_status_key: &str,
        message: &str,
    ) -> Value {
        json!({
            "task_id": task_id,
            "logs": [{
                "task_id": task_id,
                "seq_no": 1,
                "timestamp_ms": 1_783_000_903_000_i64,
                "from_status_key": from_status_key,
                "to_status_key": to_status_key,
                "message": message
            }]
        })
    }

    fn agent_scheduler_create_response(task_id: &str) -> Value {
        json!({
            "task_id": task_id,
            "task": {
                "id": task_id,
                "title": format!("Task {task_id}"),
                "kind": "finite",
                "status": "queued",
                "created_at_ms": 1,
                "updated_at_ms": 1,
                "metadata": {
                    "prompt": format!("Prompt {task_id}")
                }
            }
        })
    }

    fn wake_response(task_id: &str) -> AgentSchedulerWakeResponse {
        AgentSchedulerWakeResponse {
            dispatched: true,
            task_id: Some(task_id.to_string()),
            run_id: Some(format!("run-{task_id}")),
            worker_id: Some("gateway-local".to_string()),
            outbox_message_id: None,
        }
    }

    fn wake_response_value(task_id: &str) -> Value {
        serde_json::to_value(wake_response(task_id)).expect("wake response json")
    }

    fn matrix_wake_response_value(task_id: &str, outbox_message_id: &str) -> Value {
        let mut response = wake_response(task_id);
        response.worker_id = Some("matrix-worker".to_string());
        response.outbox_message_id = Some(outbox_message_id.to_string());
        serde_json::to_value(response).expect("matrix wake response json")
    }

    fn pending_task(id: &str) -> Task {
        let mut task = Task::new(1);
        task.id = id.to_string();
        task.status = TaskStatus::Pending;
        task.prompt = "Do the gateway worktree task".to_string();
        task.description = "Gateway worktree task".to_string();
        task.model = "auto".to_string();
        task
    }

    fn merge_task(id: &str) -> Task {
        let mut task = Task::new(1);
        task.id = id.to_string();
        task.status = TaskStatus::PrSubmitted;
        task.prompt = "Merge task".to_string();
        task.description = "Merge task".to_string();
        task.merge_source_branch = Some(format!("vw/task/{id}"));
        task.merge_target_branch = Some("main".to_string());
        task.selected_worktree_path = Some("/tmp/merge-worktree".to_string());
        task
    }

    fn app_init_responses() -> Vec<Value> {
        Vec::new()
    }

    fn task_pool_requests(requests: &[RecordedRequest]) -> Vec<&RecordedRequest> {
        requests
            .iter()
            .filter(|request| {
                request.path.starts_with("/v1/projects/resolve")
                    || request.path.starts_with("/v1/task-pool/worktrees")
            })
            .collect()
    }

    fn task_board_scheduler_requests(requests: &[RecordedRequest]) -> Vec<&RecordedRequest> {
        requests.iter().filter(|request| request.path.starts_with("/v1/agent-scheduler")).collect()
    }

    async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
        let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
            return Vec::new();
        };
        while let Some(action) = stream.next().await {
            if let Action::Output(messages) = action {
                return messages;
            }
        }
        Vec::new()
    }

    fn apply_messages(app: &mut crate::app::App, messages: Vec<Message>) {
        for message in messages {
            if let Message::TaskBoard(task_message) = message {
                let _ = super::super::update(app, task_message);
            }
        }
    }

    async fn collect_next_task_board_messages(task: iced::Task<Message>) -> Vec<TaskBoardMessage> {
        let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
            return Vec::new();
        };
        while let Some(action) = stream.next().await {
            if let Action::Output(outputs) = action {
                let messages = outputs.into_iter().filter_map(|message| match message {
                    Message::TaskBoard(task_message) => Some(task_message),
                    _ => None,
                });
                let messages = messages.collect::<Vec<_>>();
                if !messages.is_empty() {
                    return messages;
                }
            }
        }
        Vec::new()
    }

    async fn apply_task_board_task_once(
        app: &mut crate::app::App,
        task: iced::Task<Message>,
    ) -> Vec<TaskBoardMessage> {
        let messages = collect_next_task_board_messages(task).await;
        for message in messages.iter().cloned() {
            let _ = super::super::update(app, message);
        }
        messages
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn execution_tick_registers_pool_and_pending_tasks_before_agent_scheduler_wake() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-register-pool"),
            agent_scheduler_create_response("T-register-pending"),
            wake_response_value("T-register-pool"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        app.task_board_settings.max_concurrent = 2;

        let mut pending = pending_task("T-register-pending");
        pending.priority = 9;
        pending.order = 4;
        pending.model = "gpt-register".to_string();
        pending.agent = Some("planner".to_string());
        pending.acp_agent = Some("codex".to_string());
        pending.merge_source_branch = Some("vw/task/T-register-pending".to_string());
        pending.merge_target_branch = Some("main".to_string());
        pending.selected_worktree_path = Some(format!("{project_path}/slot-register"));
        pending.selected_worktree_slot_id = Some("slot-register".to_string());
        pending.selected_worktree_branch = Some("vw/task/register".to_string());

        let mut pool = pending_task("T-register-pool");
        pool.status = TaskStatus::Pool;
        let mut completed = pending_task("T-register-completed");
        completed.status = TaskStatus::Completed;
        let mut archived = pending_task("T-register-archived");
        archived.status = TaskStatus::Archived;
        app.task_board_tasks = vec![pool, pending, completed, archived];

        let task = super::super::update(&mut app, TaskBoardMessage::ExecutionTick);
        let _messages = collect_task_messages(task).await;

        assert!(app.task_board_executor.running_tasks.is_empty());

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        let create_index = scheduler_requests
            .iter()
            .position(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks")
            .expect("pending task create request");
        let wake_index = scheduler_requests
            .iter()
            .position(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/wake")
            .expect("agent scheduler wake request");
        assert_eq!(scheduler_requests[wake_index].body["max_concurrent"], 2);
        assert_eq!(scheduler_requests[wake_index].body["auto_promote_pool_tasks"], true);
        assert!(
            create_index < wake_index,
            "pending task must be registered before wake: {:?}",
            scheduler_requests
        );
        assert_eq!(
            scheduler_requests
                .iter()
                .filter(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks")
                .count(),
            2
        );
        let create_requests = scheduler_requests
            .iter()
            .filter(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks")
            .collect::<Vec<_>>();
        assert!(create_requests.iter().all(|request| request.method == "POST"));
        assert!(create_requests.iter().any(|request| {
            request.body["task_id"] == "T-register-pool" && request.body["status"] == "draft"
        }));
        assert!(create_requests.iter().any(|request| {
            request.body["task_id"] == "T-register-pending"
                && request.body["status"] == "queued"
        }));
        let create_request = create_requests
            .iter()
            .find(|request| request.body["task_id"] == "T-register-pending")
            .expect("pending task create request");
        assert_eq!(create_request.body["kind"], "finite");
        assert_eq!(create_request.body["status"], "queued");
        assert_eq!(create_request.body["metadata"]["prompt"], "Do the gateway worktree task");
        assert_eq!(create_request.body["metadata"]["priority"], 9);
        assert_eq!(create_request.body["metadata"]["order_no"], 4);
        assert_eq!(create_request.body["metadata"]["model"], "gpt-register");
        assert_eq!(create_request.body["metadata"]["agent"], "planner");
        assert_eq!(create_request.body["metadata"]["acp_agent"], "codex");
        assert_eq!(
            create_request.body["metadata"]["merge_source_branch"],
            "vw/task/T-register-pending"
        );
        assert_eq!(create_request.body["metadata"]["merge_target_branch"], "main");
        assert_eq!(
            create_request.body["metadata"]["selected_worktree_path"],
            format!("{project_path}/slot-register")
        );
        assert_eq!(create_request.body["metadata"]["selected_worktree_slot_id"], "slot-register");
        assert_eq!(create_request.body["metadata"]["selected_worktree_branch"], "vw/task/register");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn matrix_execution_wake_drains_agent_scheduler_outbox() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-matrix-wake"),
            matrix_wake_response_value("T-matrix-wake", "outbox-matrix-wake"),
            json!({"claimed": 1, "messages": []}),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        app.task_board_settings.max_concurrent = 2;
        app.task_board_settings.agent_scheduler_dispatch_mode =
            AgentSchedulerDispatchModeDto::Matrix;
        app.task_board_tasks = vec![pending_task("T-matrix-wake")];

        let task = super::super::update(&mut app, TaskBoardMessage::ExecutionTick);
        let messages = collect_task_messages(task).await;
        assert!(
            messages.iter().any(|message| matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(response)))
                    if response.outbox_message_id.as_deref() == Some("outbox-matrix-wake")
            )),
            "wake messages: {:?}",
            messages
        );

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        let wake_index = scheduler_requests
            .iter()
            .position(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/wake")
            .expect("agent scheduler wake request");
        let drain_index = scheduler_requests
            .iter()
            .position(|request| {
                request.path == "/v1/agent-scheduler/projects/project-gateway/outbox/drain"
            })
            .expect("agent scheduler outbox drain request");
        assert!(
            wake_index < drain_index,
            "outbox drain must follow Matrix wake: {:?}",
            scheduler_requests
        );
        assert_eq!(scheduler_requests[wake_index].body["dispatch_mode"], "matrix");
        assert_eq!(scheduler_requests[drain_index].method, "POST");
        assert_eq!(scheduler_requests[drain_index].body["limit"], 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn view_task_logs_requests_agent_scheduler_logs_for_current_task() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            runtime_logs_response(
                "T-detail-logs",
                "running",
                "code_complete",
                "gateway detail log visible",
            ),
            project_response(project_path),
            runtime_logs_response(
                "T-detail-logs",
                "running",
                "code_complete",
                "gateway detail log visible",
            ),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_tasks = vec![pending_task("T-detail-logs")];

        let task = super::super::update(
            &mut app,
            TaskBoardMessage::ViewTaskLogs("T-detail-logs".to_string()),
        );
        apply_messages(&mut app, collect_task_messages(task).await);
        let task = super::super::update(
            &mut app,
            TaskBoardMessage::SelectDetailTab(super::super::TaskBoardDetailTab::Logs),
        );
        apply_messages(&mut app, collect_task_messages(task).await);

        assert_eq!(app.task_board_detail_tab, super::super::TaskBoardDetailTab::Logs);
        let requests = gateway.take_requests();
        assert!(
            requests.iter().any(|request| request.path == "/v1/projects/resolve"),
            "viewing logs should resolve current project: {:?}",
            requests
        );
        let logs_request_count = requests
            .iter()
            .filter(|request| {
                request.method == "GET"
                    && request.path
                        == "/v1/agent-scheduler/projects/project-gateway/tasks/T-detail-logs/logs"
            })
            .count();
        assert_eq!(
            logs_request_count, 2,
            "viewing logs should request gateway logs for current task: {:?}",
            requests
        );
        assert!(
            app.task_board_logs_editor.text().contains("gateway detail log visible"),
            "gateway logs should be visible in detail editor: {}",
            app.task_board_logs_editor.text()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn promote_tick_requests_pool_promotion_without_legacy_bulk_command() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-promote-pool"),
            wake_response_value("T-promote-pool"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_settings.auto_promote_delay_seconds = 0;

        let mut pool = pending_task("T-promote-pool");
        pool.status = TaskStatus::Pool;
        pool.priority = 8;
        pool.order = 5;
        app.task_board_tasks = vec![pool];

        let task = super::super::update(&mut app, TaskBoardMessage::PromotePoolTasksTick);
        let _messages = collect_task_messages(task).await;

        assert_eq!(app.task_board_tasks[0].status, TaskStatus::Pool);
        assert!(
            app.task_board_tasks[0]
                .logs
                .iter()
                .all(|entry| { entry.message != "自动执行任务池：已发送提升命令" })
        );

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(
            scheduler_requests.iter().all(|request| {
                request.path != "/v1/agent-scheduler/projects/project-gateway/commands/bulk"
            }),
            "legacy promote bulk command must not be sent: {:?}",
            scheduler_requests
        );
        assert!(
            scheduler_requests
                .iter()
                .any(|request| {
                    request.path == "/v1/agent-scheduler/projects/project-gateway/wake"
                        && request.body["auto_promote_pool_tasks"] == true
                })
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_pool_local_pipeline_runs_pending_to_completed() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let task_id = "T-local-loop";
        let slot_id = "slot-local-loop";
        let branch = "vw/task/T-local-loop";
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response(task_id),
            wake_response_value(task_id),
            project_response(project_path),
            runtime_status_response_with_workspace(
                project_path,
                task_id,
                "code_complete",
                slot_id,
                branch,
            ),
            runtime_logs_response(
                task_id,
                "running",
                "code_complete",
                "local execution submitted result",
            ),
            project_response(project_path),
            runtime_status_response_with_workspace(
                project_path,
                task_id,
                "completed",
                slot_id,
                branch,
            ),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        app.task_board_settings.agent_scheduler_dispatch_mode =
            AgentSchedulerDispatchModeDto::Local;
        let mut pending = pending_task(task_id);
        pending.priority = 8;
        pending.order = 5;
        app.task_board_tasks = vec![pending];

        let wake_task = super::super::update(&mut app, TaskBoardMessage::ExecutionTick);
        let wake_messages = apply_task_board_task_once(&mut app, wake_task).await;
        assert!(
            wake_messages.iter().any(|message| matches!(
                message,
                TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(response))
                    if response.task_id.as_deref() == Some(task_id)
            )),
            "wake messages: {:?}",
            wake_messages
        );

        let status_task =
            super::super::super::helpers::sync_agent_scheduler_status(project_path.to_string());
        let status_messages = apply_task_board_task_once(&mut app, status_task).await;
        assert!(
            status_messages.iter().any(|message| matches!(
                message,
                TaskBoardMessage::AgentSchedulerStatusLoaded { .. }
            )),
            "status messages: {:?}",
            status_messages
        );
        let project_id = ProjectId("project-gateway".to_string());
        let logs_task = super::super::super::helpers::sync_agent_scheduler_logs(
            project_path.to_string(),
            project_id.clone(),
            vec![task_id.to_string()],
        );
        let _ = apply_task_board_task_once(&mut app, logs_task).await;

        let task = app
            .task_board_tasks
            .iter()
            .find(|task| task.id == task_id)
            .expect("code complete task");
        assert_eq!(task.status, TaskStatus::CodeComplete);
        assert_eq!(
            task.selected_worktree_path.as_deref(),
            Some(format!("{project_path}/{slot_id}").as_str())
        );
        assert_eq!(task.selected_worktree_branch.as_deref(), Some(branch));
        assert!(task.logs.iter().any(|entry| {
            entry.message == "local execution submitted result"
                && entry.status_from == Some(TaskStatus::Running)
                && entry.status_to == Some(TaskStatus::CodeComplete)
        }));

        let status_task = super::super::super::helpers::sync_agent_scheduler_status(
            project_path.to_string(),
        );
        let status_messages = apply_task_board_task_once(&mut app, status_task).await;
        assert!(
            status_messages.iter().any(|message| matches!(
                message,
                TaskBoardMessage::AgentSchedulerStatusLoaded { result: Ok(_), .. }
            )),
            "status messages: {:?}",
            status_messages
        );
        let task =
            app.task_board_tasks.iter().find(|task| task.id == task_id).expect("completed task");
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.selected_worktree_branch.as_deref(), Some(branch));
        assert!(!app.task_board_executor.running_tasks.contains(&task_id.to_string()));

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        let scheduler_steps = scheduler_requests
            .iter()
            .map(|request| (request.method.as_str(), request.path.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            scheduler_steps,
            vec![
                ("POST", "/v1/agent-scheduler/projects/project-gateway/tasks"),
                ("POST", "/v1/agent-scheduler/projects/project-gateway/wake"),
                ("GET", "/v1/agent-scheduler/projects/project-gateway/tasks"),
                ("GET", "/v1/agent-scheduler/projects/project-gateway/tasks/T-local-loop/logs"),
                ("GET", "/v1/agent-scheduler/projects/project-gateway/tasks"),
            ]
        );
        assert!(
            requests.iter().all(|request| {
                !request.path.starts_with("/v1/task-pool/worktrees/acquire")
                    && !request.path.starts_with("/v1/task-pool/worktrees/merge-lock")
            }),
            "local pipeline must not use desktop worktree acquire or merge lock: {:?}",
            requests
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_execution_uses_agent_scheduler_native_client() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-agent-scheduler-start"),
            json!({"accepted": true}),
            runtime_status_response(project_path, "T-agent-scheduler-start", "running"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_tasks = vec![pending_task("T-agent-scheduler-start")];

        let update_task = super::super::update(
            &mut app,
            TaskBoardMessage::ExecuteTask { task_id: "T-agent-scheduler-start".to_string() },
        );
        apply_messages(&mut app, collect_task_messages(update_task).await);

        let board_task = app
            .task_board_tasks
            .iter()
            .find(|task| task.id == "T-agent-scheduler-start")
            .expect("agent scheduler task");
        assert_eq!(board_task.status, TaskStatus::Running);
        assert_eq!(board_task.selected_worktree_slot_id.as_deref(), Some("slot-runtime"));

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(
            scheduler_requests
                .iter()
                .all(|request| request.path.starts_with("/v1/agent-scheduler")),
            "execution must use agent scheduler requests: {:?}",
            scheduler_requests
        );
        let create_request = scheduler_requests
            .iter()
            .find(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks")
            .expect("agent scheduler create request");
        assert_eq!(create_request.method, "POST");
        assert_eq!(create_request.body["task_id"], "T-agent-scheduler-start");
        assert_eq!(create_request.body["metadata"]["prompt"], "Do the gateway worktree task");
        let command_request = scheduler_requests
            .iter()
            .find(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/commands")
            .expect("agent scheduler command request");
        assert_eq!(command_request.method, "POST");
        assert_eq!(command_request.body["task_id"], "T-agent-scheduler-start");
        assert_eq!(command_request.body["command"], "start");
        assert!(
	            scheduler_requests
	                .iter()
	                .any(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks"),
	            "requests: {:?}",
	            scheduler_requests
	        );
        assert!(
            requests.iter().all(|request| {
                !request.path.starts_with("/v1/task-pool")
                    && !request.path.to_ascii_lowercase().contains("matrix")
            }),
            "execution refresh must not call legacy task-pool or Matrix routes: {:?}",
            requests
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn staff_stage_assignment_persisted_auto_schedule_uses_agent_scheduler_path() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            json!({
                "project_id": "project-gateway",
                "plan": {
                    "commands": [{
                        "type": "start_task",
                        "command_id": "staff-auto-start-T-staff-auto-start",
                        "task_id": "T-staff-auto-start",
                        "member_id": "staff-orchestrator"
                    }],
                    "command_contexts": [],
                    "skipped": []
                }
            }),
            project_response(project_path),
            json!({
                "accepted": true,
                "message": "staff command accepted",
                "action": {
                    "type": "execute_task",
                    "task_id": "T-staff-auto-start"
                },
                "scheduler_result": {
                    "accepted": true
                }
            }),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_view_mode = TaskBoardViewMode::Staff;
        app.task_board_staff_config = TaskBoardStaffConfig::default();
        app.task_board_staff_config.stage_assignments = vec![TaskBoardStageAssignment {
            status: TaskStatus::Pending,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }];
        let member = app
            .task_board_staff_config
            .members
            .iter_mut()
            .find(|member| member.id == "staff-orchestrator")
            .expect("default orchestrator");
        member.current_task_ids = vec!["T-staff-auto-start".to_string()];
        app.task_board_tasks = vec![pending_task("T-staff-auto-start")];

        let auto_task =
            super::super::update(&mut app, TaskBoardMessage::StaffStageAssignmentPersisted(Ok(())));
        let mut auto_messages = Vec::new();
        for message in collect_next_task_board_messages(auto_task).await {
            let task = super::super::update(&mut app, message);
            auto_messages.extend(collect_next_task_board_messages(task).await);
        }
        assert_eq!(auto_messages.len(), 1);
        let command = match auto_messages.into_iter().next().expect("auto schedule message") {
            TaskBoardMessage::SubmitStaffCommandSilently(command) => command,
            message => panic!("expected silent staff command, got {message:?}"),
        };
        match &command {
            TaskBoardStaffCommand::StartTask { task_id, member_id, .. } => {
                assert_eq!(task_id, "T-staff-auto-start");
                assert_eq!(member_id, "staff-orchestrator");
            }
            command => panic!("expected staff start command, got {command:?}"),
        }

        let queue_task =
            super::super::update(&mut app, TaskBoardMessage::SubmitStaffCommandSilently(command));
        let queue_messages = collect_next_task_board_messages(queue_task).await;
        assert!(
            queue_messages
                .iter()
                .any(|message| matches!(message, TaskBoardMessage::StaffCommandQueueTick)),
            "queue messages: {:?}",
            queue_messages
        );
        let mut completion_seen = false;
        for message in queue_messages {
            let task = super::super::update(&mut app, message);
            for task_message in collect_next_task_board_messages(task).await {
                completion_seen |=
                    matches!(task_message, TaskBoardMessage::StaffCommandGatewayCompleted { .. });
                let _ = super::super::update(&mut app, task_message);
            }
        }
        assert!(completion_seen, "staff command should complete through gateway");

        assert_eq!(app.task_board_staff_command_queue.pending_len(), 0);
        assert!(
            app.task_board_staff_events
                .iter()
                .any(|event| event.message.contains("staff command accepted")),
            "staff events: {:?}",
            app.task_board_staff_events
        );

        let requests = gateway.take_requests();
        let auto_schedule_request = requests
            .iter()
            .find(|request| request.path == "/v1/staff/projects/project-gateway/auto-schedule-plan")
            .expect("staff auto schedule request");
        assert_eq!(auto_schedule_request.method, "POST");
        let command_request = requests
            .iter()
            .find(|request| request.path == "/v1/staff/projects/project-gateway/commands")
            .expect("staff command request");
        assert_eq!(command_request.method, "POST");
        assert_eq!(command_request.body["command"]["type"], "start_task");
        assert_eq!(command_request.body["command"]["task_id"], "T-staff-auto-start");
        assert!(
            requests
                .iter()
                .all(|request| !request.path.starts_with("/v1/task-pool/worktrees/acquire")),
            "staff auto schedule must not acquire a worktree: {:?}",
            requests
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn worktree_snapshot_refresh_uses_gateway_client() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_path = "/tmp/vw-task-board-worktree-snapshot-gateway";
        let mut responses = app_init_responses();
        responses.extend([project_response(project_path), snapshot_response(project_path)]);
        let gateway = TestGateway::start(responses);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());

        let task =
            super::super::super::helpers::maybe_schedule_worktree_snapshot_refresh(&mut app, true);
        assert!(app.task_board_worktree_snapshot_loading);
        apply_messages(&mut app, collect_task_messages(task).await);

        assert!(!app.task_board_worktree_snapshot_loading);
        let snapshot = app.task_board_worktree_snapshot.as_ref().expect("gateway snapshot");
        assert_eq!(snapshot.repo_root, project_path);
        assert_eq!(snapshot.idle_count, 1);
        assert_eq!(snapshot.busy_count, 1);
        assert_eq!(snapshot.merge_target_locks, vec![("main".to_string(), "T-lock".to_string())]);
        assert_eq!(snapshot.slots.len(), 2);
        assert_eq!(snapshot.slots[1].leased_task_id.as_deref(), Some("T-busy"));

        let requests = gateway.take_requests();
        let requests = task_pool_requests(&requests);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].path, "/v1/projects/resolve");
        assert_eq!(requests[0].body["directory"], project_path);
        assert_eq!(requests[1].path, "/v1/task-pool/worktrees/snapshot");
        assert_eq!(requests[1].body["project_id"], "project-gateway");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_agent_scheduler_client_does_not_acquire_worktree_on_wake() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            runtime_status_response(project_path, "T-runtime-schedule", "running"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        app.task_board_tasks = vec![pending_task("T-runtime-schedule")];

        let task = super::super::update(
            &mut app,
            TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(wake_response(
                "T-runtime-schedule",
            ))),
        );
        apply_messages(&mut app, collect_task_messages(task).await);

        let task = app
            .task_board_tasks
            .iter()
            .find(|task| task.id == "T-runtime-schedule")
            .expect("runtime task");
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.selected_worktree_slot_id.as_deref(), Some("slot-runtime"));
        assert!(app.task_board_executor.running_tasks.is_empty());

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(
	            scheduler_requests
	                .iter()
	                .any(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks"),
	            "requests: {:?}",
	            scheduler_requests
	        );
        assert!(
            requests
                .iter()
                .all(|request| !request.path.starts_with("/v1/task-pool/worktrees/acquire")),
            "schedule must not acquire a worktree: {:?}",
            requests
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_gateway_runtime_client_does_not_recycle_worktree_on_execution_failure() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway =
            TestGateway::start(vec![project_response(project_path), json!({"recycled": true})]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        let mut task = pending_task("T-runtime-failure");
        task.status = TaskStatus::Running;
        task.selected_worktree_slot_id = Some("slot-runtime".to_string());
        task.selected_worktree_path = Some(format!("{project_path}/slot-runtime"));
        app.task_board_tasks = vec![task];
        app.task_board_executor.start_task("T-runtime-failure");

        let update_task = super::super::update(
            &mut app,
            TaskBoardMessage::TaskExecutionCompleted {
                project_path: project_path.to_string(),
                task_id: "T-runtime-failure".to_string(),
                result: Err("boom".to_string()),
            },
        );
        apply_messages(&mut app, collect_task_messages(update_task).await);

        let requests = gateway.take_requests();
        let task_pool_requests = task_pool_requests(&requests);
        assert!(
            task_pool_requests
                .iter()
                .all(|request| !request.path.starts_with("/v1/task-pool/worktrees/recycle")),
            "execution failure must not recycle a worktree: {:?}",
            task_pool_requests
        );
        let task = app
            .task_board_tasks
            .iter()
            .find(|task| task.id == "T-runtime-failure")
            .expect("runtime task");
        assert!(
            task.logs.iter().all(|entry| !entry.message.contains("准备回收 worktree 槽位")),
            "logs: {:?}",
            task.logs
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_gateway_runtime_client_does_not_lock_merge_target() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-runtime-merge"),
            json!({"accepted": true}),
            runtime_status_response(project_path, "T-runtime-merge", "pr_submitted"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_manual_merge_queue = vec!["T-runtime-merge".to_string()];
        app.task_board_tasks = vec![merge_task("T-runtime-merge")];

        let task = super::super::super::helpers::start_manual_merge_queue_task(&mut app);
        apply_messages(&mut app, collect_task_messages(task).await);

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(
            requests
                .iter()
                .all(|request| !request.path.starts_with("/v1/task-pool/worktrees/merge-lock")),
            "manual merge must not take a desktop merge lock: {:?}",
            requests
        );
        let command_request = scheduler_requests
            .iter()
            .find(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/commands")
            .expect("agent scheduler merge command request");
        assert_eq!(command_request.body["task_id"], "T-runtime-merge");
        assert_eq!(command_request.body["command"], "merge");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_agent_scheduler_client_applies_task_snapshot() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            runtime_status_response(project_path, "T-runtime-snapshot", "running"),
        ]);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_tasks = vec![pending_task("T-runtime-snapshot")];

        let task = super::super::update(
            &mut app,
            TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(wake_response(
                "T-runtime-snapshot",
            ))),
        );
        apply_messages(&mut app, collect_task_messages(task).await);

        let task = app
            .task_board_tasks
            .iter()
            .find(|task| task.id == "T-runtime-snapshot")
            .expect("runtime task");
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.priority, 7);
        assert_eq!(task.order, 3);
        assert_eq!(task.model, "gpt-runtime");
        assert_eq!(task.retry_count, 2);
        assert_eq!(task.last_error.as_deref(), Some("runtime retry note"));
        assert_eq!(task.merge_source_branch.as_deref(), Some("vw/task/runtime"));
        assert_eq!(task.merge_target_branch.as_deref(), Some("main"));
        assert_eq!(
            task.selected_worktree_path.as_deref(),
            Some(format!("{project_path}/slot-runtime").as_str())
        );
        assert!(
            task.logs.iter().any(|entry| entry.message.contains("Agent Scheduler 阶段: running"))
        );

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(
	            scheduler_requests
	                .iter()
	                .any(|request| request.path == "/v1/agent-scheduler/projects/project-gateway/tasks"),
	            "requests: {:?}",
	            scheduler_requests
	        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn worktree_pool_maintenance_uses_gateway_client() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_path = "/tmp/vw-task-board-worktree-maintenance-gateway";
        let mut responses = app_init_responses();
        responses.extend([
            project_response(project_path),
            json!({
                "idle_count": 2,
                "target_idle_count": 3,
                "snapshot": snapshot_response(project_path)
            }),
        ]);
        let gateway = TestGateway::start(responses);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_worktree_desired_capacity = Some(3);

        let task = super::super::super::helpers::schedule_worktree_pool_maintenance(
            &mut app,
            project_path,
            0,
        );
        assert!(app.task_board_worktree_maintenance_in_flight);
        apply_messages(&mut app, collect_task_messages(task).await);

        assert!(!app.task_board_worktree_maintenance_in_flight);
        assert_eq!(
            app.task_board_worktree_snapshot.as_ref().map(|snapshot| snapshot.idle_count),
            Some(1)
        );

        let requests = gateway.take_requests();
        let requests = task_pool_requests(&requests);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path, "/v1/projects/resolve");
        assert_eq!(requests[1].path, "/v1/task-pool/worktrees/maintain");
        assert_eq!(requests[1].body["project_id"], "project-gateway");
        assert_eq!(requests[1].body["desired_capacity"], 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn worktree_manual_recycle_uses_gateway_client() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_path = "/tmp/vw-task-board-worktree-recycle-gateway";
        let mut responses = app_init_responses();
        responses.extend([project_response(project_path), json!({"recycled": true})]);
        let gateway = TestGateway::start(responses);
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_worktree_snapshot = Some(WorktreePoolSnapshot {
            repo_root: project_path.to_string(),
            pool_root: format!("{project_path}/.vibe-window/task-worktrees"),
            base_branch: "main".to_string(),
            idle_count: 1,
            busy_count: 0,
            tainted_count: 0,
            recycling_count: 0,
            dead_count: 0,
            merge_target_locks: Vec::new(),
            slots: vec![WorktreeSlotSnapshot {
                id: "slot-1".to_string(),
                path: format!("{project_path}/slot-1"),
                branch: "vw/slot-1".to_string(),
                state: WorktreeState::Idle,
                leased_task_id: None,
                taint_reason: None,
            }],
        });

        let task = super::super::update(
            &mut app,
            TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "slot-1".to_string() },
        );
        assert_eq!(app.task_board_worktree_manual_action_kind.as_deref(), Some("slot_recycle"));
        apply_messages(&mut app, collect_task_messages(task).await);

        let requests = gateway.take_requests();

        assert!(
            app.notifications
                .iter()
                .any(|notification| notification.message.contains("worktree 槽位已回收")),
            "notifications: {:?}",
            app.notifications
        );

        let requests = task_pool_requests(&requests);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path, "/v1/projects/resolve");
        assert_eq!(requests[1].path, "/v1/task-pool/worktrees/slots/recycle");
        assert_eq!(requests[1].body["project_id"], "project-gateway");
        assert_eq!(requests[1].body["slot_id"], "slot-1");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_execution_agent_scheduler_wake_ignores_legacy_acquire_action() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let (mut app, _task) = crate::app::App::new();
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            runtime_status_response(project_path, "T-acquire", "running"),
        ]);
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        app.task_board_settings.max_concurrent = 1;
        app.task_board_worktree_maintenance_in_flight = true;
        app.task_board_tasks = vec![pending_task("T-acquire")];

        let task = super::super::update(
            &mut app,
            TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(Ok(wake_response("T-acquire"))),
        );
        apply_messages(&mut app, collect_task_messages(task).await);

        let task = app.task_board_tasks.iter().find(|task| task.id == "T-acquire").unwrap();
        assert_eq!(task.status, TaskStatus::Running);
        let expected_path = format!("{project_path}/slot-runtime");
        assert_eq!(task.selected_worktree_path.as_deref(), Some(expected_path.as_str()));
        assert_eq!(task.selected_worktree_slot_id.as_deref(), Some("slot-runtime"));
        assert_eq!(task.selected_worktree_branch.as_deref(), Some("vw/task/runtime"));
        assert!(app.task_board_executor.running_tasks.is_empty());

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert_eq!(scheduler_requests.len(), 1);
        assert_eq!(
            scheduler_requests[0].path,
            "/v1/agent-scheduler/projects/project-gateway/tasks"
        );
        assert!(
            requests
                .iter()
                .all(|request| !request.path.starts_with("/v1/task-pool/worktrees/acquire")),
            "schedule must not acquire a worktree: {:?}",
            requests
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_execution_result_ready_for_merge_keeps_gateway_slot() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some(project_path.to_string());
        app.task_board_executor_running = true;
        let mut task = pending_task("T-merge-ready");
        task.status = TaskStatus::Running;
        task.selected_worktree_slot_id = Some("slot-merge-ready".to_string());
        task.selected_worktree_path = Some(format!("{project_path}/slot-merge-ready"));
        app.task_board_tasks = vec![task];
        app.task_board_executor.start_task("T-merge-ready");

        let output = format!(
            "done\n__VW_GIT_SOURCE_BRANCH__: vw/task/T-merge-ready\n__VW_GIT_TARGET_BRANCH__: main\n__VW_GIT_WORKTREE_PATH__: {project_path}/slot-merge-ready"
        );

        let _task = super::super::update(
            &mut app,
            TaskBoardMessage::TaskExecutionCompleted {
                project_path: project_path.to_string(),
                task_id: "T-merge-ready".to_string(),
                result: Ok(output),
            },
        );

        let task = app.task_board_tasks.iter().find(|task| task.id == "T-merge-ready").unwrap();
        let expected_path = format!("{project_path}/slot-merge-ready");
        assert_eq!(task.status, TaskStatus::CodeComplete);
        assert_eq!(task.selected_worktree_path.as_deref(), Some(expected_path.as_str()));
        assert!(
            task.logs.iter().any(|entry| entry.message == "等待代码完成调度"),
            "logs: {:?}",
            task.logs
        );

        let gateway = TestGateway::start(vec![
            project_response(project_path),
            runtime_status_response_with_workspace(
                project_path,
                "T-merge-ready",
                "pr_submitted",
                "slot-merge-ready",
                "vw/task/T-merge-ready",
            ),
        ]);
        let status_task = super::super::super::helpers::sync_agent_scheduler_status(
            project_path.to_string(),
        );
        let _ = apply_task_board_task_once(&mut app, status_task).await;

        let task = app.task_board_tasks.iter().find(|task| task.id == "T-merge-ready").unwrap();
        assert_eq!(task.status, TaskStatus::PrSubmitted);
        assert_eq!(task.selected_worktree_path.as_deref(), Some(expected_path.as_str()));

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert!(scheduler_requests.iter().any(|request| {
            request.path == "/v1/agent-scheduler/projects/project-gateway/tasks"
        }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_execution_worktree_gateway_execution_failure_waits_for_runtime_cleanup() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let (mut app, _task) = crate::app::App::new();
        let gateway =
            TestGateway::start(vec![project_response(project_path), json!({"recycled": true})]);
        app.project_path = Some(project_path.to_string());
        let mut task = pending_task("T-recycle");
        task.status = TaskStatus::Running;
        task.selected_worktree_slot_id = Some("slot-recycle".to_string());
        task.selected_worktree_path = Some(format!("{project_path}/slot-recycle"));
        app.task_board_tasks = vec![task];
        app.task_board_executor.start_task("T-recycle");

        let update_task = super::super::update(
            &mut app,
            TaskBoardMessage::TaskExecutionCompleted {
                project_path: project_path.to_string(),
                task_id: "T-recycle".to_string(),
                result: Err("boom".to_string()),
            },
        );
        apply_messages(&mut app, collect_task_messages(update_task).await);

        let requests = gateway.take_requests();
        let requests = task_pool_requests(&requests);
        assert!(
            requests
                .iter()
                .all(|request| { !request.path.starts_with("/v1/task-pool/worktrees/recycle") }),
            "execution failure must not recycle a worktree: {:?}",
            requests
        );
        let task = app.task_board_tasks.iter().find(|task| task.id == "T-recycle").unwrap();
        assert!(
            task.logs
                .iter()
                .any(|entry| entry.message.contains("等待网关运行时清理 worktree 槽位")),
            "logs: {:?}",
            task.logs
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_board_execution_worktree_gateway_merge_scheduler_sends_agent_scheduler_command() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let (mut app, _task) = crate::app::App::new();
        let gateway = TestGateway::start(vec![
            project_response(project_path),
            agent_scheduler_create_response("T-merge"),
            json!({"accepted": true}),
            runtime_status_response(project_path, "T-merge", "pr_submitted"),
        ]);
        app.project_path = Some(project_path.to_string());
        app.task_board_manual_merge_queue = vec!["T-merge".to_string()];
        app.task_board_tasks = vec![merge_task("T-merge")];

        let task = super::super::super::helpers::start_manual_merge_queue_task(&mut app);
        apply_messages(&mut app, collect_task_messages(task).await);

        assert!(app.task_board_manual_merge_queue.is_empty());
        let merge_task = app.task_board_tasks.iter().find(|task| task.id == "T-merge").unwrap();
        assert!(
            merge_task.logs.iter().any(|entry| entry.message.contains("Agent Scheduler 合并命令")),
            "logs: {:?}",
            merge_task.logs
        );

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert_eq!(scheduler_requests.len(), 3);
        assert_eq!(scheduler_requests[0].path, "/v1/agent-scheduler/projects/project-gateway/tasks");
        assert_eq!(scheduler_requests[0].body["task_id"], "T-merge");
        assert_eq!(
            scheduler_requests[1].path,
            "/v1/agent-scheduler/projects/project-gateway/commands"
        );
        assert_eq!(scheduler_requests[1].body["task_id"], "T-merge");
        assert_eq!(scheduler_requests[1].body["command"], "merge");
        assert_eq!(
            scheduler_requests[2].path,
            "/v1/agent-scheduler/projects/project-gateway/tasks"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn merge_scheduler_syncs_status_after_command_error() {
        let _guard =
            super::super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let project_dir = tempfile::tempdir().expect("project dir");
        let project_path = project_dir.path().to_str().expect("utf8 project path");
        let (mut app, _task) = crate::app::App::new();
        let gateway = TestGateway::start_http(vec![
            TestGatewayResponse::ok(project_response(project_path)),
            TestGatewayResponse::ok(agent_scheduler_create_response("T-merge-command-error")),
            TestGatewayResponse::with_status(
                400,
                json!({"error": "merge_source_branch is required"}),
            ),
            TestGatewayResponse::ok(runtime_status_response(
                project_path,
                "T-merge-command-error",
                "failed",
            )),
        ]);
        app.project_path = Some(project_path.to_string());
        app.task_board_manual_merge_queue = vec!["T-merge-command-error".to_string()];
        app.task_board_tasks = vec![merge_task("T-merge-command-error")];

        let task = super::super::super::helpers::start_manual_merge_queue_task(&mut app);
        apply_messages(&mut app, collect_task_messages(task).await);

        assert!(app.task_board_manual_merge_queue.is_empty());
        let merge_task =
            app.task_board_tasks.iter().find(|task| task.id == "T-merge-command-error").unwrap();
        assert_eq!(merge_task.status, TaskStatus::Failed);
        assert_eq!(merge_task.last_error.as_deref(), Some("runtime retry note"));

        let requests = gateway.take_requests();
        let scheduler_requests = task_board_scheduler_requests(&requests);
        assert_eq!(scheduler_requests.len(), 3);
        assert_eq!(scheduler_requests[0].path, "/v1/agent-scheduler/projects/project-gateway/tasks");
        assert_eq!(
            scheduler_requests[1].path,
            "/v1/agent-scheduler/projects/project-gateway/commands"
        );
        assert_eq!(scheduler_requests[1].body["command"], "merge");
        assert_eq!(
            scheduler_requests[2].path,
            "/v1/agent-scheduler/projects/project-gateway/tasks"
        );
    }
}
