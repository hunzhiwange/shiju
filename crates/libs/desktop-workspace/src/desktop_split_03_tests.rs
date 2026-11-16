use iced::widget::text;
fn session(id: &str, title: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn session_rename_uses_global_session_title() {
    let mut sessions = Vec::new();
    let project_sessions = std::collections::HashMap::new();
    let session_id = Some("session-1".to_string());
    let value = "new title".to_string();

    sessions.push(session("session-1", "old title"));

    assert_eq!(
        crate::views::modals::rename::session_rename_old_title(
            &sessions,
            &project_sessions,
            "session-1"
        ),
        "old title"
    );
    let element = crate::views::modals::rename::with_session_rename(
        session_id.as_deref(),
        &value,
        &sessions,
        &project_sessions,
        Default::default(),
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn session_rename_falls_back_to_project_session_title() {
    let sessions = Vec::new();
    let mut project_sessions = std::collections::HashMap::new();
    let session_id = Some("session-1".to_string());
    let value = "new title".to_string();

    project_sessions.insert("/workspace".to_string(), vec![session("session-1", "project title")]);

    assert_eq!(
        crate::views::modals::rename::session_rename_old_title(
            &sessions,
            &project_sessions,
            "session-1"
        ),
        "project title"
    );
    let element = crate::views::modals::rename::with_session_rename(
        session_id.as_deref(),
        &value,
        &sessions,
        &project_sessions,
        Default::default(),
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn session_rename_prefers_global_session_over_project_session() {
    let mut sessions = Vec::new();
    let mut project_sessions = std::collections::HashMap::new();
    let session_id = Some("session-1".to_string());
    let value = String::new();

    sessions.push(session("session-1", "global title"));
    project_sessions.insert("/workspace".to_string(), vec![session("session-1", "project title")]);

    assert_eq!(
        crate::views::modals::rename::session_rename_old_title(
            &sessions,
            &project_sessions,
            "session-1"
        ),
        "global title"
    );
    let element = crate::views::modals::rename::with_session_rename(
        session_id.as_deref(),
        &value,
        &sessions,
        &project_sessions,
        Default::default(),
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn session_rename_builds_modal_when_session_is_unknown() {
    let sessions = Vec::new();
    let project_sessions = std::collections::HashMap::new();
    let session_id = Some("missing-session".to_string());
    let value = String::new();

    assert_eq!(
        crate::views::modals::rename::session_rename_old_title(
            &sessions,
            &project_sessions,
            "missing-session"
        ),
        ""
    );
    let element = crate::views::modals::rename::with_session_rename(
        session_id.as_deref(),
        &value,
        &sessions,
        &project_sessions,
        Default::default(),
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn session_rename_without_id_leaves_root_content() {
    let sessions = Vec::new();
    let project_sessions = std::collections::HashMap::new();
    let session_id: Option<String> = None;
    let value = String::new();

    let element = crate::views::modals::rename::with_session_rename(
        session_id.as_deref(),
        &value,
        &sessions,
        &project_sessions,
        Default::default(),
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

fn keep_element(element: iced::Element<'_, crate::views::modals::recent_project_remove::Event>) {
    let _ = element.as_widget().size();
}
#[test]
fn without_pending_path_returns_root_content_unchanged() {
    let path = None;
    let projects = vec![];
    let names = vec![];
    let root = text("root").into();
    keep_element(crate::views::modals::recent_project_remove::with_recent_project_remove_confirm(
        path.as_ref(),
        &projects,
        &names,
        &[],
        Default::default(),
        |event| event,
        root,
    ));
}

#[test]
fn with_pending_path_builds_confirm_overlay() {
    let projects = vec!["/tmp/demo-project".to_string()];
    let names = vec!["Demo".to_string()];
    let path = Some("/tmp/demo-project".to_string());
    let root = text("root").into();
    keep_element(crate::views::modals::recent_project_remove::with_recent_project_remove_confirm(
        path.as_ref(),
        &projects,
        &names,
        &[],
        Default::default(),
        |event| event,
        root,
    ));
}
