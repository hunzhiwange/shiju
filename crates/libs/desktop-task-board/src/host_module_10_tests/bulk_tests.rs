use desktop_task_model::{Task, TaskSplitMode, TaskStatus};
fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(1);
    task.id = id.to_string();
    task.status = status;
    task
}
#[test]
fn reset_bulk_operation_inputs_copies_draft_and_recent_choices() {
    let mut app = super::Fixture::default();
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
    let mut app = super::Fixture::default();
    app.task_board_draft.priority = "0".to_string();
    app.task_board_draft.agent = None;
    app.task_board_last_model = "   ".to_string();

    super::reset_bulk_operation_inputs(&mut app);

    assert_eq!(app.task_board_bulk_priority_input, "999");
    assert_eq!(app.task_board_bulk_model_input, desktop_task_model::TASK_MODEL_AUTO);
    assert_eq!(app.task_board_bulk_agent, desktop_task_model::TASK_AGENT_MAIN);
}

#[test]
fn deactivate_bulk_selection_mode_clears_only_active_status_selection() {
    let mut app = super::Fixture::default();
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
    let mut app = super::Fixture::default();
    app.task_board_selected_tasks.insert("T-selected".to_string());

    super::deactivate_bulk_selection_mode(&mut app);

    assert!(app.task_board_selected_tasks.contains("T-selected"));
}

#[test]
fn manual_merge_running_requires_running_pr_submitted_board_task() {
    let mut app = super::Fixture::default();
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
    let mut app = super::Fixture::default();
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
    let mut app = super::Fixture::default();
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];

    assert!(super::pop_next_manual_merge_task(&mut app).is_none());
    assert!(app.task_board_manual_merge_queue.is_empty());
}

#[test]
fn start_manual_merge_queue_task_returns_none_when_pr_merge_is_already_running() {
    let mut app = super::Fixture::default();
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
    let mut app = super::Fixture::default();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert!(app.task_board_manual_merge_queue.is_empty());
}

#[test]
fn start_manual_merge_queue_task_with_eligible_task_adds_log_and_pops_queue() {
    let mut app = super::Fixture::default();
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
