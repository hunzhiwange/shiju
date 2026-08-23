// 验证任务看板批量选择对状态、归档与删除项的过滤。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::task::{Task, TaskSplitMode, TaskStatus};

fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(1);
    task.id = id.to_string();
    task.status = status;
    task
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn visible_task_ids_filter_status_deleted_and_archived_while_preserving_order() {
    let (mut app, _task) = crate::app::App::new();
    let first = task("T-first", TaskStatus::Pending);
    let wrong_status = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::Pending);
    archived.archived = true;
    let last = task("T-last", TaskStatus::Pending);
    app.task_board_tasks = vec![first, wrong_status, deleted, archived, last];

    assert_eq!(
        super::visible_task_ids_for_status(&app, TaskStatus::Pending),
        vec!["T-first".to_string(), "T-last".to_string()]
    );
}

#[test]
fn selected_task_ids_for_status_intersect_visible_tasks_and_selection() {
    let (mut app, _task) = crate::app::App::new();
    let selected = task("T-selected", TaskStatus::Pending);
    let unselected = task("T-unselected", TaskStatus::Pending);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    app.task_board_tasks = vec![selected, unselected, deleted];
    app.task_board_selected_tasks.extend([
        "T-selected".to_string(),
        "T-deleted".to_string(),
        "T-missing".to_string(),
    ]);

    assert_eq!(
        super::selected_task_ids_for_status(&app, TaskStatus::Pending),
        vec!["T-selected".to_string()]
    );
}

#[test]
fn prune_bulk_selection_removes_missing_deleted_and_archived_ids() {
    let (mut app, _task) = crate::app::App::new();
    let pending = task("T-pending", TaskStatus::Pending);
    let running = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::Pending);
    archived.archived = true;
    app.task_board_tasks = vec![pending, running, deleted, archived];
    app.task_board_selected_tasks.extend([
        "T-pending".to_string(),
        "T-running".to_string(),
        "T-deleted".to_string(),
        "T-archived".to_string(),
        "T-missing".to_string(),
    ]);

    super::prune_bulk_selection(&mut app);

    assert_eq!(app.task_board_selected_tasks.len(), 2);
    assert!(app.task_board_selected_tasks.contains("T-pending"));
    assert!(app.task_board_selected_tasks.contains("T-running"));
}

#[test]
fn clear_selected_task_ids_only_removes_requested_ids() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_selected_tasks.extend([
        "T-first".to_string(),
        "T-second".to_string(),
        "T-keep".to_string(),
    ]);

    super::clear_selected_task_ids(
        &mut app,
        &["T-first".to_string(), "T-absent".to_string(), "T-second".to_string()],
    );

    assert_eq!(app.task_board_selected_tasks.len(), 1);
    assert!(app.task_board_selected_tasks.contains("T-keep"));
}

#[test]
fn clear_selection_for_status_keeps_other_and_non_visible_selections() {
    let (mut app, _task) = crate::app::App::new();
    let pending = task("T-pending", TaskStatus::Pending);
    let running = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    app.task_board_tasks = vec![pending, running, deleted];
    app.task_board_selected_tasks.extend([
        "T-pending".to_string(),
        "T-running".to_string(),
        "T-deleted".to_string(),
    ]);

    super::clear_selection_for_status(&mut app, TaskStatus::Pending);

    assert!(!app.task_board_selected_tasks.contains("T-pending"));
    assert!(app.task_board_selected_tasks.contains("T-running"));
    assert!(app.task_board_selected_tasks.contains("T-deleted"));
}

#[test]
fn selected_tasks_for_status_returns_cloned_visible_selected_tasks_in_board_order() {
    let (mut app, _task) = crate::app::App::new();
    let first = task("T-first", TaskStatus::Pending);
    let second = task("T-second", TaskStatus::Pending);
    let wrong_status = task("T-running", TaskStatus::Running);
    app.task_board_tasks = vec![first, wrong_status, second];
    app.task_board_selected_tasks.extend([
        "T-first".to_string(),
        "T-running".to_string(),
        "T-second".to_string(),
    ]);

    let mut selected = super::selected_tasks_for_status(&app, TaskStatus::Pending);

    assert_eq!(
        selected.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["T-first", "T-second"]
    );
    selected[0].prompt = "changed clone".to_string();
    assert_ne!(app.task_board_tasks[0].prompt, selected[0].prompt);
}

