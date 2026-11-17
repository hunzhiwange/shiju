use iced::widget::text_editor;
/// Git message domain tests; host routing and cross-domain effects stay in vw-desktop.
use crate::git_panel::diff_view::{GitDiffContextMenuState, GitDiffLineRange};
use crate::git_panel::types::ConventionalCommitType;
use crate::message_git::shared as domain;
use desktop_workspace::workspace_state::GitDiffSelectedLine;
use std::collections::{HashMap, HashSet};

#[test]
fn build_selected_commit_request_formats_conventional_subject_and_body() {
    let git_commit_type: Option<ConventionalCommitType> = Some(ConventionalCommitType::Fix);
    let git_commit_scope: String = "desktop".to_string();
    let git_commit_message: String = " refresh git state ".to_string();
    let git_commit_description: String = " body line ".to_string();
    let staged_files_selected: Vec<String> = vec!["src/main.rs".to_string()];
    let staged_hunks_selected: Vec<(String, usize)> = vec![("src/lib.rs".to_string(), 2)];
    let mut staged_lines_selected: Vec<(String, usize)> = vec![("src/lib.rs".to_string(), 4)];
    let mut staged_old_lines_selected: Vec<(String, usize)> = vec![("src/lib.rs".to_string(), 3)];

    let request = domain::build_commit_request(
        &git_commit_message,
        &git_commit_description,
        &git_commit_type,
        &git_commit_scope,
        &staged_files_selected,
        &staged_hunks_selected,
        &staged_lines_selected,
        &staged_old_lines_selected,
        false,
    )
    .expect("message is valid");

    assert_eq!(request.message, "fix(desktop): refresh git state\n\nbody line");
    assert!(!request.stage_all);
    assert_eq!(request.selected_files, vec!["src/main.rs"]);
    assert_eq!(request.selected_hunks, vec![("src/lib.rs".to_string(), 2)]);
    assert_eq!(request.selected_lines, vec![("src/lib.rs".to_string(), 4)]);
    assert_eq!(request.selected_old_lines, vec![("src/lib.rs".to_string(), 3)]);
}

#[test]
fn build_all_commit_request_enables_atomic_stage_all() {
    let git_commit_description: String = Default::default();
    let git_commit_scope: String = Default::default();
    let git_commit_type: Option<ConventionalCommitType> = Some(ConventionalCommitType::Feat);
    let staged_files_selected: Vec<String> = Default::default();
    let staged_hunks_selected: Vec<(String, usize)> = Default::default();
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();
    let git_commit_message: String = "ship everything".to_string();

    let request = domain::build_commit_request(
        &git_commit_message,
        &git_commit_description,
        &git_commit_type,
        &git_commit_scope,
        &staged_files_selected,
        &staged_hunks_selected,
        &staged_lines_selected,
        &staged_old_lines_selected,
        true,
    )
    .expect("message is valid");

    assert!(request.stage_all);
    assert_eq!(request.message, "feat: ship everything");
}

#[test]
fn build_selected_commit_request_rejects_blank_message() {
    let git_commit_scope: String = Default::default();
    let staged_files_selected: Vec<String> = Default::default();
    let staged_hunks_selected: Vec<(String, usize)> = Default::default();
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();
    let git_commit_type: Option<ConventionalCommitType> = None;
    let git_commit_message: String = "   ".to_string();
    let git_commit_description: String = "\n\t".to_string();

    let error = domain::build_commit_request(
        &git_commit_message,
        &git_commit_description,
        &git_commit_type,
        &git_commit_scope,
        &staged_files_selected,
        &staged_hunks_selected,
        &staged_lines_selected,
        &staged_old_lines_selected,
        false,
    )
    .expect_err("blank is invalid");

    assert_eq!(error, "提交消息不能为空");
}

#[test]
fn build_selected_commit_request_formats_subject_without_scope_or_type() {
    let mut git_commit_description: String = Default::default();
    let staged_files_selected: Vec<String> = Default::default();
    let staged_hunks_selected: Vec<(String, usize)> = Default::default();
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();
    let mut git_commit_type: Option<ConventionalCommitType> = Some(ConventionalCommitType::Feat);
    let git_commit_scope: String = "  ".to_string();
    let git_commit_message: String = " add tests ".to_string();

    let conventional = domain::build_commit_request(
        &git_commit_message,
        &git_commit_description,
        &git_commit_type,
        &git_commit_scope,
        &staged_files_selected,
        &staged_hunks_selected,
        &staged_lines_selected,
        &staged_old_lines_selected,
        false,
    )
    .expect("summary is valid");

    assert_eq!(conventional.message, "feat: add tests");

    git_commit_type = None;
    git_commit_description = " details ".to_string();

    let plain = domain::build_commit_request(
        &git_commit_message,
        &git_commit_description,
        &git_commit_type,
        &git_commit_scope,
        &staged_files_selected,
        &staged_hunks_selected,
        &staged_lines_selected,
        &staged_old_lines_selected,
        false,
    )
    .expect("body is valid");

    assert_eq!(plain.message, "add tests\n\ndetails");
}

