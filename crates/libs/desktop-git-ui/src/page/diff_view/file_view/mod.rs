use super::super::utils::FileStatus;
use super::DiffRenderCtx;
use crate::DiffTheme;
use crate::page::Message;
use crate::page::ViewContext;
use iced::Color;
use iced::Element;

pub fn view_file(
    app: &ViewContext,
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
        || app.git.expanded_files_set.contains(file),
        |(index, _)| app.git.git_repository.selected_file == Some(*index),
    );
    let menu_open = render_ctx.history_file.as_ref().map_or_else(
        || app.git.git_diff_file_menu.as_ref().is_some_and(|menu| menu.file == file),
        |(index, _)| app.git.git_repository.file_menu == Some(*index),
    );
    crate::git_panel::diff_view::file_view::view_file(
        &super::diff_view_state(app),
        expanded,
        menu_open,
        render_ctx,
        file,
        contents,
        loading,
        status,
        insertions,
        deletions,
        effective_theme,
        bg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        is_modified,
    )
    .map(super::map_diff_message)
}
