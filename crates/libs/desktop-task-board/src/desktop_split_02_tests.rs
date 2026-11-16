
mod behavior_0 {
use crate::pet_state::*;
use crate::pet_interaction::TaskPetState;

#[test]
fn task_pet_window_animation_drag_and_avatar_state_transitions() {
    let mut app = TaskPetState {
task_pet_avatar_kind: &mut TaskPetAvatarKind::Robot,
task_pet_collapsed: &mut false,
task_pet_dismissed_request_ids: &mut Default::default(),
task_pet_drag_anchor: &mut None,
task_pet_drag_direction: &mut 1,
task_pet_drag_start: &mut iced::Point::ORIGIN,
task_pet_dragging: &mut false,
task_pet_expand_progress: &mut 1.0,
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
window_size: (500.0, 420.0), now_ms: 1000,
};
    app.window_size = (500.0, 420.0);
    (*app.task_pet_position) = iced::Point::new(300.0, 220.0);
    (*app.task_pet_expand_progress) = 1.0;
    (*app.task_pet_collapsed) = false;

    app.toggle_task_pet_collapsed();
    assert!((*app.task_pet_collapsed));
    assert!(!app.task_pet_render_collapsed());

    let (size, position) = app.advance_task_pet_expand_animation().unwrap();
    assert!(size.width < 360.0);
    assert!(position.x > 300.0);
    assert!((*app.task_pet_expand_target).is_some());

    (*app.task_pet_expand_progress) = 0.1;
    (*app.task_pet_expand_target) = Some(0.0);
    let _ = app.advance_task_pet_expand_animation().unwrap();
    assert_eq!((*app.task_pet_expand_progress), 0.0);
    assert!((*app.task_pet_expand_target).is_none());
    assert!(app.task_pet_render_collapsed());

    assert!(app.advance_task_pet_expand_animation().is_none());

    app.start_task_pet_drag();
    assert!((*app.task_pet_dragging));
    app.drag_task_pet_to(100.0, 100.0);
    assert_eq!((*app.task_pet_drag_anchor), Some(iced::Point::new(100.0, 100.0)));
    let before = (*app.task_pet_position);
    app.drag_task_pet_to(-200.0, 600.0);
    assert!((*app.task_pet_position).x <= before.x);
    assert!(app.task_pet_facing_left());
    assert!(app.task_pet_is_walking());
    app.finish_task_pet_drag();
    assert!(!(*app.task_pet_dragging));
    assert!((*app.task_pet_drag_anchor).is_none());

    app.set_task_pet_robot_hovered(true);
    assert!((*app.task_pet_robot_hovered));
    app.cycle_task_pet_avatar();
    assert_eq!((*app.task_pet_avatar_kind), TaskPetAvatarKind::Beauty);
}
}

mod behavior_1 {
use crate::pet_state::*;
use crate::pet_interaction::TaskPetState;

#[test]
fn task_pet_reply_dismiss_refresh_and_completed_click_behaviors() {
    let mut app = TaskPetState {
task_pet_avatar_kind: &mut TaskPetAvatarKind::Robot,
task_pet_collapsed: &mut false,
task_pet_dismissed_request_ids: &mut Default::default(),
task_pet_drag_anchor: &mut None,
task_pet_drag_direction: &mut 1,
task_pet_drag_start: &mut iced::Point::ORIGIN,
task_pet_dragging: &mut false,
task_pet_expand_progress: &mut 1.0,
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
window_size: (500.0, 420.0), now_ms: 1000,
};
    (*app.task_pet_items) = vec![
        TaskPetItem {
            request_id: 1,
            session_id: "s1".to_string(),
            title: "Old".to_string(),
            detail: String::new(),
            detail_feedback: None,
            project: None,
            status: TaskPetStatus::Running,
            last_click_at: None,
        },
        TaskPetItem {
            request_id: 2,
            session_id: "s2".to_string(),
            title: "Done".to_string(),
            detail: "finished".to_string(),
            detail_feedback: None,
            project: None,
            status: TaskPetStatus::Completed,
            last_click_at: None,
        },
    ];

    app.refresh_task_pet_session_title("s1", "\n Refreshed title");
    assert_eq!((*app.task_pet_items)[0].title, "Refreshed title");

    app.set_task_pet_hovered(Some(1));
    app.open_task_pet_reply(1);
    assert_eq!((*app.task_pet_reply_request_id), Some(1));
    assert_eq!((*app.task_pet_hovered_request_id), Some(1));
    app.update_task_pet_reply_input(" \n ".to_string());
    assert_eq!(app.take_task_pet_reply(), None);
    app.update_task_pet_reply_input(" continue please ".to_string());
    assert_eq!(app.take_task_pet_reply(), Some(("s1".to_string(), "continue please".to_string())));
    assert!((*app.task_pet_reply_input).is_empty());

    app.open_task_pet_reply(1);
    app.update_task_pet_reply_input("draft".to_string());
    app.open_task_pet_reply(1);
    assert!((*app.task_pet_reply_request_id).is_none());
    assert!((*app.task_pet_reply_input).is_empty());

    assert_eq!(app.task_pet_item_clicked(99), None);
    assert_eq!(app.task_pet_item_clicked(1), None);
    assert_eq!(app.task_pet_item_clicked(1), Some("s1".to_string()));
    assert_eq!(app.task_pet_item_clicked(2), None);
    assert_eq!(app.task_pet_item_clicked(2), Some("s2".to_string()));
    assert!((*app.task_pet_items).iter().any(|item| item.request_id == 2));
    app.dismiss_task_pet_item(2);

    app.open_task_pet_reply(1);
    app.update_task_pet_reply_input("draft".to_string());
    app.dismiss_task_pet_item(1);
    assert!((*app.task_pet_dismissed_request_ids).contains(&1));
    assert!((*app.task_pet_items).is_empty());
    assert!((*app.task_pet_hovered_request_id).is_none());
    assert!((*app.task_pet_reply_request_id).is_none());
    assert!((*app.task_pet_reply_input).is_empty());
}
}

