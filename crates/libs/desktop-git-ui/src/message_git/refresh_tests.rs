use crate::message_git::messages::GitMessage;
use crate::state::State as Fixture;
fn test_app() -> Fixture {
    Fixture::default()
}
fn update(app: &mut Fixture, message: GitMessage) {
    let _ = crate::message_git::controller::refresh::update(
        app,
        &Default::default(),
        &mut Vec::new(),
        message,
    );
}
#[test]
fn diff_file_metas_ready_ignores_stale_repo_path() {
    let mut app = Fixture::default();
    app.git_diff_file_metas_repo_path = Some("/current".to_string());
    app.git_diff_file_metas_loading = true;

    let _task = update(
        &mut app,
        GitMessage::DiffFileMetasReady { repo_path: Some("/old".to_string()), metas: Vec::new() },
    );

    assert!(app.git_diff_file_metas.is_empty());
    assert!(app.git_diff_file_metas_loading);
}
#[test]
fn diff_file_metas_ready_clears_cached_diff_contents() {
    let mut app = Fixture::default();
    app.git_diff_file_metas_repo_path = Some("/repo".to_string());
    app.git_diff_file_metas_loading = true;
    app.git_diff_contents.insert("old.rs".to_string(), ("old".to_string(), "new".to_string()));
    app.git_diff_contents_loading.insert("pending.rs".to_string());

    let _task = update(
        &mut app,
        GitMessage::DiffFileMetasReady { repo_path: Some("/repo".to_string()), metas: Vec::new() },
    );

    assert!(!app.git_diff_file_metas_loading);
    assert!(app.git_diff_file_metas.is_empty());
    assert!(app.git_diff_contents.is_empty());
    assert!(app.git_diff_contents_loading.is_empty());
}
