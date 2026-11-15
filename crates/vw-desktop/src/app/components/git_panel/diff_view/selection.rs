use crate::app::App;
use super::DiffRenderCtx;
use iced::{Border, Color};

pub fn selected_border() -> Border { desktop_git_ui::git_panel::diff_view::selection::selected_border() }

pub fn mix_color(a: Color, b: Color, t: f32) -> Color { desktop_git_ui::git_panel::diff_view::selection::mix_color(a, b, t) }

pub fn is_diff_selected(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    line: usize,
    is_old: bool,
) -> bool { desktop_git_ui::git_panel::diff_view::selection::is_diff_selected(&crate::app::components::git_panel::diff_view::diff_view_state(app), render_ctx, file, line, is_old) }

pub fn is_diff_hovered(app: &App, file: &str, line: usize, is_old: bool) -> bool { desktop_git_ui::git_panel::diff_view::selection::is_diff_hovered(&crate::app::components::git_panel::diff_view::diff_view_state(app), file, line, is_old) }