mod behavior_2 {
use crate::pet_state::*;
use crate::pet_interaction::TaskPetState;
fn mini_task(id: u64, status: TaskPetStatus) -> TaskPetItem {
    TaskPetItem {
        request_id: id,
        session_id: format!("session-{id}"),
        title: format!("任务 {id}"),
        detail: "正在整理上下文".into(),
        detail_feedback: None,
        project: None,
        status,
        last_click_at: None,
    }
}
#[test]
fn mini_companion_reply_stays_in_target_session() {
    let mut app = TaskPetState {
task_pet_avatar_kind: &mut TaskPetAvatarKind::Robot,
task_pet_collapsed: &mut false,
task_pet_dismissed_request_ids: &mut Default::default(),
task_pet_drag_anchor: &mut None,
task_pet_drag_direction: &mut 1,
task_pet_drag_start: &mut iced::Point::ORIGIN,
task_pet_dragging: &mut false,
task_pet_expand_progress: &mut 1.0,
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
window_size: (500.0, 420.0), now_ms: 1000,
};
    (*app.task_pet_items) =
        vec![mini_task(3, TaskPetStatus::Running), mini_task(2, TaskPetStatus::Completed)];
    app.open_task_pet_reply(2);
    app.update_task_pet_reply_input("仅发给第二项".into());
    (*app.task_pet_items).swap(0, 1);
    assert_eq!(app.take_task_pet_reply(), Some(("session-2".into(), "仅发给第二项".into())));
    app.open_task_pet_reply(2);
    app.update_task_pet_reply_input("不应串到另一会话".into());
    app.open_task_pet_reply(3);
    assert!((*app.task_pet_reply_input).is_empty());
}
}

mod behavior_3 {
use crate::pet_state::*;
use crate::pet_interaction::TaskPetState;
fn mini_task(id: u64, status: TaskPetStatus) -> TaskPetItem {
    TaskPetItem {
        request_id: id,
        session_id: format!("session-{id}"),
        title: format!("任务 {id}"),
        detail: "正在整理上下文".into(),
        detail_feedback: None,
        project: None,
        status,
        last_click_at: None,
    }
}
#[test]
fn mini_companion_double_click_opens_without_dismissing_completed_card() {
    let mut app = TaskPetState {
task_pet_avatar_kind: &mut TaskPetAvatarKind::Robot,
task_pet_collapsed: &mut false,
task_pet_dismissed_request_ids: &mut Default::default(),
task_pet_drag_anchor: &mut None,
task_pet_drag_direction: &mut 1,
task_pet_drag_start: &mut iced::Point::ORIGIN,
task_pet_dragging: &mut false,
task_pet_expand_progress: &mut 1.0,
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
window_size: (500.0, 420.0), now_ms: 1000,
};
    (*app.task_pet_items) = vec![mini_task(2, TaskPetStatus::Completed)];
    assert_eq!(app.task_pet_item_clicked(2), None);
    assert_eq!(app.task_pet_item_clicked(2), Some("session-2".into()));
    assert_eq!((*app.task_pet_items).len(), 1);
}
}
