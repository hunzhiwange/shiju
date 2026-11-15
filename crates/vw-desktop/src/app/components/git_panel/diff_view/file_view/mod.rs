use iced::{Color, Element};
use crate::app::{App, DiffTheme, Message};
use super::DiffRenderCtx;
use super::super::utils::FileStatus;

pub fn view_file(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    contents: Option<(&str, &str)>,
    loading: bool,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
    is_modified: bool,
) -> Element<'static, Message> {
    let expanded = render_ctx.history_file.as_ref().map_or_else(
        || app.is_diff_file_expanded(file),
        |(index, _)| app.git_repository.selected_file == Some(*index),
    );
    let menu_open = render_ctx.history_file.as_ref().map_or_else(
        || app.git_diff_file_menu.as_ref().is_some_and(|menu| menu.file == file),
        |(index, _)| app.git_repository.file_menu == Some(*index),
    );
    desktop_git_ui::git_panel::diff_view::file_view::view_file(
        &super::diff_view_state(app), expanded, menu_open, render_ctx, file,
        contents, loading, status, insertions, deletions, effective_theme,
        bg_default, add_line_bg, add_word_bg, del_line_bg, del_word_bg, is_modified,
    ).map(super::map_diff_message)
}
