use iced::{Color, Length, Size};

use crate::app::components::git_panel::diff_view::DiffRenderCtx;
use crate::app::components::git_panel::utils::Lang;
use crate::app::state::{GitDiffContextMenuState, GitDiffSelectedLine};
use crate::app::{App, DiffTheme};

const FILE: &str = "src/lib.rs";

fn assert_closed_equal_row(
    app: &App,
    old_idx: usize,
    new_idx: usize,
    content: &str,
    lang: Lang,
    theme: DiffTheme,
) {
    let ctx = DiffRenderCtx::new(app);
    let element = super::standard::render_equal_line(
        app,
        &ctx,
        FILE,
        old_idx,
        new_idx,
        content,
        lang,
        theme,
        Color::WHITE,
        Color::from_rgba8(255, 210, 0, 0.22),
        false,
    );

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Fill));
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn render_equal_line_builds_fill_width_closed_row_for_plain_content() {
    let app = App::new().0;

    assert_closed_equal_row(&app, 3, 7, "let value = 1;", Lang::Rust, DiffTheme::GitHub);
}

#[test]
fn render_equal_line_handles_selected_and_hovered_old_and_new_sides() {
    let mut app = App::new().0;
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: FILE.to_string(),
        line: 3,
        is_old: true,
        text: "old selected".to_string(),
    });
    assert_closed_equal_row(&app, 3, 7, "selected old", Lang::Other, DiffTheme::Monokai);

    app.git_diff_selected_lines.clear();
    app.git_diff_selected_lines.push(GitDiffSelectedLine {
        file: FILE.to_string(),
        line: 7,
        is_old: false,
        text: "new selected".to_string(),
    });
    assert_closed_equal_row(&app, 3, 7, "selected new", Lang::Other, DiffTheme::Monokai);

    app.git_diff_selected_lines.clear();
    app.git_diff_hovered_line = Some((FILE.to_string(), 3, true));
    assert_closed_equal_row(&app, 3, 7, "hovered old", Lang::Other, DiffTheme::Monokai);

    app.git_diff_hovered_line = Some((FILE.to_string(), 7, false));
    assert_closed_equal_row(&app, 3, 7, "hovered new", Lang::Other, DiffTheme::Monokai);
}

#[test]
fn render_equal_line_opens_context_menu_only_for_matching_new_side_line() {
    let mut app = App::new().0;
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: FILE.to_string(),
        line: 7,
        is_old: false,
        x: 12.0,
        y: 24.0,
    });
    let ctx = DiffRenderCtx::new(&app);
    let open = super::standard::render_equal_line(
        &app,
        &ctx,
        FILE,
        3,
        7,
        "menu row",
        Lang::Rust,
        DiffTheme::GitHub,
        Color::BLACK,
        Color::TRANSPARENT,
        true,
    );
    assert_eq!(open.as_widget().children().len(), 2);

    app.git_diff_context_menu.as_mut().expect("context menu").is_old = true;
    assert_closed_equal_row(&app, 3, 7, "menu row", Lang::Rust, DiffTheme::GitHub);
}

#[test]
fn render_equal_line_accepts_empty_unicode_and_large_line_indices() {
    let app = App::new().0;

    assert_closed_equal_row(&app, usize::MAX, usize::MAX - 1, "", Lang::Other, DiffTheme::GitHub);
    assert_closed_equal_row(&app, 0, 0, "let 值 = 🦀;", Lang::Rust, DiffTheme::Monokai);
}
