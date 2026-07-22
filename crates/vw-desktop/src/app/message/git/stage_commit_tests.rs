use crate::app::message::git::GitMessage;
use crate::app::state::ConventionalCommitType;
use iced::widget::text_editor;

#[cfg(not(target_arch = "wasm32"))]
fn changed_file_fixture(old_content: &str, new_content: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    let path = temp.path().join("note.txt");
    std::fs::write(&path, old_content).expect("write original file");

    let mut index = repo.index().expect("index");
    index.add_path(std::path::Path::new("note.txt")).expect("stage original file");
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("tree");
    let signature = git2::Signature::now("VibeWindow Test", "test@example.com").expect("signature");
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .expect("initial commit");

    std::fs::write(path, new_content).expect("write changed file");
    temp
}

#[test]
fn commit_form_messages_update_fields_and_modals() {
    let (mut app, _task) = crate::app::App::new();

    let _task =
        super::stage_commit::update(&mut app, GitMessage::CommitMessageChanged("summary".into()));
    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::CommitTypeSelected(ConventionalCommitType::Fix),
    );
    let _task = super::stage_commit::update(&mut app, GitMessage::CommitScopeChanged("ui".into()));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::CommitDescriptionChanged("body".into()));
    let _task = super::stage_commit::update(&mut app, GitMessage::CommitHelpOpen);
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterHelpOpen);

    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Fix));
    assert_eq!(app.git_commit_scope, "ui");
    assert_eq!(app.git_commit_description, "body");
    assert!(app.show_git_commit_help_modal);
    assert!(app.show_git_filter_help_modal);

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitHelpClose);
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterHelpClose);

    assert!(!app.show_git_commit_help_modal);
    assert!(!app.show_git_filter_help_modal);
}

#[test]
fn commit_description_editor_action_synchronizes_plain_text() {
    let (mut app, _task) = crate::app::App::new();

    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::CommitDescriptionEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(std::sync::Arc::new("first\nsecond".to_string())),
        )),
    );

    assert_eq!(app.git_commit_description_editor.text(), "first\nsecond");
    assert_eq!(app.git_commit_description, "first\nsecond");
}

#[test]
fn filter_messages_toggle_and_clear_filter_state() {
    let (mut app, _task) = crate::app::App::new();

    let _task = super::stage_commit::update(&mut app, GitMessage::ToggleFilterOptions(true));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterQueryChanged("src".into()));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterToggleIncluded(true));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterToggleExcluded(true));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterToggleNew(true));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterToggleModified(true));
    let _task = super::stage_commit::update(&mut app, GitMessage::FilterToggleDeleted(true));

    assert!(app.show_git_filter_options);
    assert_eq!(app.git_filter_query, "src");
    assert!(app.git_filter_included);
    assert!(app.git_filter_excluded);
    assert!(app.git_filter_new);
    assert!(app.git_filter_modified);
    assert!(app.git_filter_deleted);

    let _task = super::stage_commit::update(&mut app, GitMessage::ClearFilters);

    assert!(app.git_filter_query.is_empty());
    assert!(!app.git_filter_included);
    assert!(!app.git_filter_excluded);
    assert!(!app.git_filter_new);
    assert!(!app.git_filter_modified);
    assert!(!app.git_filter_deleted);
}

#[test]
fn stage_selection_toggles_are_deduplicated_and_removable() {
    let (mut app, _task) = crate::app::App::new();

    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageFile("a.rs".into(), true));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageFile("a.rs".into(), true));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageHunk("a.rs".into(), 1, true));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageHunk("a.rs".into(), 1, true));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageLine("a.rs".into(), 2, true));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageLine("a.rs".into(), 2, true));
    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::ToggleStageOldLine("a.rs".into(), 3, true),
    );
    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::ToggleStageOldLine("a.rs".into(), 3, true),
    );

    assert_eq!(app.staged_files_selected, vec!["a.rs"]);
    assert_eq!(app.staged_hunks_selected, vec![("a.rs".to_string(), 1)]);
    assert_eq!(app.staged_lines_selected, vec![("a.rs".to_string(), 2)]);
    assert_eq!(app.staged_old_lines_selected, vec![("a.rs".to_string(), 3)]);

    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageFile("a.rs".into(), false));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageHunk("a.rs".into(), 1, false));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::ToggleStageLine("a.rs".into(), 2, false));
    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::ToggleStageOldLine("a.rs".into(), 3, false),
    );

    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
}