#[test]
fn reset_commit_form_state_clears_selection_and_restores_defaults() {
    let mut context_expansions: HashMap<(String, usize), (usize, usize)> = Default::default();
    let mut expanded_files: Vec<String> = Default::default();
    let mut expanded_files_set: HashSet<String> = Default::default();
    let mut expanded_hunks: Vec<(String, usize)> = Default::default();
    let mut staged_files_selected: Vec<String> = vec!["a.rs".to_string()];
    let mut staged_hunks_selected: Vec<(String, usize)> = vec![("a.rs".to_string(), 1)];
    let mut staged_lines_selected: Vec<(String, usize)> = vec![("a.rs".to_string(), 2)];
    let mut staged_old_lines_selected: Vec<(String, usize)> = vec![("a.rs".to_string(), 3)];
    expanded_hunks.push(("a.rs".to_string(), 1));
    context_expansions.insert(("a.rs".to_string(), 0), (1, 2));
    let mut git_commit_message: String = "msg".to_string();
    let mut git_commit_type: Option<ConventionalCommitType> = Some(ConventionalCommitType::Docs);
    let mut git_commit_scope: String = "scope".to_string();
    let mut git_commit_description: String = "body".to_string();
    let mut git_commit_description_editor: text_editor::Content =
        iced::widget::text_editor::Content::with_text("body");

    domain::reset_commit_form_state(
        &mut staged_files_selected,
        &mut staged_hunks_selected,
        &mut staged_lines_selected,
        &mut staged_old_lines_selected,
        &mut expanded_files,
        &mut expanded_files_set,
        &mut expanded_hunks,
        &mut context_expansions,
        &mut git_commit_message,
        &mut git_commit_type,
        &mut git_commit_scope,
        &mut git_commit_description,
        &mut git_commit_description_editor,
    );

    assert!(staged_files_selected.is_empty());
    assert!(staged_hunks_selected.is_empty());
    assert!(staged_lines_selected.is_empty());
    assert!(staged_old_lines_selected.is_empty());
    assert!(expanded_files.is_empty());
    assert!(expanded_hunks.is_empty());
    assert!(context_expansions.is_empty());
    assert!(git_commit_message.is_empty());
    assert_eq!(git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(git_commit_scope.is_empty());
    assert!(git_commit_description.is_empty());
    assert!(git_commit_description_editor.text().is_empty());
}

#[test]
fn diff_context_target_range_prefers_menu_inside_selected_range() {
    let git_diff_selected_lines: Vec<GitDiffSelectedLine> = Default::default();
    let git_diff_selected_range: Option<GitDiffLineRange> =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 8, end: 4, is_old: false });
    let git_diff_context_menu: Option<GitDiffContextMenuState> = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 6,
        is_old: false,
        x: 1.0,
        y: 2.0,
    });

    let range = domain::diff_context_target_range(
        &git_diff_context_menu,
        &git_diff_selected_range,
        &git_diff_selected_lines,
    )
    .expect("range should resolve");

    assert_eq!(range.file, "src/lib.rs");
    assert_eq!(range.start, 4);
    assert_eq!(range.end, 8);
    assert!(!range.is_old);
}

#[test]
fn diff_context_target_range_falls_back_to_menu_line() {
    let git_diff_selected_lines: Vec<GitDiffSelectedLine> = Default::default();
    let git_diff_selected_range: Option<GitDiffLineRange> =
        Some(GitDiffLineRange { file: "other.rs".to_string(), start: 1, end: 3, is_old: false });
    let git_diff_context_menu: Option<GitDiffContextMenuState> = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 6,
        is_old: true,
        x: 1.0,
        y: 2.0,
    });

    let range = domain::diff_context_target_range(
        &git_diff_context_menu,
        &git_diff_selected_range,
        &git_diff_selected_lines,
    )
    .expect("range should resolve");

    assert_eq!(range.file, "src/lib.rs");
    assert_eq!(range.start, 6);
    assert_eq!(range.end, 6);
    assert!(range.is_old);
}

