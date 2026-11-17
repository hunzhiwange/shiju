//! 原宿主入口的领域状态与效果断言，调用真实领域路由。

use crate::git_panel::diff_view::GitDiffCommentDraft;
use crate::git_panel::diff_view::GitDiffLineRange;
use crate::git_panel::types::GitWorktreeOption;
use crate::message_git::messages::GitMessage;
use crate::test_support::Fixture;
use crate::test_support::update;
use desktop_workspace::workspace_state::GitDiffSelectedLine;

fn test_app() -> Fixture {
    Fixture::default()
}

#[test]
fn refresh_worktree_options_without_project_clears_state() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.git.git_worktree_options = vec![GitWorktreeOption {
        directory: "/repo".to_string(),
        label: "repo".to_string(),
        branch: Some("main".to_string()),
    }];
    app.git.selected_git_worktree_directory = Some("/repo".to_string());
    app.git.git_worktree_options_loading = true;
    app.git.git_worktree_options_project_path = Some("/repo".to_string());
    app.git.git_worktree_menu_open = true;

    let _task = update(&mut app, GitMessage::RefreshWorktreeOptions);

    assert!(app.git.git_worktree_options.is_empty());
    assert_eq!(app.git.selected_git_worktree_directory, None);
    assert!(!app.git.git_worktree_options_loading);
    assert_eq!(app.git.git_worktree_options_project_path, None);
    assert!(!app.git.git_worktree_menu_open);
}

#[test]
fn refresh_worktree_options_with_project_marks_request_loading() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("init repo");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = Fixture::new();
    app.project_path = Some(project_path.clone());

    let _task = update(&mut app, GitMessage::RefreshWorktreeOptions);

    assert!(app.git.git_worktree_options_loading);
    assert_eq!(app.git.git_worktree_options_project_path.as_deref(), Some(project_path.as_str()));
}

#[test]
fn refresh_worktree_options_while_same_project_is_loading_preserves_options() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("init repo");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = Fixture::new();
    app.project_path = Some(project_path.clone());
    app.git.git_worktree_options_loading = true;
    app.git.git_worktree_options_project_path = Some(project_path.clone());
    app.git.git_worktree_options = vec![GitWorktreeOption {
        directory: project_path.clone(),
        label: "main".to_string(),
        branch: Some("main".to_string()),
    }];

    let _task = update(&mut app, GitMessage::RefreshWorktreeOptions);

    assert_eq!(app.git.git_worktree_options.len(), 1);
    assert_eq!(app.git.git_worktree_options[0].directory, project_path);
    assert!(app.git.git_worktree_options_loading);
}

#[test]
fn worktree_options_ready_ignores_stale_project_path() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_options_project_path = Some("/current".to_string());
    app.git.git_worktree_options_loading = true;

    let _task = update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "/stale".to_string(),
            result: Ok(vec![GitWorktreeOption {
                directory: "/stale".to_string(),
                label: "stale".to_string(),
                branch: Some("old".to_string()),
            }]),
        },
    );

    assert!(app.git.git_worktree_options.is_empty());
    assert!(app.git.git_worktree_options_loading);
}

#[test]
fn worktree_options_ready_selects_matching_directory_and_branch() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_options_project_path = Some("/project".to_string());
    app.git.git_worktree_options_loading = true;
    app.git.selected_git_worktree_directory = Some("/worktree/".to_string());

    let _task = update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "/project".to_string(),
            result: Ok(vec![
                GitWorktreeOption {
                    directory: "/project".to_string(),
                    label: "main".to_string(),
                    branch: Some("main".to_string()),
                },
                GitWorktreeOption {
                    directory: "/worktree".to_string(),
                    label: "feature".to_string(),
                    branch: Some("feature".to_string()),
                },
            ]),
        },
    );

    assert!(!app.git.git_worktree_options_loading);
    assert_eq!(app.git.selected_git_worktree_directory.as_deref(), Some("/worktree"));
    assert_eq!(app.git.selected_branch.as_deref(), Some("feature"));
}

#[test]
fn worktree_options_ready_normalizes_directory_separators() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_options_project_path = Some("C:/project".to_string());
    app.git.selected_git_worktree_directory = Some("C:/worktree".to_string());

    let _task = update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "C:/project".to_string(),
            result: Ok(vec![GitWorktreeOption {
                directory: "C:\\worktree\\".to_string(),
                label: "feature".to_string(),
                branch: Some("feature".to_string()),
            }]),
        },
    );

    assert_eq!(app.git.selected_git_worktree_directory.as_deref(), Some("C:\\worktree\\"));
    assert_eq!(app.git.selected_branch.as_deref(), Some("feature"));
}

