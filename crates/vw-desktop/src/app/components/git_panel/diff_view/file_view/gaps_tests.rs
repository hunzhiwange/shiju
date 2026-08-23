//! 验证差异间隙的上下文展开、分栏边界、越界容错与主题样式。
//! 仅匹配文件与间隙标识的展开状态生效，短范围和默认上下文边界不得多出控制行。

use super::super::DiffRenderCtx;
use super::gaps::{gap_expand_container_style, gap_summary_text_style, render_gap};
use super::start_end::GapRange;
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::GitDiffSelectedLine;
use crate::app::{App, DiffTheme};
use iced::{Background, Color, Theme};

fn app() -> App {
    App::new().0
}

#[test]
fn task_703_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("gaps_tests.rs"));
}

#[test]
fn render_gap_returns_empty_for_zero_length_range() {
    let app = app();
    let ctx = DiffRenderCtx::new(&app);
    let elements = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &["a", "b"],
        GapRange { start_old: 1, end_old: 1, start_new: 1, gap_id: 0 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );

    assert!(elements.is_empty());
}

#[test]
fn render_gap_covers_full_partial_merge_and_split_paths() {
    let mut app = app();
    app.merge_view = true;
    let old_lines = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    app.context_expansions.insert(("src/lib.rs".to_string(), 7), (4, 4));
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 2, true));
    let ctx = DiffRenderCtx::new(&app);

    let full = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 1, end_old: 5, start_new: 1, gap_id: 7 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::from_rgb(0.95, 0.95, 0.95),
        Color::WHITE,
        true,
    );
    assert_eq!(full.len(), 4);

    app.context_expansions.clear();
    let ctx = DiffRenderCtx::new(&app);
    let partial = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 10, start_new: 0, gap_id: 8 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(partial.len(), 7);

    app.merge_view = false;
    let ctx = DiffRenderCtx::new(&app);
    let split = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 10, start_new: 0, gap_id: 9 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(split.len(), 7);
}

#[test]
fn gap_styles_cover_light_and_dark_palettes() {
    let light_text = gap_summary_text_style(&Theme::Light).color.expect("light color");
    let dark_text = gap_summary_text_style(&Theme::Dark).color.expect("dark color");
    assert_eq!(
        light_text,
        Theme::Light.extended_palette().background.strong.text.scale_alpha(0.92)
    );
    assert_eq!(dark_text, Theme::Dark.palette().text.scale_alpha(0.98));

    let light_container = gap_expand_container_style(&Theme::Light);
    let dark_container = gap_expand_container_style(&Theme::Dark);
    assert_eq!(light_container.border.width, 1.0);
    assert_eq!(dark_container.border.width, 1.0);
    assert_eq!(
        light_container.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.10)
    );
    assert_eq!(
        dark_container.border.color,
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.24)
    );

    let Some(Background::Color(light_bg)) = light_container.background else {
        panic!("light background");
    };
    let Some(Background::Color(dark_bg)) = dark_container.background else {
        panic!("dark background");
    };
    assert_eq!(light_bg, Theme::Light.extended_palette().background.weak.color.scale_alpha(0.82));
    assert_eq!(dark_bg, Theme::Dark.extended_palette().background.weak.color.scale_alpha(0.28));
}

#[test]
fn render_gap_exact_default_context_boundary_has_no_expand_row_in_both_views() {
    let mut app = app();
    let old_lines = ["0", "1", "2", "3", "4", "5"];

    app.merge_view = true;
    let ctx = DiffRenderCtx::new(&app);
    let merged = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 6, start_new: 0, gap_id: 12 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(merged.len(), 6);

    app.merge_view = false;
    let ctx = DiffRenderCtx::new(&app);
    let split = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 6, start_new: 0, gap_id: 12 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(split.len(), 6);
}

#[test]
fn render_gap_context_expansion_requires_matching_file_and_gap_id() {
    let mut app = app();
    let old_lines = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"];
    app.context_expansions.insert(("src/lib.rs".to_string(), 99), (20, 20));
    app.context_expansions.insert(("src/other.rs".to_string(), 12), (20, 20));

    let ctx = DiffRenderCtx::new(&app);
    let unmatched = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 12, start_new: 0, gap_id: 12 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(unmatched.len(), 7);

    app.context_expansions.insert(("src/lib.rs".to_string(), 12), (2, 1));
    let ctx = DiffRenderCtx::new(&app);
    let matched = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &old_lines,
        GapRange { start_old: 0, end_old: 12, start_new: 0, gap_id: 12 },
        Lang::Other,
        DiffTheme::GitHub,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert_eq!(matched.len(), 10);
}

#[test]
fn render_gap_short_split_range_renders_each_out_of_bounds_line() {
    let mut app = app();
    app.merge_view = false;
    let ctx = DiffRenderCtx::new(&app);

    let elements = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &["only"],
        GapRange { start_old: 4, end_old: 6, start_new: 9, gap_id: 13 },
        Lang::Rust,
        DiffTheme::Monokai,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );

    assert_eq!(elements.len(), 2);
}

#[test]
fn render_gap_split_view_uses_selection_and_hover_for_visible_rows() {
    let mut app = app();
    app.merge_view = false;
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 2,
            is_old: true,
            text: "old selected".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: false,
            text: "new selected".to_string(),
        },
    ];
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 6, false));
    let ctx = DiffRenderCtx::new(&app);

    let elements = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &["0", "1", "2", "3", "4", "5", "6", "7"],
        GapRange { start_old: 0, end_old: 8, start_new: 0, gap_id: 14 },
        Lang::Rust,
        DiffTheme::Monokai,
        Color::from_rgb(0.1, 0.1, 0.1),
        Color::WHITE,
        true,
    );

    assert_eq!(elements.len(), 7);
}

#[test]
fn render_gap_handles_out_of_bounds_lines_and_selected_states() {
    let mut app = app();
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 0,
            is_old: true,
            text: "old".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 11,
            is_old: false,
            text: "new".to_string(),
        },
    ];
    app.git_diff_hovered_line = Some(("src/lib.rs".to_string(), 12, false));
    let ctx = DiffRenderCtx::new(&app);

    let empty = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &["only"],
        GapRange { start_old: 4, end_old: 2, start_new: 10, gap_id: 10 },
        Lang::Other,
        DiffTheme::Monokai,
        Color::TRANSPARENT,
        Color::WHITE,
        false,
    );
    assert!(empty.is_empty());

    let elements = render_gap(
        &app,
        &ctx,
        "src/lib.rs",
        &["only"],
        GapRange { start_old: 0, end_old: 8, start_new: 10, gap_id: 11 },
        Lang::Rust,
        DiffTheme::Monokai,
        Color::TRANSPARENT,
        Color::WHITE,
        true,
    );
    assert_eq!(elements.len(), 7);
}
