//! 原宿主入口的领域状态与效果断言，调用真实领域路由。
use crate::DiffTheme;
use crate::git_panel::diff_view::GitDiffCommentDraft;
use crate::git_panel::diff_view::GitDiffLineRange;
use crate::message_git::controller::Effect;
use crate::message_git::messages::ExpandDirection;
use crate::message_git::messages::GitMessage;
use crate::test_support::Fixture;
use crate::test_support::Output as Message;
use crate::test_support::collect_task_messages;
use crate::test_support::update;
use desktop_workspace::workspace_state::GitDiffFileMenuState;
use desktop_workspace::workspace_state::GitDiffSelectedLine;

fn test_app() -> Fixture {
    Fixture::default()
}
fn selected(file: &str, line: usize, is_old: bool, text: &str) -> GitDiffSelectedLine {
    GitDiffSelectedLine { file: file.to_string(), line, is_old, text: text.to_string() }
}

fn range(file: &str, start: usize, end: usize, is_old: bool) -> GitDiffLineRange {
    GitDiffLineRange { file: file.to_string(), start, end, is_old }
}

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("diff_tests"));
}

#[test]
fn toggle_diff_highlight_updates_flag() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::ToggleDiffHighlight(false));
    assert!(!app.git.show_git_diff_highlight);

    let _ = update(&mut app, GitMessage::ToggleDiffHighlight(true));
    assert!(app.git.show_git_diff_highlight);
}

#[test]
fn context_menu_selects_line_when_nothing_is_selected() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::OpenDiffContextMenu {
            file: "src/main.rs".to_string(),
            line: 8,
            is_old: true,
            text: "-old".to_string(),
            x: 12.0,
            y: 24.0,
        },
    );

    assert_eq!(app.git.git_diff_selected_lines.len(), 1);
    assert_eq!(app.git.git_diff_selected_range.as_ref().map(|value| value.start), Some(8));
    assert_eq!(app.git.git_diff_context_menu.as_ref().map(|value| value.x), Some(12.0));

    let _ = update(&mut app, GitMessage::CloseDiffContextMenu);
    assert!(app.git.git_diff_context_menu.is_none());
}

#[test]
fn context_menu_preserves_existing_selection_when_opened_elsewhere() {
    let mut app = test_app();
    app.git.git_diff_selected_lines = vec![selected("selected.rs", 2, false, "+selected")];

    let _ = update(
        &mut app,
        GitMessage::OpenDiffContextMenu {
            file: "other.rs".to_string(),
            line: 9,
            is_old: true,
            text: "-other".to_string(),
            x: 4.0,
            y: 5.0,
        },
    );

    assert_eq!(app.git.git_diff_selected_lines.len(), 1);
    assert_eq!(app.git.git_diff_selected_lines[0].file, "selected.rs");
    assert_eq!(app.git.git_diff_selected_lines[0].line, 2);
    assert!(!app.git.git_diff_selected_lines[0].is_old);
    assert_eq!(app.git.git_diff_selected_lines[0].text, "+selected");
    assert!(app.git.git_diff_selected_range.is_none());
    assert_eq!(
        app.git.git_diff_context_menu.as_ref().map(|menu| (menu.file.as_str(), menu.line)),
        Some(("other.rs", 9))
    );
}

#[test]
fn file_menu_revert_and_cancel_discard_update_file_state() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::OpenDiffFileMenu("src/main.rs".to_string()));
    assert_eq!(
        app.git.git_diff_file_menu.as_ref().map(|value| value.file.as_str()),
        Some("src/main.rs")
    );

    let _ = update(&mut app, GitMessage::RevertDiffFile("src/main.rs".to_string()));
    assert!(app.git.git_diff_file_menu.is_none());
    assert_eq!(app.git.file_to_discard.as_deref(), Some("src/main.rs"));

    let _ = update(&mut app, GitMessage::CancelDiscardFile);
    assert_eq!(app.git.file_to_discard, None);
}

#[test]
fn close_file_menu_and_confirm_discard_update_only_file_state() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::OpenDiffFileMenu("first.rs".to_string()));
    let _ = update(&mut app, GitMessage::CloseDiffFileMenu);
    assert!(app.git.git_diff_file_menu.is_none());

    let _ = update(&mut app, GitMessage::OpenDiffFileMenu("second.rs".to_string()));
    let _ = update(&mut app, GitMessage::ConfirmDiscardFile("second.rs".to_string()));
    assert!(app.git.git_diff_file_menu.is_none());
    assert_eq!(app.git.file_to_discard.as_deref(), Some("second.rs"));
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_diff_file_prefers_deleted_content_and_falls_back_to_path() {
    let mut app = test_app();
    app.project_path = None;

    let deleted = collect_task_messages(update(
        &mut app,
        GitMessage::CopyDiffFile {
            file: "deleted.rs".to_string(),
            deleted_content: Some("deleted content".to_string()),
        },
    ))
    .await;
    assert!(
        matches!(deleted.as_slice(), [Message::Effect(Effect::CopyCode(text))] if text == "deleted content")
    );

    let existing = collect_task_messages(update(
        &mut app,
        GitMessage::CopyDiffFile { file: "existing.rs".to_string(), deleted_content: None },
    ))
    .await;
    assert!(
        matches!(existing.as_slice(), [Message::Effect(Effect::CopyFile(path))] if path == "existing.rs")
    );
}

