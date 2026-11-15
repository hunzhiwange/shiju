use desktop_task_model::{Task, TaskStatus};
fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(1);
    task.id = id.to_string();
    task.status = status;
    task
}

#[test]
fn normalized_bulk_model_input_trims_values_and_defaults_blank_input() {
    assert_eq!(super::normalized_bulk_model_input("  provider/model  "), "provider/model");
    assert_eq!(super::normalized_bulk_model_input(" \n\t "), desktop_task_model::TASK_MODEL_AUTO);
}

#[test]
fn agent_scheduler_command_request_sets_local_defaults_and_payload() {
    use vw_api_agent_scheduler::{
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

#[test]
fn selected_tasks_for_status_returns_cloned_visible_selected_tasks_in_board_order() {
        let mut selected_ids = std::collections::HashSet::new();
    let first = task("T-first", TaskStatus::Pending);
    let second = task("T-second", TaskStatus::Pending);
    let wrong_status = task("T-running", TaskStatus::Running);
    let tasks = vec![first, wrong_status, second];
    selected_ids.extend([
        "T-first".to_string(),
        "T-running".to_string(),
        "T-second".to_string(),
    ]);

    let mut selected = super::selected_tasks_for_status(&tasks, &selected_ids, TaskStatus::Pending);

    assert_eq!(
        selected.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["T-first", "T-second"]
    );
    selected[0].prompt = "changed clone".to_string();
    assert_ne!(tasks[0].prompt, selected[0].prompt);
}

#[test]
fn clear_selection_for_status_keeps_other_and_non_visible_selections() {
        let mut selected_ids = std::collections::HashSet::new();
    let pending = task("T-pending", TaskStatus::Pending);
    let running = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let tasks = vec![pending, running, deleted];
    selected_ids.extend([
        "T-pending".to_string(),
        "T-running".to_string(),
        "T-deleted".to_string(),
    ]);

    super::clear_selection_for_status(&tasks, &mut selected_ids, TaskStatus::Pending);

    assert!(!selected_ids.contains("T-pending"));
    assert!(selected_ids.contains("T-running"));
    assert!(selected_ids.contains("T-deleted"));
}

#[test]
fn clear_selected_task_ids_only_removes_requested_ids() {
    let mut selected_ids = std::collections::HashSet::new();
    selected_ids.extend([
        "T-first".to_string(),
        "T-second".to_string(),
        "T-keep".to_string(),
    ]);

    super::clear_selected_task_ids(
        &mut selected_ids,
        &["T-first".to_string(), "T-absent".to_string(), "T-second".to_string()],
    );

    assert_eq!(selected_ids.len(), 1);
    assert!(selected_ids.contains("T-keep"));
}

#[test]
fn prune_bulk_selection_removes_missing_deleted_and_archived_ids() {
        let mut selected_ids = std::collections::HashSet::new();
    let pending = task("T-pending", TaskStatus::Pending);
    let running = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::Pending);
    archived.archived = true;
    let tasks = vec![pending, running, deleted, archived];
    selected_ids.extend([
        "T-pending".to_string(),
        "T-running".to_string(),
        "T-deleted".to_string(),
        "T-archived".to_string(),
        "T-missing".to_string(),
    ]);

    super::prune_bulk_selection(&tasks, &mut selected_ids);

    assert_eq!(selected_ids.len(), 2);
    assert!(selected_ids.contains("T-pending"));
    assert!(selected_ids.contains("T-running"));
}

#[test]
fn selected_task_ids_for_status_intersect_visible_tasks_and_selection() {
        let mut selected_ids = std::collections::HashSet::new();
    let selected = task("T-selected", TaskStatus::Pending);
    let unselected = task("T-unselected", TaskStatus::Pending);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let tasks = vec![selected, unselected, deleted];
    selected_ids.extend([
        "T-selected".to_string(),
        "T-deleted".to_string(),
        "T-missing".to_string(),
    ]);

    assert_eq!(
        super::selected_task_ids_for_status(&tasks, &selected_ids, TaskStatus::Pending),
        vec!["T-selected".to_string()]
    );
}

#[test]
fn visible_task_ids_filter_status_deleted_and_archived_while_preserving_order() {
        let first = task("T-first", TaskStatus::Pending);
    let wrong_status = task("T-running", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::Pending);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::Pending);
    archived.archived = true;
    let last = task("T-last", TaskStatus::Pending);
    let tasks = vec![first, wrong_status, deleted, archived, last];

    assert_eq!(
        super::visible_task_ids_for_status(&tasks, TaskStatus::Pending),
        vec!["T-first".to_string(), "T-last".to_string()]
    );
}
