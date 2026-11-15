use std::collections::{HashMap, HashSet};
use crate::git_panel::ops::DiffFileMeta;
#[derive(Default)]
struct Fixture {
    git_diff_file_metas_repo_path: Option<String>,
    git_diff_file_metas: Vec<DiffFileMeta>,
    git_diff_file_metas_loading: bool,
    git_diff_contents: HashMap<String, (String, String)>,
    git_diff_contents_loading: HashSet<String>,
}
enum GitMessage { DiffFileMetasReady { repo_path: Option<String>, metas: Vec<DiffFileMeta> } }
fn update(app: &mut Fixture, message: GitMessage) {
    let GitMessage::DiffFileMetasReady { repo_path, metas } = message;
    let _ = super::apply_diff_metas(&mut app.git_diff_file_metas_repo_path, &mut app.git_diff_file_metas,
        &mut app.git_diff_file_metas_loading, &mut app.git_diff_contents, &mut app.git_diff_contents_loading, repo_path, metas);
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
