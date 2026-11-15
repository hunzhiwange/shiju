#[derive(Default)]
struct Fixture {
 active_session_id: Option<String>, chat_todo_session_id: Option<String>,
 chat_todo_items: Vec<todo_types::Todo>, chat_todo_expanded: bool, chat_todo_anim: f32,
}
impl Fixture { fn panel(&mut self) -> super::TodoPanel<'_> { super::TodoPanel {
active_session_id: self.active_session_id.as_deref(), chat_todo_session_id: &mut self.chat_todo_session_id,
chat_todo_items: &mut self.chat_todo_items, chat_todo_expanded: &mut self.chat_todo_expanded, chat_todo_anim: &mut self.chat_todo_anim,
} } }
fn todo(id: &str, status: &str) -> todo_types::Todo {
    todo_types::Todo {
        id: id.to_string(),
        content: format!("todo {id}"),
        status: status.to_string(),
        priority: "medium".to_string(),
    }
}

#[test]
fn loaded_todos_for_inactive_session_are_ignored() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("old", "pending")];

    super::apply_loaded_session_todos(
        &mut app.panel(),
        "other".to_string(),
        Ok(vec![todo("new", "pending")]),
    );

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert_eq!(app.chat_todo_items[0].id, "old");
}

#[test]
fn loaded_pending_todos_expand_panel_for_new_session() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::apply_loaded_session_todos(
        &mut app.panel(),
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
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::apply_loaded_session_todos(
        &mut app.panel(),
        "active".to_string(),
        Ok(vec![todo("1", "completed"), todo("2", "completed")]),
    );

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_items.len(), 2);
}

#[test]
fn loaded_todos_error_records_session_and_clears_items() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("old", "pending")];

    super::apply_loaded_session_todos(
        &mut app.panel(),
        "active".to_string(),
        Err("gateway down".to_string()),
    );

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn already_collapsed_panel_preserves_cached_todos_at_animation_threshold() {
    let mut app = Fixture::default();
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.001;
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
    assert_eq!(app.chat_todo_anim, 0.001);
}

#[test]
fn auto_collapse_without_active_session_clears_cached_todos() {
    let mut app = Fixture::default();
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(app.chat_todo_session_id.is_none());
    assert!(app.chat_todo_items.is_empty());
    assert!(app.chat_todo_expanded);
}

#[test]
fn auto_collapse_ignores_todos_cached_for_different_session() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("cached".to_string());
    app.chat_todo_items = vec![todo("cached", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_session_id.as_deref(), Some("cached"));
    assert_eq!(app.chat_todo_items[0].id, "cached");
}

#[test]
fn auto_collapse_keeps_panel_open_for_empty_todo_list() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(app.chat_todo_expanded);
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn auto_collapse_keeps_panel_open_while_any_todo_is_incomplete() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed"), todo("open", "in_progress")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);
}

#[test]
fn completed_todos_close_panel_and_normalize_idle_animation() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = -0.0005;

    super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
}

#[test]
fn completed_todos_with_active_animation_schedule_next_tick() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_items = vec![todo("done", "completed")];
    app.chat_todo_expanded = true;
    app.chat_todo_anim = 1.0;

    let task = super::maybe_auto_collapse_todo_panel(&mut app.panel());

    assert!(!app.chat_todo_expanded);
    assert!(task);
}

#[test]
fn loaded_pending_todos_for_same_session_do_not_reopen_panel() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::apply_loaded_session_todos(
        &mut app.panel(),
        "active".to_string(),
        Ok(vec![todo("open", "pending")]),
    );

    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
    assert_eq!(app.chat_todo_items[0].id, "open");
}

#[test]
fn loaded_empty_todos_for_new_session_do_not_expand_panel() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.0;

    super::apply_loaded_session_todos(&mut app.panel(), "active".to_string(), Ok(Vec::new()));

    assert_eq!(app.chat_todo_session_id.as_deref(), Some("active"));
    assert!(app.chat_todo_items.is_empty());
    assert!(!app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 0.0);
}