#[test]
fn open_and_cancel_comment_draft_use_context_range() {
    let mut app = test_app();
    app.git.git_diff_selected_range = Some(range("src/main.rs", 3, 5, false));

    let _ = update(&mut app, GitMessage::OpenDiffCommentDraft);
    assert!(app.git.git_diff_comment_draft.is_some());
    assert_eq!(app.git.git_diff_comment_draft.as_ref().map(|draft| draft.range.end), Some(5));

    let _ = update(&mut app, GitMessage::DiffCommentCancel);
    assert!(app.git.git_diff_comment_draft.is_none());
    assert!(app.git.git_diff_selected_range.is_none());
}

#[test]
fn open_comment_draft_without_context_or_selection_is_noop() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::OpenDiffCommentDraft);

    assert!(app.git.git_diff_comment_draft.is_none());
    assert!(app.git.git_diff_selected_range.is_none());
}

#[test]
fn empty_context_stage_actions_leave_existing_stage_selection_unchanged() {
    let mut app = test_app();
    app.git.staged_lines_selected = vec![("kept.rs".to_string(), 2)];
    app.git.staged_old_lines_selected = vec![("kept.rs".to_string(), 1)];

    let _ = update(&mut app, GitMessage::SelectDiffContextStageLines);
    let _ = update(&mut app, GitMessage::ClearDiffContextStageLines);

    assert_eq!(app.git.staged_lines_selected, vec![("kept.rs".to_string(), 2)]);
    assert_eq!(app.git.staged_old_lines_selected, vec![("kept.rs".to_string(), 1)]);
}

#[test]
fn comment_submit_without_draft_preserves_selected_range() {
    let mut app = test_app();
    app.git.git_diff_selected_range = Some(range("src/main.rs", 2, 3, false));

    let _ = update(&mut app, GitMessage::DiffCommentSubmit);

    let selected_range =
        app.git.git_diff_selected_range.expect("selected range should be preserved");
    assert_eq!(selected_range.file, "src/main.rs");
    assert_eq!(selected_range.start, 2);
    assert_eq!(selected_range.end, 3);
    assert!(!selected_range.is_old);
}

#[tokio::test(flavor = "multi_thread")]
async fn comment_editor_action_updates_existing_draft_before_submit() {
    let mut app = test_app();
    app.git.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("src/lib.rs", 1, 1, false),
        editor: iced::widget::text_editor::Content::new(),
    });

    let _ = update(
        &mut app,
        GitMessage::DiffCommentEditorAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Insert('好'),
        )),
    );
    let messages = collect_task_messages(update(&mut app, GitMessage::DiffCommentSubmit)).await;

    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::AppendChat(text))]
            if text == "@src/lib.rs:2 #好 "
    ));
}