#[test]
fn diff_context_target_range_uses_selection_then_selected_line_then_none() {
    let git_diff_selected_lines = Default::default();
    let git_diff_context_menu: Option<GitDiffContextMenuState> = Default::default();
    let mut git_diff_selected_range: Option<GitDiffLineRange> =
        Some(GitDiffLineRange { file: "range.rs".to_string(), start: 7, end: 3, is_old: false });

    let range = domain::diff_context_target_range(
        &git_diff_context_menu,
        &git_diff_selected_range,
        &git_diff_selected_lines,
    )
    .expect("range should resolve");
    assert_eq!((range.file.as_str(), range.start, range.end), ("range.rs", 3, 7));

    git_diff_selected_range = None;
    let mut git_diff_selected_lines: Vec<GitDiffSelectedLine> = vec![GitDiffSelectedLine {
        file: "line.rs".to_string(),
        line: 9,
        is_old: true,
        text: "removed".to_string(),
    }];

    let line = domain::diff_context_target_range(
        &git_diff_context_menu,
        &git_diff_selected_range,
        &git_diff_selected_lines,
    )
    .expect("line should resolve");
    assert_eq!((line.file.as_str(), line.start, line.end, line.is_old), ("line.rs", 9, 9, true));

    git_diff_selected_lines.clear();
    assert!(
        domain::diff_context_target_range(
            &git_diff_context_menu,
            &git_diff_selected_range,
            &git_diff_selected_lines
        )
        .is_none()
    );
}

#[test]
fn diff_context_target_stage_lines_deduplicates_selected_lines() {
    let git_diff_context_menu: Option<GitDiffContextMenuState> = Default::default();
    let git_diff_selected_range: Option<GitDiffLineRange> = Default::default();
    let git_diff_selected_lines: Vec<GitDiffSelectedLine> = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 5,
            is_old: false,
            text: "a".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 5,
            is_old: false,
            text: "a".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 2,
            is_old: true,
            text: "b".to_string(),
        },
    ];

    let lines = domain::diff_context_target_stage_lines(
        &git_diff_context_menu,
        &git_diff_selected_range,
        &git_diff_selected_lines,
    );

    assert_eq!(
        lines,
        vec![("src/lib.rs".to_string(), 2, true), ("src/lib.rs".to_string(), 5, false)]
    );
}

#[test]
fn diff_context_target_stage_lines_uses_range_containing_menu_line() {
    let git_diff_selected_lines: Vec<GitDiffSelectedLine> = Default::default();
    let git_diff_selected_range: Option<GitDiffLineRange> =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 5, end: 3, is_old: true });
    let git_diff_context_menu: Option<GitDiffContextMenuState> = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 4,
        is_old: true,
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(
        domain::diff_context_target_stage_lines(
            &git_diff_context_menu,
            &git_diff_selected_range,
            &git_diff_selected_lines
        ),
        vec![
            ("src/lib.rs".to_string(), 3, true),
            ("src/lib.rs".to_string(), 4, true),
            ("src/lib.rs".to_string(), 5, true),
        ]
    );
}

#[test]
fn diff_context_target_stage_lines_filters_menu_side_or_uses_menu_only() {
    let git_diff_selected_range: Option<GitDiffLineRange> = Default::default();
    let git_diff_selected_lines: Vec<GitDiffSelectedLine> = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 6,
            is_old: false,
            text: "selected".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: false,
            text: "selected".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: true,
            text: "other side".to_string(),
        },
    ];
    let mut git_diff_context_menu: Option<GitDiffContextMenuState> =
        Some(GitDiffContextMenuState {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: false,
            x: 0.0,
            y: 0.0,
        });

    assert_eq!(
        domain::diff_context_target_stage_lines(
            &git_diff_context_menu,
            &git_diff_selected_range,
            &git_diff_selected_lines
        ),
        vec![("src/lib.rs".to_string(), 4, false), ("src/lib.rs".to_string(), 6, false)]
    );

    git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "other.rs".to_string(),
        line: 2,
        is_old: true,
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(
        domain::diff_context_target_stage_lines(
            &git_diff_context_menu,
            &git_diff_selected_range,
            &git_diff_selected_lines
        ),
        vec![("other.rs".to_string(), 2, true)]
    );
}

