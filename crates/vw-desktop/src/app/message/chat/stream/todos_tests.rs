//! 验证待办加载对会话隔离、面板展开和自动收起状态的影响。
//!
//! 待办结果按活动会话隔离，面板只在存在未完成项时展开并在全部完成后自动收起。

#![allow(unused_must_use)]
use crate::app::App;

fn todo(id: &str, status: &str) -> todo_types::Todo {
    todo_types::Todo {
        id: id.to_string(),
        content: format!("todo {id}"),
        status: status.to_string(),
        priority: "medium".to_string(),
    }
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