#[test]
fn context_stage_selection_adds_and_clears_both_diff_sides() {
    let mut app = test_app();
    app.git.git_diff_selected_lines =
        vec![selected("src/main.rs", 2, false, "+new"), selected("src/main.rs", 4, true, "-old")];

    let _ = update(&mut app, GitMessage::SelectDiffContextStageLines);
    assert_eq!(app.git.staged_lines_selected, vec![("src/main.rs".to_string(), 2)]);
    assert_eq!(app.git.staged_old_lines_selected, vec![("src/main.rs".to_string(), 4)]);

    let _ = update(&mut app, GitMessage::ClearDiffContextStageLines);
    assert!(app.git.staged_lines_selected.is_empty());
    assert!(app.git.staged_old_lines_selected.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn comment_submit_trims_comment_and_formats_one_based_range() {
    let mut app = test_app();
    app.git.git_diff_selected_range = Some(range("src/main.rs", 2, 4, false));
    app.git.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("src/main.rs", 2, 4, false),
        editor: iced::widget::text_editor::Content::with_text("  explain this  "),
    });

    let messages = collect_task_messages(update(&mut app, GitMessage::DiffCommentSubmit)).await;

    assert!(app.git.git_diff_comment_draft.is_none());
    assert!(app.git.git_diff_selected_range.is_none());
    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::AppendChat(text))]
            if text == "@src/main.rs:3-5 #explain this "
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn blank_comment_submit_emits_single_line_reference_without_comment_marker() {
    let mut app = test_app();
    app.git.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("src/lib.rs", 0, 0, true),
        editor: iced::widget::text_editor::Content::with_text("  \n "),
    });

    let messages = collect_task_messages(update(&mut app, GitMessage::DiffCommentSubmit)).await;

    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::AppendChat(text))]
            if text == "@src/lib.rs:1 "
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_selection_sorts_by_file_side_and_line_before_emitting() {
    let mut app = test_app();
    app.git.git_diff_selected_lines = vec![
        selected("b.rs", 1, false, "b-new"),
        selected("a.rs", 4, true, "a-old"),
        selected("a.rs", 7, false, "a-new-seven"),
        selected("a.rs", 2, false, "a-new-two"),
    ];

    let messages = collect_task_messages(update(&mut app, GitMessage::CopyDiffSelection)).await;

    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::CopyCode(text))]
            if text == "a-new-two\na-new-seven\na-old\nb-new"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_selection_comment_groups_files_and_diff_sides() {
    let mut app = test_app();
    app.git.git_diff_selected_lines = vec![
        selected("b.rs", 3, false, "+b"),
        selected("a.rs", 1, true, "-old"),
        selected("a.rs", 2, false, "+new"),
    ];

    let messages =
        collect_task_messages(update(&mut app, GitMessage::InsertDiffSelectionComment)).await;

    let expected = concat!(
        "Git Diff 评论\n",
        "文件: a.rs\n",
        "旧行: [1]\n",
        "新行: [2]\n",
        "```text\n",
        "+new\n",
        "-old\n",
        "```\n",
        "文件: b.rs\n",
        "新行: [3]\n",
        "```text\n",
        "+b\n",
        "```\n",
        "评论: \n",
    );
    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::AppendChat(text))]
            if text == expected
    ));
}

#[test]
fn hover_exit_only_clears_matching_line() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::DiffHoverEnter("a.rs".to_string(), 1, false));
    let _ = update(&mut app, GitMessage::DiffHoverExit("a.rs".to_string(), 2, false));
    assert_eq!(app.git.git_diff_hovered_line, Some(("a.rs".to_string(), 1, false)));

    let _ = update(&mut app, GitMessage::DiffHoverExit("a.rs".to_string(), 1, false));
    assert_eq!(app.git.git_diff_hovered_line, None);
}

