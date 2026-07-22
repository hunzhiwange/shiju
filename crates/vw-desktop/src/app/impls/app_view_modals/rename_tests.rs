#[test]
fn rename_tests_module_is_wired() {
    assert!(module_path!().ends_with("rename_tests"));
}

use iced::widget::text;

fn session(id: &str, title: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 2,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

#[test]
fn file_tree_rename_without_path_leaves_root_content() {
    let (app, _) = crate::app::App::new();

    let element = super::with_file_tree_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn file_tree_rename_builds_modal_from_path_filename() {
    let (mut app, _) = crate::app::App::new();
    app.file_tree_rename_path = Some("/workspace/src/main.rs".to_string());
    app.file_tree_rename_value = "lib.rs".to_string();

    assert_eq!(super::file_tree_rename_old_name(app.file_tree_rename_path.as_deref()), "main.rs");
    let element = super::with_file_tree_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn file_tree_rename_builds_modal_when_path_has_no_filename() {
    let (mut app, _) = crate::app::App::new();
    app.file_tree_rename_path = Some(String::new());

    assert_eq!(super::file_tree_rename_old_name(app.file_tree_rename_path.as_deref()), "");
    let element = super::with_file_tree_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn file_tree_rename_old_name_is_empty_for_missing_or_root_path() {
    assert_eq!(super::file_tree_rename_old_name(None), "");
    assert_eq!(super::file_tree_rename_old_name(Some("/")), "");
}

#[test]
fn session_rename_uses_global_session_title() {
    let (mut app, _) = crate::app::App::new();
    app.session_rename_id = Some("session-1".to_string());
    app.session_rename_value = "new title".to_string();
    app.sessions.push(session("session-1", "old title"));

    assert_eq!(super::session_rename_old_title(&app, "session-1"), "old title");
    let element = super::with_session_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn session_rename_falls_back_to_project_session_title() {
    let (mut app, _) = crate::app::App::new();
    app.session_rename_id = Some("session-1".to_string());
    app.session_rename_value = "new title".to_string();
    app.project_sessions
        .insert("/workspace".to_string(), vec![session("session-1", "project title")]);

    assert_eq!(super::session_rename_old_title(&app, "session-1"), "project title");
    let element = super::with_session_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn session_rename_prefers_global_session_over_project_session() {
    let (mut app, _) = crate::app::App::new();
    app.session_rename_id = Some("session-1".to_string());
    app.sessions.push(session("session-1", "global title"));
    app.project_sessions
        .insert("/workspace".to_string(), vec![session("session-1", "project title")]);

    assert_eq!(super::session_rename_old_title(&app, "session-1"), "global title");
    let element = super::with_session_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn session_rename_builds_modal_when_session_is_unknown() {
    let (mut app, _) = crate::app::App::new();
    app.session_rename_id = Some("missing-session".to_string());

    assert_eq!(super::session_rename_old_title(&app, "missing-session"), "");
    let element = super::with_session_rename(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn session_rename_without_id_leaves_root_content() {
    let (app, _) = crate::app::App::new();

    let element = super::with_session_rename(&app, text("root").into());

    std::hint::black_box(element);
}
