use super::*;
use crate::message::TaskBoardMessage;
use crate::update::{
    bulk::{BulkEffect, update_bulk},
    local::{LocalEffect, update_local},
};

#[test]
fn bulk_priority_returns_persistence_intent_and_clears_only_selected_ids() {
    let mut state = Fixture::default();
    let mut task = Task::new(3);
    task.id = "chosen".into();
    task.status = TaskStatus::Pending;
    state.task_board_tasks = vec![task];
    state.task_board_selected_tasks.extend(["chosen".into(), "other".into()]);
    state.task_board_bulk_priority_input = " 17 ".into();
    let Ok((BulkEffect::Persist(tasks, label), logs)) = update_bulk(
        bulk_state(&mut state),
        TaskBoardMessage::BulkSetPriorityInStatus(TaskStatus::Pending),
        &[],
        "main",
    ) else {
        panic!("persistence intent expected");
    };
    assert_eq!(tasks[0].priority, 17);
    assert_eq!(label, "批量设置优先级");
    assert!(logs.is_empty());
    assert_eq!(state.task_board_bulk_priority_input, "17");
    assert!(!state.task_board_selected_tasks.contains("chosen"));
    assert!(state.task_board_selected_tasks.contains("other"));
    assert_eq!(state.task_board_tasks[0].priority, 3);
}

#[test]
fn task_created_ignores_other_projects_and_closes_create_form_for_current_project() {
    let mut state = Fixture::default();
    state.project_path = Some("/current".into());
    state.task_board_close_after_create = true;
    state.task_board_create_modal_open = true;
    state.task_board_draft.prompt = "stale".into();
    let result = update_local(
        local_state(&mut state),
        TaskBoardMessage::TaskCreated { project_path: "/other".into(), task: Task::new(1) },
        "main",
    );
    assert!(matches!(result, Ok(LocalEffect::None)));
    assert!(state.task_board_tasks.is_empty());
    assert!(state.task_board_create_modal_open);
    let result = update_local(
        local_state(&mut state),
        TaskBoardMessage::TaskCreated { project_path: "/current".into(), task: Task::new(1) },
        "main",
    );
    assert!(matches!(result, Ok(LocalEffect::None)));
    assert_eq!(state.task_board_tasks.len(), 1);
    assert!(!state.task_board_create_modal_open);
    assert!(!state.task_board_create_submit_success);
    assert!(state.task_board_draft.prompt.is_empty());
    assert!(state.task_board_prompt_editor.text().is_empty());
}
