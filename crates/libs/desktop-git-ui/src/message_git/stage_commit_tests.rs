use crate::git_panel::types::ConventionalCommitType;
use iced::widget::text_editor;

#[test]
fn commit_form_messages_update_fields_and_modals() {
    let mut git_commit_description: String = Default::default();
    let mut git_commit_message: String = Default::default();
    let mut git_commit_scope: String = Default::default();
    let mut git_commit_type: Option<ConventionalCommitType> = None;
    let mut show_git_commit_help_modal: bool = Default::default();
    let mut show_git_filter_help_modal: bool = Default::default();

    let _result =
        super::commit_message_changed(&mut git_commit_message, "summary".into());
    let _result = super::commit_type_selected(&mut git_commit_type, ConventionalCommitType::Fix);
    let _result = super::commit_scope_changed(&mut git_commit_scope, "ui".into());
    let _result =
        super::commit_description_changed(&mut git_commit_description, "body".into());
    let _result = super::commit_help_open(&mut show_git_commit_help_modal);
    let _result = super::filter_help_open(&mut show_git_filter_help_modal);

    assert_eq!(git_commit_message, "summary");
    assert_eq!(git_commit_type, Some(ConventionalCommitType::Fix));
    assert_eq!(git_commit_scope, "ui");
    assert_eq!(git_commit_description, "body");
    assert!(show_git_commit_help_modal);
    assert!(show_git_filter_help_modal);

    let _result = super::commit_help_close(&mut show_git_commit_help_modal);
    let _result = super::filter_help_close(&mut show_git_filter_help_modal);

    assert!(!show_git_commit_help_modal);
    assert!(!show_git_filter_help_modal);
}

#[test]
fn commit_description_editor_action_synchronizes_plain_text() {
    let mut git_commit_description: String = Default::default();
    let mut git_commit_description_editor: text_editor::Content = text_editor::Content::new();

    let _result = super::commit_description_editor_action(&mut git_commit_description_editor, &mut git_commit_description, text_editor::Action::Edit(
            text_editor::Edit::Paste(std::sync::Arc::new("first\nsecond".to_string())),
        ));

    assert_eq!(git_commit_description_editor.text(), "first\nsecond");
    assert_eq!(git_commit_description, "first\nsecond");
}

#[test]
fn filter_messages_toggle_and_clear_filter_state() {
    let mut git_filter_deleted: bool = Default::default();
    let mut git_filter_excluded: bool = Default::default();
    let mut git_filter_included: bool = Default::default();
    let mut git_filter_modified: bool = Default::default();
    let mut git_filter_new: bool = Default::default();
    let mut git_filter_query: String = Default::default();
    let mut show_git_filter_options: bool = Default::default();

    let _result = super::toggle_filter_options(&mut show_git_filter_options, true);
    let _result = super::filter_query_changed(&mut git_filter_query, "src".into());
    let _result = super::filter_toggle_included(&mut git_filter_included, true);
    let _result = super::filter_toggle_excluded(&mut git_filter_excluded, true);
    let _result = super::filter_toggle_new(&mut git_filter_new, true);
    let _result = super::filter_toggle_modified(&mut git_filter_modified, true);
    let _result = super::filter_toggle_deleted(&mut git_filter_deleted, true);

    assert!(show_git_filter_options);
    assert_eq!(git_filter_query, "src");
    assert!(git_filter_included);
    assert!(git_filter_excluded);
    assert!(git_filter_new);
    assert!(git_filter_modified);
    assert!(git_filter_deleted);

    let _result = super::clear_filters(&mut git_filter_query, &mut git_filter_included, &mut git_filter_excluded, &mut git_filter_new, &mut git_filter_modified, &mut git_filter_deleted);

    assert!(git_filter_query.is_empty());
    assert!(!git_filter_included);
    assert!(!git_filter_excluded);
    assert!(!git_filter_new);
    assert!(!git_filter_modified);
    assert!(!git_filter_deleted);
}