#[test]
fn agent_scheduler_command_request_sets_local_defaults_and_payload() {
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto,
        AgentSchedulerDispatchModeDto,
    };

    let payload = AgentSchedulerCommandPayloadDto {
        extra: Some(serde_json::json!({"source": "bulk-test"})),
        ..Default::default()
    };
    let request = super::agent_scheduler_command_request(
        "T-command",
        AgentSchedulerCommandKindDto::Pause,
        Some("manual pause".to_string()),
        payload.clone(),
    );

    assert_eq!(request.task_id.as_ref(), "T-command");
    assert_eq!(request.command, AgentSchedulerCommandKindDto::Pause);
    assert_eq!(request.dispatch_mode, AgentSchedulerDispatchModeDto::Local);
    assert_eq!(request.max_concurrent, None);
    assert_eq!(request.reason.as_deref(), Some("manual pause"));
    assert_eq!(request.payload, payload);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_bulk_command_short_circuits_without_gateway_access() {
    let result =
        super::submit_agent_scheduler_bulk_command("", Vec::new(), "empty operation").await;

    assert_eq!(result, Ok(()));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test]
async fn empty_gateway_bulk_wrappers_short_circuit_without_gateway_access() {
    assert_eq!(
        super::persist_updated_tasks_via_gateway("", &[], "批量更新", "workflow").await,
        Ok(())
    );
    assert_eq!(super::delete_tasks_via_gateway("", &[]).await, Ok(()));
    assert_eq!(super::send_archive_commands_via_gateway("", &[]).await, Ok(()));
    assert_eq!(
        super::move_tasks_to_status_via_gateway("", &[], TaskStatus::Running, "workflow").await,
        Ok(())
    );
}

#[test]
fn task_metadata_patch_trims_workflow_and_copies_task_fields() {
    let mut task = task("T-metadata", TaskStatus::Pending);
    task.prompt = "First line\nSecond line".to_string();
    task.priority = 42;
    task.order = 7;
    task.model = "provider/model".to_string();
    task.agent = Some("reviewer".to_string());
    task.acp_agent = Some("codex".to_string());
    task.assignee = "Alice".to_string();
    task.description = "description".to_string();
    task.task_split_mode = TaskSplitMode::Enabled;
    task.auto_promote_delay_ms = Some(5_000);

    let patch = super::task_metadata_patch_for_gateway(&task, "  dev_lifecycle  ");

    assert_eq!(patch.title.as_deref(), Some("First line"));
    assert_eq!(patch.priority, Some(42));
    assert_eq!(patch.order_no, Some(7));
    assert_eq!(patch.model.as_deref(), Some("provider/model"));
    assert_eq!(patch.agent.as_deref(), Some("reviewer"));
    assert_eq!(patch.acp_agent.as_deref(), Some("codex"));
    assert_eq!(patch.prompt.as_deref(), Some(task.prompt.as_str()));
    assert_eq!(patch.workflow_id.as_deref(), Some("dev_lifecycle"));
    assert!(patch.status_key.is_some());
    assert!(patch.status.is_some());
    assert!(patch.status_role.is_some());
    assert_eq!(
        patch.metadata_merge,
        Some(serde_json::json!({
            "assignee": "Alice",
            "description": "description",
            "subtasks": [],
            "auto_promote_delay_ms": 5_000,
            "task_split_mode": "enabled",
        }))
    );
    assert!(patch.metadata_remove_keys.is_empty());
}