#[test]
fn worktree_options_ready_falls_back_to_first_option() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_options_project_path = Some("/project".to_string());
    app.git.selected_git_worktree_directory = Some("/missing".to_string());

    let _task = update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "/project".to_string(),
            result: Ok(vec![GitWorktreeOption {
                directory: "/first".to_string(),
                label: "first".to_string(),
                branch: Some("main".to_string()),
            }]),
        },
    );

    assert_eq!(app.git.selected_git_worktree_directory.as_deref(), Some("/first"));
    assert_eq!(app.git.selected_branch.as_deref(), Some("main"));
}

#[test]
fn worktree_options_ready_with_empty_result_clears_selected_directory() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_options_project_path = Some("/project".to_string());
    app.git.git_worktree_options_loading = true;
    app.git.selected_git_worktree_directory = Some("/stale".to_string());

    let _task = update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "/project".to_string(),
            result: Ok(Vec::new()),
        },
    );

    assert!(!app.git.git_worktree_options_loading);
    assert!(app.git.git_worktree_options.is_empty());
    assert_eq!(app.git.selected_git_worktree_directory, None);
}

#[test]
fn select_git_worktree_resets_diff_state_and_updates_branch() {
    let (mut app, _task) = Fixture::new();
    app.git.git_worktree_menu_open = true;
    app.git.git_changed_files = vec!["src/lib.rs".to_string()];
    app.git.git_changed_files_loading = true;
    app.git.git_changed_files_repo_path = Some("/old".to_string());
    app.git.git_diff_file_metas_loading = true;
    app.git.git_diff_file_metas_repo_path = Some("/old".to_string());
    app.git
        .git_diff_contents
        .insert("src/lib.rs".to_string(), ("old".to_string(), "new".to_string()));
    app.git.git_diff_contents_loading.insert("src/lib.rs".to_string());
    app.git.git_diff_selected_lines.push(desktop_workspace::workspace_state::GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 1,
        is_old: false,
        text: "new".to_string(),
    });
    app.git.git_diff_selected_range = Some(crate::git_panel::diff_view::GitDiffLineRange {
        file: "src/lib.rs".to_string(),
        start: 1,
        end: 2,
        is_old: false,
    });
    app.git.git_diff_comment_draft = Some(crate::git_panel::diff_view::GitDiffCommentDraft {
        range: crate::git_panel::diff_view::GitDiffLineRange {
            file: "src/lib.rs".to_string(),
            start: 1,
            end: 1,
            is_old: false,
        },
        editor: iced::widget::text_editor::Content::with_text("note"),
    });

    let _task = update(
        &mut app,
        GitMessage::SelectGitWorktree(GitWorktreeOption {
            directory: "/next".to_string(),
            label: "next".to_string(),
            branch: Some("feat".to_string()),
        }),
    );

    assert!(!app.git.git_worktree_menu_open);
    assert_eq!(app.git.selected_git_worktree_directory.as_deref(), Some("/next"));
    assert_eq!(app.git.selected_branch.as_deref(), Some("feat"));
    assert!(app.git.git_changed_files.is_empty());
    assert!(!app.git.git_changed_files_loading);
    assert_eq!(app.git.git_changed_files_repo_path, None);
    assert!(app.git.git_diff_file_metas.is_empty());
    assert!(!app.git.git_diff_file_metas_loading);
    assert_eq!(app.git.git_diff_file_metas_repo_path, None);
    assert!(app.git.git_diff_contents.is_empty());
    assert!(app.git.git_diff_contents_loading.is_empty());
    assert!(app.git.git_diff_selected_lines.is_empty());
    assert!(app.git.git_diff_selected_range.is_none());
    assert!(app.git.git_diff_comment_draft.is_none());
}

#[test]
fn toggle_git_worktree_menu_uses_requested_state() {
    let (mut app, _task) = Fixture::new();

    let _task = update(&mut app, GitMessage::ToggleGitWorktreeMenu(true));
    assert!(app.git.git_worktree_menu_open);

    let _task = update(&mut app, GitMessage::ToggleGitWorktreeMenu(false));
    assert!(!app.git.git_worktree_menu_open);
}

#[test]
fn refresh_changed_files_without_repository_clears_cached_state() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.git.selected_git_worktree_directory = None;
    app.git.git_changed_files = vec!["stale.rs".to_string()];
    app.git.git_changed_files_loading = false;
    app.git.git_changed_files_repo_path = Some("/stale".to_string());

    let _task = update(&mut app, GitMessage::RefreshChangedFiles);

    assert!(app.git.git_changed_files.is_empty());
    assert!(!app.git.git_changed_files_loading);
    assert_eq!(app.git.git_changed_files_repo_path, None);
}

#[test]
fn refresh_changed_files_while_loading_preserves_cached_state() {
    let (mut app, _task) = Fixture::new();
    app.git.git_changed_files = vec!["pending.rs".to_string()];
    app.git.git_changed_files_loading = true;
    app.git.git_changed_files_repo_path = Some("/repo".to_string());

    let _task = update(&mut app, GitMessage::RefreshChangedFiles);

    assert_eq!(app.git.git_changed_files, vec!["pending.rs"]);
    assert!(app.git.git_changed_files_loading);
    assert_eq!(app.git.git_changed_files_repo_path.as_deref(), Some("/repo"));
}