#[test]
fn stage_selection_toggles_are_deduplicated_and_removable() {
    let mut staged_files_selected: Vec<String> = Default::default();
    let mut staged_hunks_selected: Vec<(String, usize)> = Default::default();
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();

    let _result =
        super::toggle_stage_file(&mut staged_files_selected, "a.rs".into(), true);
    let _result =
        super::toggle_stage_file(&mut staged_files_selected, "a.rs".into(), true);
    let _result =
        super::toggle_stage_hunk(&mut staged_hunks_selected, "a.rs".into(), 1, true);
    let _result =
        super::toggle_stage_hunk(&mut staged_hunks_selected, "a.rs".into(), 1, true);
    let _result =
        super::toggle_stage_line(&mut staged_lines_selected, "a.rs".into(), 2, true);
    let _result =
        super::toggle_stage_line(&mut staged_lines_selected, "a.rs".into(), 2, true);
    let _result = super::toggle_stage_old_line(&mut staged_old_lines_selected, "a.rs".into(), 3, true);
    let _result = super::toggle_stage_old_line(&mut staged_old_lines_selected, "a.rs".into(), 3, true);

    assert_eq!(staged_files_selected, vec!["a.rs"]);
    assert_eq!(staged_hunks_selected, vec![("a.rs".to_string(), 1)]);
    assert_eq!(staged_lines_selected, vec![("a.rs".to_string(), 2)]);
    assert_eq!(staged_old_lines_selected, vec![("a.rs".to_string(), 3)]);

    let _result =
        super::toggle_stage_file(&mut staged_files_selected, "a.rs".into(), false);
    let _result =
        super::toggle_stage_hunk(&mut staged_hunks_selected, "a.rs".into(), 1, false);
    let _result =
        super::toggle_stage_line(&mut staged_lines_selected, "a.rs".into(), 2, false);
    let _result = super::toggle_stage_old_line(&mut staged_old_lines_selected, "a.rs".into(), 3, false);

    assert!(staged_files_selected.is_empty());
    assert!(staged_hunks_selected.is_empty());
    assert!(staged_lines_selected.is_empty());
    assert!(staged_old_lines_selected.is_empty());
}

#[test]
fn clear_all_file_lines_only_removes_matching_file() {
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();
    staged_lines_selected = vec![("a.rs".to_string(), 1), ("b.rs".to_string(), 2)];
    staged_old_lines_selected = vec![("a.rs".to_string(), 3), ("b.rs".to_string(), 4)];

    let _result = super::clear_all_file_lines(&mut staged_lines_selected, &mut staged_old_lines_selected, "a.rs".into());

    assert_eq!(staged_lines_selected, vec![("b.rs".to_string(), 2)]);
    assert_eq!(staged_old_lines_selected, vec![("b.rs".to_string(), 4)]);
}

#[test]
fn hover_messages_set_and_clear_only_matching_header() {
    let mut git_hovered_file_header: Option<String> = Default::default();
    let mut git_panel_header_hovered: bool = Default::default();

    let _result =
        super::hover_file_header_enter(&mut git_hovered_file_header, "a.rs".into());
    let _result =
        super::hover_file_header_exit(&mut git_hovered_file_header, "b.rs".into());

    assert_eq!(git_hovered_file_header.as_deref(), Some("a.rs"));

    let _result =
        super::hover_file_header_exit(&mut git_hovered_file_header, "a.rs".into());
    let _result = super::hover_git_panel_header_enter(&mut git_panel_header_hovered);

    assert_eq!(git_hovered_file_header, None);
    assert!(git_panel_header_hovered);

    let _result = super::hover_git_panel_header_exit(&mut git_panel_header_hovered);

    assert!(!git_panel_header_hovered);
}

#[test]
fn expand_direction_variants_are_distinct() {
    assert_ne!(super::super::messages::ExpandDirection::Up, super::super::messages::ExpandDirection::Down);
    assert_eq!(super::super::messages::ExpandDirection::All, super::super::messages::ExpandDirection::All);
}

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


#[cfg(not(target_arch = "wasm32"))]
// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn select_all_file_lines_replaces_only_target_with_changed_lines() {
    let temp = changed_file_fixture("keep\nold\nremove\n", "keep\nnew\n");
    let path = temp.path().to_string_lossy().into_owned();
    let mut staged_lines_selected = vec![("note.txt".to_string(), 99), ("other.txt".to_string(), 7)];
    let mut staged_old_lines_selected =
        vec![("note.txt".to_string(), 99), ("other.txt".to_string(), 8)];

    let _result =
        super::select_all_file_lines(&mut staged_lines_selected, &mut staged_old_lines_selected, &path, "note.txt");

    assert_eq!(
        staged_lines_selected,
        vec![("note.txt".to_string(), 1), ("other.txt".to_string(), 7)]
    );
    assert_eq!(
        staged_old_lines_selected,
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
    let mut staged_lines_selected = Vec::new();
    let mut staged_old_lines_selected = Vec::new();
    let path = temp.path().to_string_lossy().into_owned();

    let _result = super::select_visible_file_lines(
        &mut staged_lines_selected, &mut staged_old_lines_selected, Some(&path),
        vec!["missing.txt".into(), "note.txt".into()], false,
    );

    assert_eq!(
        staged_lines_selected,
        vec![("note.txt".to_string(), 1), ("note.txt".to_string(), 2)]
    );
    assert_eq!(staged_old_lines_selected, vec![("note.txt".to_string(), 1)]);

    let _result = super::select_visible_file_lines(
        &mut staged_lines_selected, &mut staged_old_lines_selected, Some(&path),
        vec!["missing.txt".into(), "note.txt".into()], true,
    );

    assert!(staged_lines_selected.is_empty());
    assert!(staged_old_lines_selected.is_empty());
}