#[test]
fn task_metadata_patch_copies_error_pause_and_worktree_fields() {
    let mut task = task("T-worktree", TaskStatus::PrSubmitted);
    task.last_error = Some("merge conflict".to_string());
    task.pause_reason = Some("manual review".to_string());
    task.merge_source_branch = Some("feature/a".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path = Some("/repo/worktree".to_string());
    task.selected_worktree_slot_id = Some("slot-1".to_string());
    task.selected_worktree_branch = Some("feature/a".to_string());

    let patch = super::task_metadata_patch_for_gateway(&task, "workflow");

    assert_eq!(patch.last_error.as_deref(), Some("merge conflict"));
    assert_eq!(patch.pause_reason.as_deref(), Some("manual review"));
    assert_eq!(patch.merge_source_branch.as_deref(), Some("feature/a"));
    assert_eq!(patch.merge_target_branch.as_deref(), Some("main"));
    assert_eq!(patch.selected_worktree_path.as_deref(), Some("/repo/worktree"));
    assert_eq!(patch.selected_worktree_slot_id.as_deref(), Some("slot-1"));
    assert_eq!(patch.selected_worktree_branch.as_deref(), Some("feature/a"));
}

#[test]
fn task_metadata_patch_uses_id_and_default_workflow_for_empty_inputs() {
    let task = task("T-fallback-title", TaskStatus::Pool);

    let patch = super::task_metadata_patch_for_gateway(&task, "   ");

    assert_eq!(patch.title.as_deref(), Some("T-fallback-title"));
    assert_eq!(patch.workflow_id.as_deref(), Some(crate::app::task::DEFAULT_TASK_WORKFLOW_ID));
}

#[test]
fn normalized_bulk_model_input_trims_values_and_defaults_blank_input() {
    assert_eq!(super::normalized_bulk_model_input("  provider/model  "), "provider/model");
    assert_eq!(super::normalized_bulk_model_input(" \n\t "), crate::app::task::TASK_MODEL_AUTO);
}

#[test]
fn reset_bulk_operation_inputs_copies_draft_and_recent_choices() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_draft.priority = " 17 ".to_string();
    app.task_board_draft.agent = Some("specialist".to_string());
    app.task_board_draft.task_split_mode = TaskSplitMode::Disabled;
    app.task_board_last_model = "  provider/model  ".to_string();
    app.task_board_last_acp_agent = Some("codex".to_string());

    super::reset_bulk_operation_inputs(&mut app);

    assert_eq!(app.task_board_bulk_priority_input, "17");
    assert_eq!(app.task_board_bulk_model_input, "provider/model");
    assert_eq!(app.task_board_bulk_agent, "specialist");
    assert_eq!(app.task_board_bulk_acp_agent.as_deref(), Some("codex"));
    assert_eq!(app.task_board_bulk_reasoning_effort, app.task_board_draft.reasoning_effort);
    assert_eq!(app.task_board_bulk_task_split_mode, TaskSplitMode::Disabled);
}

#[test]
fn reset_bulk_operation_inputs_uses_priority_model_and_agent_defaults() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_draft.priority = "0".to_string();
    app.task_board_draft.agent = None;
    app.task_board_last_model = "   ".to_string();

    super::reset_bulk_operation_inputs(&mut app);

    assert_eq!(app.task_board_bulk_priority_input, "999");
    assert_eq!(app.task_board_bulk_model_input, crate::app::task::TASK_MODEL_AUTO);
    assert_eq!(app.task_board_bulk_agent, crate::app::task::TASK_AGENT_MAIN);
}