#[test]
fn extend_and_clear_stage_selection_for_lines_are_idempotent() {
    let mut staged_lines_selected: Vec<(String, usize)> = Default::default();
    let mut staged_old_lines_selected: Vec<(String, usize)> = Default::default();
    let lines = vec![
        ("src/lib.rs".to_string(), 2, false),
        ("src/lib.rs".to_string(), 2, false),
        ("src/lib.rs".to_string(), 1, true),
    ];

    domain::extend_stage_selection_for_lines(
        &mut staged_old_lines_selected,
        &mut staged_lines_selected,
        &lines,
    );

    assert_eq!(staged_lines_selected, vec![("src/lib.rs".to_string(), 2)]);
    assert_eq!(staged_old_lines_selected, vec![("src/lib.rs".to_string(), 1)]);

    domain::clear_stage_selection_for_lines(
        &mut staged_old_lines_selected,
        &mut staged_lines_selected,
        &[("src/lib.rs".to_string(), 2, false)],
    );

    assert!(staged_lines_selected.is_empty());
    assert_eq!(staged_old_lines_selected, vec![("src/lib.rs".to_string(), 1)]);
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn normalize_stage_line_selections_sorts_and_deduplicates_both_sides() {
    let mut staged_lines_selected: Vec<(String, usize)> =
        vec![("b.rs".to_string(), 3), ("a.rs".to_string(), 2), ("a.rs".to_string(), 2)];
    let mut staged_old_lines_selected: Vec<(String, usize)> =
        vec![("a.rs".to_string(), 4), ("a.rs".to_string(), 1)];

    domain::normalize_stage_line_selections(
        &mut staged_lines_selected,
        &mut staged_old_lines_selected,
    );

    assert_eq!(staged_lines_selected, vec![("a.rs".to_string(), 2), ("b.rs".to_string(), 3)]);
    assert_eq!(staged_old_lines_selected, vec![("a.rs".to_string(), 1), ("a.rs".to_string(), 4)]);
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn replace_stage_selection_for_file_lines_replaces_only_target_file() {
    let mut staged_lines_selected: Vec<(String, usize)> =
        vec![("target.rs".to_string(), 1), ("keep.rs".to_string(), 8)];
    let mut staged_old_lines_selected: Vec<(String, usize)> =
        vec![("target.rs".to_string(), 2), ("keep.rs".to_string(), 7)];
    let old_changed = HashSet::from([3, 5]);
    let new_changed = HashSet::from([4, 6]);

    domain::replace_stage_selection_for_file_lines(
        &mut staged_lines_selected,
        &mut staged_old_lines_selected,
        "target.rs",
        &old_changed,
        &new_changed,
    );

    assert!(staged_lines_selected.contains(&("keep.rs".to_string(), 8)));
    assert!(staged_lines_selected.contains(&("target.rs".to_string(), 4)));
    assert!(staged_lines_selected.contains(&("target.rs".to_string(), 6)));
    assert_eq!(staged_lines_selected.len(), 3);
    assert!(staged_old_lines_selected.contains(&("keep.rs".to_string(), 7)));
    assert!(staged_old_lines_selected.contains(&("target.rs".to_string(), 3)));
    assert!(staged_old_lines_selected.contains(&("target.rs".to_string(), 5)));
    assert_eq!(staged_old_lines_selected.len(), 3);
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn invert_stage_selection_for_file_lines_toggles_changed_lines_only() {
    let mut staged_lines_selected: Vec<(String, usize)> =
        vec![("target.rs".to_string(), 1), ("keep.rs".to_string(), 8)];
    let mut staged_old_lines_selected: Vec<(String, usize)> =
        vec![("target.rs".to_string(), 2), ("keep.rs".to_string(), 7)];
    let old_changed = HashSet::from([2, 3]);
    let new_changed = HashSet::from([1, 4]);

    domain::invert_stage_selection_for_file_lines(
        &mut staged_lines_selected,
        &mut staged_old_lines_selected,
        "target.rs",
        &old_changed,
        &new_changed,
    );

    assert!(!staged_lines_selected.contains(&("target.rs".to_string(), 1)));
    assert!(staged_lines_selected.contains(&("target.rs".to_string(), 4)));
    assert!(staged_lines_selected.contains(&("keep.rs".to_string(), 8)));
    assert!(!staged_old_lines_selected.contains(&("target.rs".to_string(), 2)));
    assert!(staged_old_lines_selected.contains(&("target.rs".to_string(), 3)));
    assert!(staged_old_lines_selected.contains(&("keep.rs".to_string(), 7)));
}
