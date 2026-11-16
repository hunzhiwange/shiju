use crate::project_message::session_open::{self, ListEffect, ListEvent, SessionLists};
use std::collections::{HashMap, HashSet};
#[derive(Default)]
struct Fixture {
    sessions: Vec<session_info::Info>,
    session_previews: HashMap<String, String>,
    archived_session_ids: HashSet<String>,
    project_sessions: HashMap<String, Vec<session_info::Info>>,
    counts: HashMap<String, usize>,
    project_sessions_loading: HashSet<String>,
    scrollbars: HashMap<String, bool>,
    notifications: Vec<Notification>,
    error_message: Option<String>,
}
struct Notification {
    message: String,
}
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, event: ListEvent) -> Option<Vec<ListEffect>> {
    let effects = session_open::update_lists(
        &mut SessionLists {
            sessions: &mut app.sessions,
            previews: &mut app.session_previews,
            archived: &mut app.archived_session_ids,
            projects: &mut app.project_sessions,
            counts: &mut app.counts,
            loading: &mut app.project_sessions_loading,
            scrollbars: &mut app.scrollbars,
        },
        &None,
        event,
    );
    for effect in &effects {
        match effect {
            ListEffect::Notification(message) => {
                app.notifications.push(Notification { message: message.clone() })
            }
            ListEffect::Error(message) => app.error_message = Some(message.clone()),
            ListEffect::Load(_) => {}
        }
    }
    Some(effects)
}
fn session_info(id: &str, additions: i64, deletions: i64) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: Some(session_info::Summary { additions, deletions, files: 1, diffs: None }),
        share: None,
        title: id.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}
#[test]
fn sessions_loaded_updates_list_or_pushes_notification() {
    let mut app = app();
    let sessions = vec![session_info("s1", 1, 0)];

    let task = apply(&mut app, ListEvent::Loaded(Ok(sessions.clone())));

    assert!(task.is_some());
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].id, "s1");

    let _ = apply(&mut app, ListEvent::Loaded(Err("boom".to_string())));
    assert!(app.notifications.iter().any(|item| item.message.contains("boom")));
}
#[test]
fn session_bootstrap_loaded_keeps_metadata_and_reports_session_error() {
    let mut app = app();
    let previews = std::collections::HashMap::from([(
        "failed-session".to_string(),
        "cached preview".to_string(),
    )]);
    let archived = std::collections::HashSet::from(["archived-session".to_string()]);

    let _ = apply(
        &mut app,
        ListEvent::Bootstrap {
            project_path: None,
            result: Err("gateway unavailable".to_string()),
            previews: previews.clone(),
            archived: archived.clone(),
        },
    );

    assert_eq!(app.session_previews, previews);
    assert_eq!(app.archived_session_ids, archived);
    assert!(app.sessions.is_empty());
    assert!(app.notifications.iter().any(|item| item.message.contains("gateway unavailable")));
}
#[test]
fn recent_hover_marks_uncached_project_loading_once() {
    let mut app = app();
    let path = "/tmp/uncached-project".to_string();

    let first_task = apply(&mut app, ListEvent::Hover(path.clone()));

    assert!(first_task.is_some());
    assert!(app.project_sessions_loading.contains(&path));

    let second_task = apply(&mut app, ListEvent::Hover(path.clone()));

    assert!(second_task.is_some());
    assert_eq!(app.project_sessions_loading.len(), 1);
}
#[test]
fn project_sessions_loaded_records_error_message() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions_loading.insert(path.clone());

    let _ = apply(&mut app, ListEvent::ProjectLoaded(path.clone(), Err("offline".to_string())));

    assert!(!app.project_sessions_loading.contains(&path));
    assert_eq!(app.error_message.as_deref(), Some("Failed to load project sessions: offline"));
}
#[test]
fn project_load_sessions_skips_duplicate_loads_and_marks_new_load() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions_loading.insert(path.clone());

    let _ = apply(&mut app, ListEvent::Load(path.clone()));
    assert_eq!(app.project_sessions_loading.len(), 1);

    app.project_sessions_loading.clear();
    let task = apply(&mut app, ListEvent::Load(path.clone()));

    assert!(task.is_some());
    assert!(app.project_sessions_loading.contains(&path));
}
