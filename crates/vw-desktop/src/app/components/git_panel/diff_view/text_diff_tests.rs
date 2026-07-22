use iced::Color;

use crate::app::state::{GitDiffLineRange, GitDiffSelectedLine};
use crate::app::{DiffTheme, Message};

#[test]
fn task_716_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("text_diff_tests.rs"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn colors() -> (Color, Color, Color, Color, Color) {
    (
        Color::from_rgba8(255, 255, 255, 1.0),
        Color::from_rgba8(220, 255, 220, 1.0),
        Color::from_rgba8(160, 245, 160, 1.0),
        Color::from_rgba8(255, 220, 220, 1.0),
        Color::from_rgba8(245, 160, 160, 1.0),
    )
}

#[test]
fn custom_text_diff_builds_merge_view_for_equal_insert_and_delete_lines() {
    let mut app = app();
    app.merge_view = true;
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 1, false));
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 0, end: 1, is_old: false });
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 1,
        is_old: false,
        text: "new".to_string(),
    });
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "Example".to_string(),
        Some("src/lib.rs".to_string()),
        "same\nold\n".to_string(),
        "same\nnew\n".to_string(),
        Some(Message::None),
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn custom_text_diff_builds_split_view_with_context_menu_wrapping() {
    let mut app = app();
    app.merge_view = false;
    app.git_diff_hovered_line = Some(("Example".to_string(), 1, true));
    app.staged_old_lines_selected.push(("Example".to_string(), 1));
    app.staged_lines_selected.push(("Example".to_string(), 1));
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "Example".to_string(),
        None,
        "a\nb\nc\n".to_string(),
        "a\nB\nc\nnew\n".to_string(),
        None,
        DiffTheme::Monokai,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn custom_text_diff_applies_equal_line_hover_in_merge_and_split_views() {
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let mut merge_app = app();
    merge_app.merge_view = true;
    merge_app.git_diff_hovered_line = Some(("src/main.rs".to_string(), 0, false));
    let merge = super::text_diff::view_custom_text_diff(
        &merge_app,
        "main".to_string(),
        Some("src/main.rs".to_string()),
        "same\nold\n".to_string(),
        "same\nnew\n".to_string(),
        None,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
    std::hint::black_box(merge);

    let mut split_app = app();
    split_app.merge_view = false;
    split_app.git_diff_hovered_line = Some(("main".to_string(), 0, false));
    let split = super::text_diff::view_custom_text_diff(
        &split_app,
        "main".to_string(),
        None,
        "same\nold\n".to_string(),
        "same\nnew\n".to_string(),
        None,
        DiffTheme::Monokai,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
    std::hint::black_box(split);
}

#[test]
fn custom_text_diff_handles_empty_old_or_new_content() {
    let app = app();
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _added = super::text_diff::view_custom_text_diff(
        &app,
        "added.txt".to_string(),
        Some("added.txt".to_string()),
        String::new(),
        "one\ntwo\n".to_string(),
        None,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );

    let _deleted = super::text_diff::view_custom_text_diff(
        &app,
        "deleted.txt".to_string(),
        Some("deleted.txt".to_string()),
        "one\ntwo\n".to_string(),
        String::new(),
        None,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn custom_text_diff_handles_both_inputs_empty() {
    let app = app();
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "empty.txt".to_string(),
        Some("empty.txt".to_string()),
        String::new(),
        String::new(),
        Some(Message::None),
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn split_view_with_diff_line_selection() {
    let mut app = app();
    app.merge_view = false;
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: "test.rs".to_string(),
        line: 0,
        is_old: true,
        text: "a".to_string(),
    });
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: "test.rs".to_string(),
        line: 0,
        is_old: false,
        text: "A".to_string(),
    });
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "test.rs".to_string(),
        None,
        "a\nb\n".to_string(),
        "A\nb\n".to_string(),
        Some(Message::None),
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn handles_content_without_trailing_newlines() {
    let app = app();
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "no_newline.txt".to_string(),
        None,
        "line one".to_string(),
        "line two".to_string(),
        None,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}

#[test]
fn handles_unicode_content() {
    let app = app();
    let (bg, add_line, add_word, del_line, del_word) = colors();

    let _element = super::text_diff::view_custom_text_diff(
        &app,
        "unicode.txt".to_string(),
        None,
        "你好\n世界\n".to_string(),
        "你好\n🌍\n".to_string(),
        None,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
    );
}
