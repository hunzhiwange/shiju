//! 验证差异块操作分派覆盖所有变体并容忍空分组和缺失行。
//! 调度层只组合子渲染结果，不应因任一侧内容不足而改变操作序列。

use iced::Color;
use similar::DiffOp;

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::{App, DiffTheme};

fn palette() -> (Color, Color, Color, Color, Color, Color) {
    (
        Color::WHITE,
        Color::from_rgba8(0, 255, 0, 0.10),
        Color::from_rgba8(0, 255, 0, 0.25),
        Color::from_rgba8(255, 0, 0, 0.10),
        Color::from_rgba8(255, 0, 0, 0.25),
        Color::from_rgba8(255, 210, 0, 0.22),
    )
}

#[test]
fn render_hunk_ops_dispatches_all_diff_op_variants() {
    let app = App::new().0;
    let ctx = DiffRenderCtx::new(&app);
    let (bg, add_line, add_word, del_line, del_word, hover) = palette();
    let group = vec![
        DiffOp::Equal { old_index: 0, new_index: 0, len: 1 },
        DiffOp::Delete { old_index: 1, old_len: 1, new_index: 1 },
        DiffOp::Insert { old_index: 2, new_index: 1, new_len: 1 },
        DiffOp::Replace { old_index: 2, old_len: 1, new_index: 2, new_len: 2 },
    ];

    let rows = super::render_hunk_ops(
        &app,
        &ctx,
        "src/lib.rs",
        &group,
        0,
        &["same", "old", "old replace"],
        &["same", "new", "new replace", "extra"],
        Lang::Rust,
        DiffTheme::GitHub,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
        hover,
        0.22,
        hover,
        false,
        true,
    );

    assert_eq!(rows.len(), 6);
}

#[test]
fn render_hunk_ops_tolerates_empty_groups_and_missing_lines() {
    let app = App::new().0;
    let ctx = DiffRenderCtx::new(&app);
    let (bg, add_line, add_word, del_line, del_word, hover) = palette();

    let rows = super::render_hunk_ops(
        &app,
        &ctx,
        "src/lib.rs",
        &[DiffOp::Equal { old_index: 5, new_index: 5, len: 2 }],
        0,
        &[],
        &[],
        Lang::Other,
        DiffTheme::Monokai,
        bg,
        add_line,
        add_word,
        del_line,
        del_word,
        hover,
        0.22,
        hover,
        true,
        false,
    );

    assert_eq!(rows.len(), 2);
    assert!(
        super::render_hunk_ops(
            &app,
            &ctx,
            "src/lib.rs",
            &[],
            0,
            &[],
            &[],
            Lang::Other,
            DiffTheme::Monokai,
            bg,
            add_line,
            add_word,
            del_line,
            del_word,
            hover,
            0.22,
            hover,
            true,
            false,
        )
        .is_empty()
    );
}
