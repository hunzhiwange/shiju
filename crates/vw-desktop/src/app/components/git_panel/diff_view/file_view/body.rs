use crate::app::{App, DiffTheme, Message};
use iced::{Element, Color};
use crate::app::components::git_panel::utils::{FileStatus};
use crate::app::components::git_panel::diff_view::DiffRenderCtx;

pub(in crate::app::components::git_panel::diff_view) fn view_file_body(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    contents: Option<(&str, &str)>,
    loading: bool,
    status: FileStatus,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
    is_modified: bool,
) -> Element<'static, Message> { desktop_git_ui::git_panel::diff_view::file_view::body::view_file_body(&crate::app::components::git_panel::diff_view::diff_view_state(app), render_ctx, file, contents, loading, status, effective_theme, bg_default, add_line_bg, add_word_bg, del_line_bg, del_word_bg, is_modified).map(crate::app::components::git_panel::diff_view::map_diff_message) }
