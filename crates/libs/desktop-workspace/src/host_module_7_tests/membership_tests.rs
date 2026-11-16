use crate::project_message::session::lifecycle::SessionMembership;
#[derive(Default)]
struct Fixture {
    project_path: Option<String>,
    sessions: Vec<session_info::Info>,
    project_sessions: std::collections::HashMap<String, Vec<session_info::Info>>,
}
fn app() -> Fixture {
    Fixture::default()
}
fn apply(app: &mut Fixture, info: Option<session_info::Info>) -> Option<()> {
    SessionMembership { sessions: &mut app.sessions, projects: &mut app.project_sessions }.insert(
        app.project_path.as_deref(),
        info?,
        false,
    );
    Some(())
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
fn session_copied_adds_to_current_and_project_session_lists_without_duplicates() {
    let mut app = app();
    let info = session_info("s1", "Copy", "/tmp/project");
    app.project_path = Some("/tmp/project".to_string());
    app.sessions = vec![info.clone()];

    let _ = apply(&mut app, Some(info.clone()));
    assert_eq!(app.sessions.iter().filter(|s| s.id == "s1").count(), 1);
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap().len(), 1);

    let second = session_info("s2", "Copy 2", "/tmp/project");
    let _ = apply(&mut app, Some(second));
    assert_eq!(app.sessions[0].id, "s2");
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap()[0].id, "s2");
}
#[test]
fn session_copied_without_active_project_uses_info_directory_bucket_only() {
    let mut app = app();
    let info = session_info("copy-dir", "Copy", "/tmp/copied");

    let task = apply(&mut app, Some(info));

    assert!(task.is_some());
    assert!(app.sessions.is_empty());
    assert_eq!(app.project_sessions.get("/tmp/copied").unwrap()[0].id, "copy-dir");
}