#[test]
fn refresh_changed_files_with_project_marks_request_loading() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("init repo");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = Fixture::new();
    app.project_path = Some(project_path.clone());

    let _task = update(&mut app, GitMessage::RefreshChangedFiles);

    assert!(app.git.git_changed_files_loading);
    assert_eq!(app.git.git_changed_files_repo_path.as_deref(), Some(project_path.as_str()));
}

#[test]
fn refresh_diff_file_metas_without_repository_clears_cached_contents() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.git.selected_git_worktree_directory = None;
    app.git.git_diff_file_metas_loading = true;
    app.git.git_diff_file_metas_repo_path = Some("/stale".to_string());
    app.git
        .git_diff_contents
        .insert("stale.rs".to_string(), ("old".to_string(), "new".to_string()));
    app.git.git_diff_contents_loading.insert("pending.rs".to_string());

    let _task = update(&mut app, GitMessage::RefreshDiffFileMetas);

    assert!(app.git.git_diff_file_metas.is_empty());
    assert!(!app.git.git_diff_file_metas_loading);
    assert_eq!(app.git.git_diff_file_metas_repo_path, None);
    assert!(app.git.git_diff_contents.is_empty());
    assert!(app.git.git_diff_contents_loading.is_empty());
}

#[test]
fn refresh_diff_file_metas_while_same_repo_is_loading_preserves_contents() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("init repo");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = Fixture::new();
    app.project_path = Some(project_path.clone());
    app.git.git_diff_file_metas_loading = true;
    app.git.git_diff_file_metas_repo_path = Some(project_path.clone());
    app.git
        .git_diff_contents
        .insert("pending.rs".to_string(), ("old".to_string(), "new".to_string()));

    let _task = update(&mut app, GitMessage::RefreshDiffFileMetas);

    assert!(app.git.git_diff_file_metas_loading);
    assert_eq!(app.git.git_diff_file_metas_repo_path.as_deref(), Some(project_path.as_str()));
    assert!(app.git.git_diff_contents.contains_key("pending.rs"));
}

#[test]
fn refresh_diff_file_metas_with_project_marks_request_loading() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("init repo");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = Fixture::new();
    app.project_path = Some(project_path.clone());

    let _task = update(&mut app, GitMessage::RefreshDiffFileMetas);

    assert!(app.git.git_diff_file_metas_loading);
    assert_eq!(app.git.git_diff_file_metas_repo_path.as_deref(), Some(project_path.as_str()));
}

#[test]
fn changed_files_ready_ignores_stale_repo_path() {
    let (mut app, _task) = Fixture::new();
    app.git.git_changed_files_repo_path = Some("/current".to_string());
    app.git.git_changed_files_loading = true;

    let _task = update(
        &mut app,
        GitMessage::ChangedFilesReady {
            repo_path: Some("/old".to_string()),
            files: vec!["a.rs".to_string()],
        },
    );

    assert!(app.git.git_changed_files.is_empty());
    assert!(app.git.git_changed_files_loading);
}

#[test]
fn diff_content_ready_for_stale_repo_preserves_current_loading_state() {
    let (mut app, _task) = Fixture::new();
    app.git.git_diff_file_metas_repo_path = Some("/current".to_string());
    app.git.git_diff_contents_loading.insert("src/lib.rs".to_string());

    let _task = update(
        &mut app,
        GitMessage::DiffContentReady {
            repo_path: Some("/stale".to_string()),
            file: "src/lib.rs".to_string(),
            old_content: "old".to_string(),
            new_content: "new".to_string(),
        },
    );

    assert!(app.git.git_diff_contents_loading.contains("src/lib.rs"));
    assert!(!app.git.git_diff_contents.contains_key("src/lib.rs"));
}

#[test]
fn load_diff_content_without_repository_does_not_mark_file_loading() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.git.selected_git_worktree_directory = None;

    let _task = update(&mut app, GitMessage::LoadDiffContent("src/lib.rs".to_string()));

    assert!(!app.git.git_diff_contents_loading.contains("src/lib.rs"));
    assert!(!app.git.git_diff_contents.contains_key("src/lib.rs"));
}

#[test]
fn diff_content_ready_ignores_unknown_file_but_clears_loading() {
    let (mut app, _task) = Fixture::new();
    app.git.git_diff_file_metas_repo_path = Some("/repo".to_string());
    app.git.git_diff_contents_loading.insert("src/lib.rs".to_string());

    let _task = update(
        &mut app,
        GitMessage::DiffContentReady {
            repo_path: Some("/repo".to_string()),
            file: "src/lib.rs".to_string(),
            old_content: "old".to_string(),
            new_content: "new".to_string(),
        },
    );

    assert!(!app.git.git_diff_contents_loading.contains("src/lib.rs"));
    assert!(!app.git.git_diff_contents.contains_key("src/lib.rs"));
}
