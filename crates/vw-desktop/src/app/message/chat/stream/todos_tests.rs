//! 验证待办加载对会话隔离、面板展开和自动收起状态的影响。
//!
//! 待办结果按活动会话隔离，面板只在存在未完成项时展开并在全部完成后自动收起。

#![allow(unused_must_use)]
use crate::app::App;

fn todo(id: &str, status: &str) -> vw_shared::todo::Todo {
    vw_shared::todo::Todo {
        id: id.to_string(),
        content: format!("todo {id}"),
        status: status.to_string(),
        priority: "medium".to_string(),
    }
}

#[test]
fn loaded_todos_for_inactive_session_are_ignored() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("old", "pending")];

    super::handle_input_panel_todos_loaded(
        &mut app,
        "other".to_string(),
        Ok(vec![todo("new", "pending")]),
    );

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert_eq!(app.chat_todo_items[0].id, "old");
}

#[test]
fn loaded_pending_todos_expand_panel_for_new_session() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::handle_input_panel_todos_loaded(
        &mut app,
        "active".to_string(),
        Ok(vec![todo("1", "pending")]),
    );

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);
    assert_eq!(app.chat_todo_items.len(), 1);
}

#[test]
fn loaded_completed_todos_auto_collapse_open_panel() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::handle_input_panel_todos_loaded(
        &mut app,
        "active".to_string(),
        Ok(vec![todo("1", "completed"), todo("2", "completed")]),
    );

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_items.len(), 2);
}

#[test]
fn load_todos_without_active_session_clears_cached_items() {
    let (mut app, _task) = App::new();
    app.chat_todo_session_id = Some("old".to_string());
    app.chat_todo_items = vec![todo("old", "pending")];

    super::handle_load_input_panel_todos(&mut app);

    assert_eq!(app.chat_todo_session_id, None);
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn loaded_todos_error_records_session_and_clears_items() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("old", "pending")];

    super::handle_input_panel_todos_loaded(
        &mut app,
        "active".to_string(),
        Err("gateway down".to_string()),
    );

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn already_collapsed_panel_preserves_cached_todos_at_animation_threshold() {
    let (mut app, _task) = App::new();
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.001;
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
    assert_eq!(app.chat_todo_anim, 0.001);
}

#[test]
fn auto_collapse_without_active_session_clears_cached_todos() {
    let (mut app, _task) = App::new();
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(app.chat_todo_session_id.is_none());
    assert!(app.chat_todo_items.is_empty());
    assert!(app.chat_todo_expanded);
}

#[test]
fn auto_collapse_ignores_todos_cached_for_different_session() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
}

#[test]
fn auto_collapse_keeps_panel_open_for_empty_todo_list() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(app.chat_todo_expanded);
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn auto_collapse_keeps_panel_open_while_any_todo_is_incomplete() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed"), todo("open", "in_progress")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);
}

#[test]
fn completed_todos_close_panel_and_normalize_idle_animation() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = -0.0005;

    super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
}

#[test]
fn completed_todos_with_active_animation_schedule_next_tick() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    let task = super::maybe_auto_collapse_todo_panel(&mut app);

    assert!(!app.chat_todo_expanded);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn load_input_panel_todos_task_schedules_gateway_request() {
    let task =
        super::load_input_panel_todos_task("active".to_string(), Some("/tmp/project".to_string()));

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn todo_list_directory_falls_back_to_project_path() {
    let (mut app, _task) = App::new();
    assert_eq!(super::todo_list_directory(&app), None);

    app.project_path = Some("/project".to_string());
    assert_eq!(super::todo_list_directory(&app).as_deref(), Some("/project"));
}

#[test]
fn loaded_pending_todos_for_same_session_do_not_reopen_panel() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::handle_input_panel_todos_loaded(
        &mut app,
        "active".to_string(),
        Ok(vec![todo("open", "pending")]),
    );

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
    assert_eq!(app.chat_todo_items[0].id, "open");
}

#[test]
fn loaded_empty_todos_for_new_session_do_not_expand_panel() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::handle_input_panel_todos_loaded(&mut app, "active".to_string(), Ok(Vec::new()));

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert!(app.chat_todo_items.is_empty());
    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
}

#[test]
fn load_todos_with_active_session_defers_cached_state_changes() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "pending")];

    super::handle_load_input_panel_todos(&mut app);

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
}

#[test]
fn poll_without_active_session_clears_cached_items() {
    let (mut app, _task) = App::new();
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "pending")];

    super::handle_todo_poll_tick(&mut app);

    assert!(app.chat_todo_session_id.is_none());
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn poll_with_active_session_defers_cached_state_changes() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "pending")];

    super::handle_todo_poll_tick(&mut app);

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
}
