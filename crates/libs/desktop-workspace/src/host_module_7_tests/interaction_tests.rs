use crate::project_message::session::interaction::{self, Event, State};
use std::collections::{HashMap, HashSet};
#[derive(Default)]
struct Fixture {
    session_menu_id: Option<String>,
    session_menu_anchor: Option<iced::Point>,
    actions: bool,
    project_menu: Option<String>,
    session_tree_expanded: HashSet<String>,
    session_rename_id: Option<String>,
    session_rename_value: String,
    sessions: Vec<session_info::Info>,
    project_sessions: HashMap<String, Vec<session_info::Info>>,
}
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, event: Event) -> Option<Option<interaction::Rename>> {
    Some(interaction::update(
        &mut State {
            menu: &mut app.session_menu_id,
            anchor: &mut app.session_menu_anchor,
            actions: &mut app.actions,
            project_menu: &mut app.project_menu,
            expanded: &mut app.session_tree_expanded,
            rename_id: &mut app.session_rename_id,
            rename_value: &mut app.session_rename_value,
            sessions: &mut app.sessions,
            projects: &mut app.project_sessions,
        },
        event,
    ))
}
fn session_info(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}
#[test]
fn session_context_menu_and_rename_state_are_updated_synchronously() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.project_sessions
        .insert("/tmp/other".to_string(), vec![session_info("s2", "Second", "/tmp/other")]);

    let _ = apply(&mut app, Event::RightClick("s1".to_string(), 10.0, 20.0));
    assert_eq!(app.session_menu_id.as_deref(), Some("s1"));
    assert_eq!(app.session_menu_anchor.expect("anchor").x, 10.0);

    let _ = apply(&mut app, Event::Close);
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());

    let _ = apply(&mut app, Event::Rename("s2".to_string()));
    assert_eq!(app.session_rename_id.as_deref(), Some("s2"));
    assert_eq!(app.session_rename_value, "Second");

    let _ = apply(&mut app, Event::Change(" Renamed ".to_string()));
    assert_eq!(app.session_rename_value, " Renamed ");

    let _ = apply(&mut app, Event::Cancel);
    assert!(app.session_rename_id.is_none());
    assert!(app.session_rename_value.is_empty());
}

#[test]
fn session_tree_toggle_updates_non_persistent_expansion_state() {
    let mut app = app();
    assert!(app.session_tree_expanded.is_empty());

    let first = apply(&mut app, Event::Toggle("parent".to_string()));
    assert!(first.is_some());
    assert!(app.session_tree_expanded.contains("parent"));

    let second = apply(&mut app, Event::Toggle("parent".to_string()));
    assert!(second.is_some());
    assert!(!app.session_tree_expanded.contains("parent"));
}

#[test]
fn session_rename_save_ignores_missing_or_empty_title_and_updates_local_titles() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.project_sessions
        .insert("/tmp/project".to_string(), vec![session_info("s1", "First", "/tmp/project")]);

    let _ = apply(&mut app, Event::Save);
    assert_eq!(app.sessions[0].title, "First");

    app.session_rename_id = Some("s1".to_string());
    app.session_rename_value = "   ".to_string();
    let _ = apply(&mut app, Event::Save);
    assert_eq!(app.sessions[0].title, "First");
    assert_eq!(app.session_rename_id.as_deref(), Some("s1"));

    app.session_rename_value = "Updated".to_string();
    let task = apply(&mut app, Event::Save);

    assert!(task.is_some());
    assert!(app.session_rename_id.is_none());
    assert!(app.session_rename_value.is_empty());
    assert_eq!(app.sessions[0].title, "Updated");
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap()[0].title, "Updated");
}

#[test]
fn session_title_double_click_enters_rename_mode_and_closes_actions() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.session_menu_id = Some("s1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(1.0, 2.0));
    app.actions = true;

    let _ = apply(&mut app, Event::Title("s1".to_string(), Some(0)));
    let _ = apply(&mut app, Event::Title("s1".to_string(), Some(0)));

    assert_eq!(app.session_rename_id.as_deref(), Some("s1"));
    assert_eq!(app.session_rename_value, "First");
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(!app.actions);
}
