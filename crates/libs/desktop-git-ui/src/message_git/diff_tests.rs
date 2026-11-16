//! 原局部状态断言，使用仅含被测领域字段的中性夹具。
use crate::git_panel::diff_view::GitDiffCommentDraft;
use crate::git_panel::diff_view::GitDiffContextMenuState;
use crate::git_panel::diff_view::GitDiffLineRange;
use crate::message_git::messages::GitMessage;
use crate::state::State as Fixture;
use desktop_workspace::workspace_state::GitDiffSelectedLine;
fn test_app() -> Fixture {
    Fixture::default()
}
fn update(app: &mut Fixture, message: GitMessage) {
    let _ = crate::message_git::controller::diff::update(
        app,
        &Default::default(),
        &mut Vec::new(),
        message,
    );
}
fn selected(file: &str, line: usize, is_old: bool, text: &str) -> GitDiffSelectedLine {
    GitDiffSelectedLine { file: file.to_string(), line, is_old, text: text.to_string() }
}
fn range(file: &str, start: usize, end: usize, is_old: bool) -> GitDiffLineRange {
    GitDiffLineRange { file: file.to_string(), start, end, is_old }
}
#[test]
fn toggle_diff_line_selection_adds_and_removes_same_line() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::ToggleDiffLineSelection(
            "src/main.rs".to_string(),
            4,
            false,
            "+new".to_string(),
        ),
    );
    assert_eq!(app.git_diff_selected_lines.len(), 1);
    assert_eq!(app.git_diff_selected_lines[0].text, "+new");

    let _ = update(
        &mut app,
        GitMessage::ToggleDiffLineSelection(
            "src/main.rs".to_string(),
            4,
            false,
            "+new".to_string(),
        ),
    );
    assert_eq!(app.git_diff_selected_lines.len(), 0);
}
#[test]
fn toggle_diff_line_selection_clears_range_and_distinguishes_diff_side() {
    let mut app = test_app();
    app.git_diff_selected_range = Some(range("src/main.rs", 4, 6, false));
    app.git_diff_selected_lines = vec![selected("src/main.rs", 4, true, "-old")];

    let _ = update(
        &mut app,
        GitMessage::ToggleDiffLineSelection(
            "src/main.rs".to_string(),
            4,
            false,
            "+new".to_string(),
        ),
    );

    assert!(app.git_diff_selected_range.is_none());
    assert_eq!(app.git_diff_selected_lines.len(), 2);
    assert!(app.git_diff_selected_lines.iter().any(|line| line.is_old));
    assert!(app.git_diff_selected_lines.iter().any(|line| !line.is_old));
}
#[test]
fn drag_select_start_clears_transient_selection_state() {
    let mut app = test_app();
    app.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("old.rs", 1, 1, true),
        editor: iced::widget::text_editor::Content::new(),
    });
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "old.rs".to_string(),
        line: 1,
        is_old: true,
        x: 1.0,
        y: 2.0,
    });
    app.git_diff_selected_range = Some(range("old.rs", 1, 2, true));

    let _ = update(
        &mut app,
        GitMessage::DiffDragSelectStart("new.rs".to_string(), 7, false, "+line".to_string()),
    );

    assert!(app.git_diff_comment_draft.is_none());
    assert!(app.git_diff_context_menu.is_none());
    assert!(app.git_diff_selected_range.is_none());
    assert!(app.git_diff_dragging);
    assert_eq!(app.git_diff_drag_start_text.as_deref(), Some("+line"));
    let drag_range = app.git_diff_drag_range.as_ref().expect("drag range should be initialized");
    assert_eq!(drag_range.file, "new.rs");
    assert_eq!((drag_range.start, drag_range.end), (7, 7));
    assert!(!drag_range.is_old);
}
#[test]
fn drag_select_end_without_range_stops_dragging_and_consumes_start_text() {
    let mut app = test_app();
    app.git_diff_dragging = true;
    app.git_diff_drag_start_text = Some("unused".to_string());

    let _ = update(&mut app, GitMessage::DiffDragSelectEnd);

    assert!(!app.git_diff_dragging);
    assert!(app.git_diff_drag_start_text.is_none());
    assert!(app.git_diff_selected_range.is_none());
}
#[test]
fn drag_select_multiple_lines_normalizes_reverse_range_and_closes_comment() {
    let mut app = test_app();
    app.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: range("src/main.rs", 1, 1, false),
        editor: iced::widget::text_editor::Content::new(),
    });

    let _ = update(
        &mut app,
        GitMessage::DiffDragSelectStart("src/main.rs".to_string(), 8, false, "+eight".to_string()),
    );
    let _ = update(&mut app, GitMessage::DiffDragSelectHover("src/main.rs".to_string(), 3, false));
    let _ = update(&mut app, GitMessage::DiffDragSelectEnd);

    let selected_range = app.git_diff_selected_range.as_ref().expect("range should be selected");
    assert_eq!(selected_range.file, "src/main.rs");
    assert_eq!((selected_range.start, selected_range.end), (3, 8));
    assert!(!selected_range.is_old);
    assert!(app.git_diff_comment_draft.is_none());
    assert!(!app.git_diff_dragging);
    assert!(app.git_diff_drag_range.is_none());
}
#[test]
fn drag_select_single_line_toggles_selection_and_double_click_opens_comment_draft() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::DiffDragSelectStart("src/main.rs".to_string(), 10, false, "+line".to_string()),
    );
    let _ = update(&mut app, GitMessage::DiffDragSelectEnd);
    assert_eq!(app.git_diff_selected_lines.len(), 1);
    assert!(app.git_diff_selected_range.is_none());

    let _ = update(
        &mut app,
        GitMessage::DiffDragSelectStart("src/main.rs".to_string(), 10, false, "+line".to_string()),
    );
    let _ = update(&mut app, GitMessage::DiffDragSelectEnd);
    assert_eq!(app.git_diff_selected_lines.len(), 1);
    assert!(app.git_diff_comment_draft.is_some());
}
#[test]
fn drag_hover_only_updates_matching_active_range() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::DiffDragSelectStart("src/main.rs".to_string(), 2, false, "+a".to_string()),
    );
    let _ = update(&mut app, GitMessage::DiffDragSelectHover("src/lib.rs".to_string(), 9, false));
    assert_eq!(app.git_diff_drag_range.as_ref().map(|value| value.end), Some(2));

    let _ = update(&mut app, GitMessage::DiffDragSelectHover("src/main.rs".to_string(), 5, false));
    assert_eq!(app.git_diff_drag_range.as_ref().map(|value| value.end), Some(5));
}