#[test]
fn clear_all_file_lines_only_removes_matching_file() {
    let (mut app, _task) = crate::app::App::new();
    app.staged_lines_selected = vec![("a.rs".to_string(), 1), ("b.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 3), ("b.rs".to_string(), 4)];

    let _task = super::stage_commit::update(&mut app, GitMessage::ClearAllFileLines("a.rs".into()));

    assert_eq!(app.staged_lines_selected, vec![("b.rs".to_string(), 2)]);
    assert_eq!(app.staged_old_lines_selected, vec![("b.rs".to_string(), 4)]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn select_all_file_lines_replaces_only_target_with_changed_lines() {
    let temp = changed_file_fixture("keep\nold\nremove\n", "keep\nnew\n");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(temp.path().to_string_lossy().into_owned());
    app.staged_lines_selected = vec![("note.txt".to_string(), 99), ("other.txt".to_string(), 7)];
    app.staged_old_lines_selected =
        vec![("note.txt".to_string(), 99), ("other.txt".to_string(), 8)];

    let _task =
        super::stage_commit::update(&mut app, GitMessage::SelectAllFileLines("note.txt".into()));

    assert_eq!(
        app.staged_lines_selected,
        vec![("note.txt".to_string(), 1), ("other.txt".to_string(), 7)]
    );
    assert_eq!(
        app.staged_old_lines_selected,
        vec![
            ("note.txt".to_string(), 1),
            ("note.txt".to_string(), 2),
            ("other.txt".to_string(), 8),
        ]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn visible_line_selection_skips_missing_files_and_inverts_changed_lines() {
    let temp = changed_file_fixture("same\nold\n", "same\nnew\nadded\n");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(temp.path().to_string_lossy().into_owned());

    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::SelectAllVisibleFileLines(vec!["missing.txt".into(), "note.txt".into()]),
    );

    assert_eq!(
        app.staged_lines_selected,
        vec![("note.txt".to_string(), 1), ("note.txt".to_string(), 2)]
    );
    assert_eq!(app.staged_old_lines_selected, vec![("note.txt".to_string(), 1)]);

    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::InvertVisibleFileLines(vec!["missing.txt".into(), "note.txt".into()]),
    );

    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
}

#[test]
fn hover_messages_set_and_clear_only_matching_header() {
    let (mut app, _task) = crate::app::App::new();

    let _task =
        super::stage_commit::update(&mut app, GitMessage::HoverFileHeaderEnter("a.rs".into()));
    let _task =
        super::stage_commit::update(&mut app, GitMessage::HoverFileHeaderExit("b.rs".into()));

    assert_eq!(app.git_hovered_file_header.as_deref(), Some("a.rs"));

    let _task =
        super::stage_commit::update(&mut app, GitMessage::HoverFileHeaderExit("a.rs".into()));
    let _task = super::stage_commit::update(&mut app, GitMessage::HoverGitPanelHeaderEnter);

    assert_eq!(app.git_hovered_file_header, None);
    assert!(app.git_panel_header_hovered);

    let _task = super::stage_commit::update(&mut app, GitMessage::HoverGitPanelHeaderExit);

    assert!(!app.git_panel_header_hovered);
}

#[test]
fn commit_selected_rejects_blank_message_without_starting_commit() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_message = "   ".to_string();
    app.git_commit_description.clear();
    app.git_commit_type = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

#[test]
fn commit_selected_while_in_progress_preserves_existing_state() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.error_message = Some("existing error".to_string());

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.error_message.as_deref(), Some("existing error"));
}

#[test]
fn commit_selected_without_project_path_restores_idle_state_and_reports_error() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;
    app.git_commit_message = "summary".to_string();

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关提交"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("未找到项目路径，无法通过网关提交")
    );
}

#[test]
fn commit_selected_with_valid_context_enters_in_progress_state() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/repo".to_string());
    app.git_commit_message = "summary".to_string();
    app.git_commit_in_progress = false;
    app.error_message = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert!(app.error_message.is_none());
}

#[test]
fn commit_selected_finished_success_resets_form_and_reports_success() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git".to_string();
    app.git_commit_description = "details".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];
    app.staged_hunks_selected = vec![("a.rs".to_string(), 1)];
    app.staged_lines_selected = vec![("a.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 3)];

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelectedFinished(Ok(())));

    assert!(!app.git_commit_in_progress);
    assert!(app.git_commit_message.is_empty());
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(app.git_commit_scope.is_empty());
    assert!(app.git_commit_description.is_empty());
    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("提交完成")
    );
}

#[test]
fn commit_selected_finished_error_keeps_form_and_reports_error() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];

    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::CommitSelectedFinished(Err("failed".to_string())),
    );

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.staged_files_selected, vec!["a.rs"]);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("提交失败: failed")
    );
}
