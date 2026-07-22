use super::super::DiffRenderCtx;
use super::added_deleted::{render_added_lines, render_deleted_lines};
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::{GitDiffContextMenuState, GitDiffSelectedLine};
use crate::app::{App, DiffTheme, Message};
use iced::advanced::widget::Tree;
use iced::{Color, Element};

fn app() -> App {
    App::new().0
}

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}

fn widget_shape(column: iced::widget::Column<'static, Message>) -> (usize, usize) {
    let element: Element<'static, Message> = column.into();
    let rows = element.as_widget().children();
    let node_count = rows.iter().map(tree_node_count).sum();
    (rows.len(), node_count)
}

fn active_app(file: &str, merge_view: bool, is_old: bool) -> App {
    let mut app = app();
    app.merge_view = merge_view;
    if is_old {
        app.staged_old_lines_selected.push((file.to_string(), 0));
    } else {
        app.staged_lines_selected.push((file.to_string(), 0));
    }
    app.git_diff_hovered_line = Some((file.to_string(), 0, is_old));
    app.git_diff_selected_lines = vec![GitDiffSelectedLine {
        file: file.to_string(),
        line: 1,
        is_old,
        text: "selected".to_string(),
    }];
    app
}

#[test]
fn task_702_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("added_deleted_tests.rs"));
}

#[test]
fn empty_added_and_deleted_inputs_render_no_rows() {
    let app = app();
    let ctx = DiffRenderCtx::new(&app);

    let added = render_added_lines(
        &app,
        &ctx,
        "src/empty.rs",
        &[],
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );
    let deleted = render_deleted_lines(
        &app,
        &ctx,
        "src/empty.rs",
        &[],
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );

    assert_eq!(widget_shape(added), (0, 0));
    assert_eq!(widget_shape(deleted), (0, 0));
}

#[test]
fn added_lines_preserve_row_count_and_split_view_has_distinct_structure() {
    const FILE: &str = "src/lib.rs";
    const LINES: &[&str] = &["", "let added = true;", "新增🙂"];

    let merge_app = active_app(FILE, true, false);
    let merge_ctx = DiffRenderCtx::new(&merge_app);
    let merge = render_added_lines(
        &merge_app,
        &merge_ctx,
        FILE,
        LINES,
        Lang::Rust,
        DiffTheme::GitHub,
        Color::from_rgb(0.8, 1.0, 0.8),
        Color::from_rgb(0.5, 0.9, 0.5),
        Color::BLACK,
        0.2,
        Color::WHITE,
        true,
        true,
    );

    let split_app = active_app(FILE, false, false);
    let split_ctx = DiffRenderCtx::new(&split_app);
    let split = render_added_lines(
        &split_app,
        &split_ctx,
        FILE,
        LINES,
        Lang::Rust,
        DiffTheme::Monokai,
        Color::from_rgb(0.8, 1.0, 0.8),
        Color::from_rgb(0.5, 0.9, 0.5),
        Color::BLACK,
        0.2,
        Color::WHITE,
        true,
        true,
    );

    let (merge_rows, merge_nodes) = widget_shape(merge);
    let (split_rows, split_nodes) = widget_shape(split);
    assert_eq!(merge_rows, LINES.len());
    assert_eq!(split_rows, LINES.len());
    assert!(split_nodes > merge_nodes);
}

#[test]
fn deleted_lines_preserve_row_count_and_split_view_has_distinct_structure() {
    const FILE: &str = "src/lib.rs";
    const LINES: &[&str] = &["", "let deleted = true;", "删除🙂"];

    let merge_app = active_app(FILE, true, true);
    let merge_ctx = DiffRenderCtx::new(&merge_app);
    let merge = render_deleted_lines(
        &merge_app,
        &merge_ctx,
        FILE,
        LINES,
        Lang::Rust,
        DiffTheme::GitHub,
        Color::from_rgb(1.0, 0.8, 0.8),
        Color::BLACK,
        0.2,
        Color::WHITE,
        true,
        true,
    );

    let split_app = active_app(FILE, false, true);
    let split_ctx = DiffRenderCtx::new(&split_app);
    let split = render_deleted_lines(
        &split_app,
        &split_ctx,
        FILE,
        LINES,
        Lang::Rust,
        DiffTheme::Monokai,
        Color::from_rgb(1.0, 0.8, 0.8),
        Color::BLACK,
        0.2,
        Color::WHITE,
        true,
        true,
    );

    let (merge_rows, merge_nodes) = widget_shape(merge);
    let (split_rows, split_nodes) = widget_shape(split);
    assert_eq!(merge_rows, LINES.len());
    assert_eq!(split_rows, LINES.len());
    assert!(split_nodes > merge_nodes);
}

#[test]
fn file_level_staging_preserves_added_and_deleted_rows() {
    const FILE: &str = "src/staged.rs";
    let mut app = app();
    app.merge_view = false;
    app.staged_files_selected.push(FILE.to_string());
    let ctx = DiffRenderCtx::new(&app);

    let added = render_added_lines(
        &app,
        &ctx,
        FILE,
        &["staged addition"],
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );
    let deleted = render_deleted_lines(
        &app,
        &ctx,
        FILE,
        &["staged deletion"],
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );

    assert_eq!(widget_shape(added).0, 1);
    assert_eq!(widget_shape(deleted).0, 1);
}

#[test]
fn matching_context_menu_adds_menu_nodes_for_added_and_deleted_rows() {
    const FILE: &str = "src/menu.rs";
    let closed_app = app();
    let closed_ctx = DiffRenderCtx::new(&closed_app);
    let closed_added = render_added_lines(
        &closed_app,
        &closed_ctx,
        FILE,
        &["added"],
        Lang::Rust,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );
    let closed_deleted = render_deleted_lines(
        &closed_app,
        &closed_ctx,
        FILE,
        &["deleted"],
        Lang::Rust,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );

    let mut added_app = app();
    added_app.merge_view = true;
    added_app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: FILE.to_string(),
        line: 0,
        is_old: false,
        x: 12.0,
        y: 24.0,
    });
    let added_ctx = DiffRenderCtx::new(&added_app);
    let opened_added = render_added_lines(
        &added_app,
        &added_ctx,
        FILE,
        &["added"],
        Lang::Rust,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );

    let mut deleted_app = app();
    deleted_app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: FILE.to_string(),
        line: 0,
        is_old: true,
        x: 32.0,
        y: 64.0,
    });
    let deleted_ctx = DiffRenderCtx::new(&deleted_app);
    let opened_deleted = render_deleted_lines(
        &deleted_app,
        &deleted_ctx,
        FILE,
        &["deleted"],
        Lang::Rust,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::BLACK,
        0.0,
        Color::WHITE,
        false,
        false,
    );

    assert!(widget_shape(opened_added).1 > widget_shape(closed_added).1);
    assert!(widget_shape(opened_deleted).1 > widget_shape(closed_deleted).1);
}
