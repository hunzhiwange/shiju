use iced::Color;
use similar::DiffOp;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::{App, DiffTheme};

fn render_count(app: &App, group: &[DiffOp], old_lines: &[&str], new_lines: &[&str]) -> usize {
    let ctx = DiffRenderCtx::new(app);
    let hover = Color::from_rgba8(255, 210, 0, 0.22);

    super::render_hunk_ops(
        app,
        &ctx,
        "src/lib.rs",
        group,
        7,
        old_lines,
        new_lines,
        Lang::Rust,
        DiffTheme::GitHub,
        Color::WHITE,
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
        hover,
        0.22,
        hover,
        false,
        true,
    )
    .len()
}

#[test]
fn task_706_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("tests.rs"));
}

#[test]
fn render_hunk_ops_zero_length_variants_produce_no_rows_in_either_view() {
    let mut app = App::new().0;
    let group = [
        DiffOp::Equal { old_index: 0, new_index: 0, len: 0 },
        DiffOp::Delete { old_index: 0, old_len: 0, new_index: 0 },
        DiffOp::Insert { old_index: 0, new_index: 0, new_len: 0 },
        DiffOp::Replace { old_index: 0, old_len: 0, new_index: 0, new_len: 0 },
    ];

    app.merge_view = true;
    assert_eq!(render_count(&app, &group, &[], &[]), 0);

    app.merge_view = false;
    assert_eq!(render_count(&app, &group, &[], &[]), 0);
}

#[test]
fn render_hunk_ops_preserves_child_row_counts_across_view_modes() {
    let mut app = App::new().0;
    let group = [
        DiffOp::Equal { old_index: 4, new_index: 4, len: 2 },
        DiffOp::Delete { old_index: 6, old_len: 2, new_index: 6 },
        DiffOp::Insert { old_index: 8, new_index: 6, new_len: 3 },
        DiffOp::Replace { old_index: 8, old_len: 2, new_index: 9, new_len: 4 },
    ];

    app.merge_view = true;
    assert_eq!(render_count(&app, &group, &[], &[]), 13);

    app.merge_view = false;
    assert_eq!(render_count(&app, &group, &[], &[]), 11);
}