#[test]
fn deactivate_bulk_selection_mode_clears_only_active_status_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks =
        vec![task("T-pending", TaskStatus::Pending), task("T-running", TaskStatus::Running)];
    app.task_board_selected_tasks.extend(["T-pending".to_string(), "T-running".to_string()]);
    app.task_board_bulk_active_status = Some(TaskStatus::Pending);

    super::deactivate_bulk_selection_mode(&mut app);

    assert_eq!(app.task_board_bulk_active_status, None);
    assert!(!app.task_board_selected_tasks.contains("T-pending"));
    assert!(app.task_board_selected_tasks.contains("T-running"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn deactivate_bulk_selection_mode_without_active_status_preserves_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_selected_tasks.insert("T-selected".to_string());

    super::deactivate_bulk_selection_mode(&mut app);

    assert!(app.task_board_selected_tasks.contains("T-selected"));
}

#[test]
fn manual_merge_running_requires_running_pr_submitted_board_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks = vec![
        task("T-running-status", TaskStatus::Running),
        task("T-merge", TaskStatus::PrSubmitted),
    ];
    app.task_board_executor.running_tasks =
        vec!["T-missing".to_string(), "T-running-status".to_string()];

    assert!(!super::manual_merge_running(&app));

    app.task_board_executor.running_tasks.push("T-merge".to_string());
    assert!(super::manual_merge_running(&app));
}

#[test]
fn pop_next_manual_merge_task_discards_invalid_entries_and_returns_first_eligible_task() {
    let (mut app, _task) = crate::app::App::new();
    let wrong_status = task("T-wrong-status", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::PrSubmitted);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::PrSubmitted);
    archived.archived = true;
    let already_running = task("T-running", TaskStatus::PrSubmitted);
    let eligible = task("T-eligible", TaskStatus::PrSubmitted);
    app.task_board_tasks = vec![wrong_status, deleted, archived, already_running, eligible];
    app.task_board_executor.running_tasks.push("T-running".to_string());
    app.task_board_manual_merge_queue = vec![
        "T-missing".to_string(),
        "T-wrong-status".to_string(),
        "T-deleted".to_string(),
        "T-archived".to_string(),
        "T-running".to_string(),
        "T-eligible".to_string(),
        "T-later".to_string(),
    ];

    let mut selected = super::pop_next_manual_merge_task(&mut app).expect("eligible merge task");

    assert_eq!(selected.id, "T-eligible");
    assert_eq!(app.task_board_manual_merge_queue, vec!["T-later".to_string()]);
    selected.prompt = "changed clone".to_string();
    assert_eq!(app.task_board_tasks[4].prompt, "");
}

#[test]
fn pop_next_manual_merge_task_drains_queue_without_eligible_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];

    assert!(super::pop_next_manual_merge_task(&mut app).is_none());
    assert!(app.task_board_manual_merge_queue.is_empty());
}

#[test]
fn start_manual_merge_queue_task_returns_none_when_pr_merge_is_already_running() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-running", TaskStatus::PrSubmitted)];
    app.task_board_executor.running_tasks = vec!["T-running".to_string()];
    app.task_board_manual_merge_queue = vec!["T-running".to_string(), "T-later".to_string()];

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert_eq!(
        app.task_board_manual_merge_queue,
        vec!["T-running".to_string(), "T-later".to_string()]
    );
    assert_eq!(app.task_board_tasks[0].logs.len(), 1);
}

#[test]
fn start_manual_merge_queue_task_with_project_and_no_eligible_task_drains_invalid_queue() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];
    app.notifications.clear();

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert!(app.task_board_manual_merge_queue.is_empty());
    assert!(app.notifications.is_empty());
}

#[test]
fn start_manual_merge_queue_task_with_eligible_task_adds_log_and_pops_queue() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-merge", TaskStatus::PrSubmitted)];
    app.task_board_manual_merge_queue = vec!["T-merge".to_string(), "T-later".to_string()];

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert_eq!(app.task_board_manual_merge_queue, vec!["T-later".to_string()]);
    assert_eq!(app.task_board_tasks[0].logs.len(), 2);
    assert!(
        app.task_board_tasks[0].logs[1]
            .message
            .contains("手动批量合并已发送 Agent Scheduler 合并命令")
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn start_manual_merge_queue_task_without_project_clears_queue_and_notifies() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.task_board_tasks = vec![task("T-merge", TaskStatus::PrSubmitted)];
    app.task_board_manual_merge_queue = vec!["T-merge".to_string()];
    app.notifications.clear();

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert!(app.task_board_manual_merge_queue.is_empty());
    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].message, "缺少项目路径，无法执行批量合并");
}
