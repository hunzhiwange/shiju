#[derive(Default)]
struct DragFixture {
    task_board_drag_pending: Option<(String, desktop_task_model::TaskStatus, iced::Point)>,
    task_board_dragging: Option<(String, desktop_task_model::TaskStatus)>,
}
fn test_app() -> DragFixture {
    DragFixture::default()
}
#[test]
fn task_board_drag_starts_only_after_threshold_is_exceeded() {
    let mut app = test_app();
    app.task_board_drag_pending =
        Some(("task-1".to_owned(), desktop_task_model::TaskStatus::Planning, iced::Point::ORIGIN));

    let _task = crate::drag::promote_pending_drag(
        &mut app.task_board_drag_pending,
        &mut app.task_board_dragging,
        iced::Point::new(3.0, 4.0),
    );
    assert!(app.task_board_drag_pending.is_some());
    assert!(app.task_board_dragging.is_none());

    let _task = crate::drag::promote_pending_drag(
        &mut app.task_board_drag_pending,
        &mut app.task_board_dragging,
        iced::Point::new(6.0, 0.0),
    );
    assert!(app.task_board_drag_pending.is_none());
    assert_eq!(
        app.task_board_dragging,
        Some(("task-1".to_owned(), desktop_task_model::TaskStatus::Planning))
    );
}

// Unlike the existing populated-card fixture, this preserves the empty-inventory case.
#[test]
fn task_pet_reply_pressed_again_closes_editor_and_clears_draft() {
    use crate::{pet_interaction::TaskPetState, pet_state::TaskPetAvatarKind};
    let mut state = TaskPetState {
        task_pet_avatar_kind: &mut TaskPetAvatarKind::Robot,
        task_pet_collapsed: &mut true,
        task_pet_dismissed_request_ids: &mut Default::default(),
        task_pet_drag_anchor: &mut None,
        task_pet_drag_direction: &mut 1,
        task_pet_drag_start: &mut iced::Point::ORIGIN,
        task_pet_dragging: &mut false,
        task_pet_expand_progress: &mut 0.0,
        task_pet_expand_target: &mut None,
        task_pet_hovered_request_id: &mut None,
        task_pet_items: &mut Vec::new(),
        task_pet_position: &mut iced::Point::ORIGIN,
        task_pet_reply_input: &mut String::new(),
        task_pet_reply_request_id: &mut None,
        task_pet_robot_hovered: &mut false,
        task_pet_scale: &mut 1.0,
        task_pet_size_draft: &mut None,
        task_pet_walk_until_ms: &mut 0,
        window_size: (1200.0, 800.0),
        now_ms: 0,
    };
    state.open_task_pet_reply(9);
    state.update_task_pet_reply_input("draft".to_string());
    state.open_task_pet_reply(9);
    assert_eq!(*state.task_pet_reply_request_id, None);
    assert!(state.task_pet_reply_input.is_empty());
}