#[test]
fn empty_copy_and_insert_selection_are_noops_but_close_context_menu() {
    let mut app = test_app();
    app.git.git_diff_context_menu = Some(crate::git_panel::diff_view::GitDiffContextMenuState {
        file: "a.rs".to_string(),
        line: 1,
        is_old: false,
        x: 0.0,
        y: 0.0,
    });

    let _ = update(&mut app, GitMessage::CopyDiffSelection);
    assert!(app.git.git_diff_context_menu.is_none());

    app.git.git_diff_context_menu = Some(crate::git_panel::diff_view::GitDiffContextMenuState {
        file: "a.rs".to_string(),
        line: 1,
        is_old: false,
        x: 0.0,
        y: 0.0,
    });
    let _ = update(&mut app, GitMessage::InsertDiffSelectionToChat);
    assert!(app.git.git_diff_context_menu.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_selection_to_chat_sorts_lines_before_emitting() {
    let mut app = test_app();
    app.git.git_diff_selected_lines = vec![
        selected("b.rs", 1, false, "b"),
        selected("a.rs", 2, true, "a-old"),
        selected("a.rs", 1, false, "a-new"),
    ];

    let messages =
        collect_task_messages(update(&mut app, GitMessage::InsertDiffSelectionToChat)).await;

    assert!(matches!(
        messages.as_slice(),
        [Message::Effect(Effect::AppendChat(text))]
            if text == "a-new\na-old\nb"
    ));
}

#[test]
fn empty_selection_comment_is_noop_and_closes_context_menu() {
    let mut app = test_app();
    app.git.git_diff_context_menu = Some(crate::git_panel::diff_view::GitDiffContextMenuState {
        file: "a.rs".to_string(),
        line: 1,
        is_old: false,
        x: 0.0,
        y: 0.0,
    });

    let _ = update(&mut app, GitMessage::InsertDiffSelectionComment);

    assert!(app.git.git_diff_context_menu.is_none());
}

#[test]
fn repo_mutation_messages_without_project_path_clear_transient_file_state() {
    let mut app = test_app();
    app.project_path = None;
    app.git.git_diff_file_menu = Some(GitDiffFileMenuState { file: "a.rs".to_string() });
    app.git.file_to_discard = Some("a.rs".to_string());

    let _ = update(&mut app, GitMessage::DiscardFile("a.rs".to_string()));
    let _ = update(&mut app, GitMessage::DiscardHunk("a.rs".to_string(), 0));
    let _ = update(&mut app, GitMessage::RevertLineDelete("a.rs".to_string(), 1));
    let _ = update(&mut app, GitMessage::RevertLineRestore("a.rs".to_string(), 1, 2));

    assert!(app.git.git_diff_file_menu.is_none());
    assert!(app.git.file_to_discard.is_none());
}

#[test]
fn discard_selection_without_repo_clears_context_menu_only() {
    let mut app = test_app();
    app.project_path = None;
    app.git.git_diff_context_menu = Some(crate::git_panel::diff_view::GitDiffContextMenuState {
        file: "a.rs".to_string(),
        line: 1,
        is_old: false,
        x: 0.0,
        y: 0.0,
    });
    app.git.git_diff_selected_lines = vec![selected("a.rs", 1, false, "+line")];

    let _ = update(&mut app, GitMessage::DiscardDiffSelection);

    assert!(app.git.git_diff_context_menu.is_none());
    assert_eq!(app.git.git_diff_selected_lines.len(), 1);
}

#[test]
fn discard_selection_with_repo_context_clears_selection_even_when_lines_are_not_changed() {
    let mut app = test_app();
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("a.rs"), "line\n").expect("write unchanged file");
    let mut index = repo.index().expect("repo index");
    index.add_path(std::path::Path::new("a.rs")).expect("stage file");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    let signature = git2::Signature::now("Test", "test@example.com").expect("signature");
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .expect("commit unchanged file");
    app.project_path = Some(temp.path().to_string_lossy().into_owned());
    assert!(app.view().repo_path.is_some());
    app.git.git_diff_selected_range = Some(range("a.rs", 1, 1, false));
    app.git.git_diff_selected_lines = vec![selected("a.rs", 1, false, "line")];

    let _ = update(&mut app, GitMessage::DiscardDiffSelection);

    assert!(app.git.git_diff_selected_lines.is_empty());
    assert!(app.git.git_diff_selected_range.is_none());
    assert_eq!(std::fs::read_to_string(temp.path().join("a.rs")).expect("read file"), "line\n");
}

#[test]
fn expand_all_sets_large_symmetric_context_window() {
    let mut app = test_app();

    let _ =
        update(&mut app, GitMessage::ExpandContext("a.rs".to_string(), 3, ExpandDirection::All));

    assert_eq!(
        app.git.context_expansions.get(&("a.rs".to_string(), 3)),
        Some(&(usize::MAX / 2, usize::MAX / 2))
    );
}

#[test]
fn toggle_expand_file_collapses_existing_file_and_expands_single_file() {
    let mut app = test_app();
    app.ensure_diff_file_expanded("existing.rs".to_string());

    let _ = update(&mut app, GitMessage::ToggleExpandFile("existing.rs".to_string()));
    assert!(app.git.expanded_files.is_empty());

    let _ = update(&mut app, GitMessage::ToggleExpandFile("next.rs".to_string()));
    assert_eq!(app.git.expanded_files, vec!["next.rs".to_string()]);
    assert!(app.is_diff_file_expanded("next.rs"));
}

#[test]
fn hunk_file_context_scroll_and_theme_state_are_updated() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::ToggleExpandHunk("a.rs".to_string(), 2));
    assert_eq!(app.git.expanded_hunks, vec![("a.rs".to_string(), 2)]);
    let _ = update(&mut app, GitMessage::ToggleExpandHunk("a.rs".to_string(), 2));
    assert!(app.git.expanded_hunks.is_empty());

    let _ =
        update(&mut app, GitMessage::ExpandContext("a.rs".to_string(), 1, ExpandDirection::Down));
    let _ = update(&mut app, GitMessage::ExpandContext("a.rs".to_string(), 1, ExpandDirection::Up));
    assert_eq!(app.git.context_expansions.get(&("a.rs".to_string(), 1)), Some(&(20, 20)));

    let _ = update(&mut app, GitMessage::DiffScrollChanged { offset_y: 1.5, viewport_h: -10.0 });
    assert_eq!(app.git.git_diff_scroll_offset_y, 1.0);
    assert_eq!(app.git.git_diff_scroll_viewport_h, 0.0);

    let _ = update(&mut app, GitMessage::DiffThemeSelected(DiffTheme::Monokai));
    assert_eq!(app.git.diff_theme, DiffTheme::Monokai);
}
