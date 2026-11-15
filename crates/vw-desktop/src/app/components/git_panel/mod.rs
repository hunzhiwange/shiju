//! Git 面板模块
//!
//! 本模块提供 Git 仓库变更查看和管理的图形界面组件。主要功能包括：
//! - 显示 Git 仓库中的变更文件列表及其 diff 视图
//! - 支持文件过滤（按状态、已暂存/未暂存、搜索关键词）
//! - 自定义文本对比功能
//! - 代码复制和编辑模态框
//! - 行内评论功能
//!
//! 模块结构：
//! - `diff_view`: 变更差异视图渲染
//! - `filter_options`: 过滤选项 UI
//! - `header`: 面板头部
//! - `left_panel`: 左侧面板（汇总视图）
//! - `ops`: Git 操作工具函数
//! - `ui`: UI 组件
//! - `utils`: 工具函数（颜色、文件状态等）

use iced::{Element, Length};
use iced::widget::column;
use crate::app::components::system_settings_common::with_settings_help_modal;
use crate::app::{App, DiffTheme, Message, message};

mod diff_view;
mod filter_options;
mod header;
mod view_messages;
mod left_panel;
mod ops;
mod repository;
pub(crate) mod repository_ops;
#[cfg(test)]
mod repository_tests;
mod ui;
mod utils;

#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
#[cfg(test)]
#[path = "left_panel_tests.rs"]
mod left_panel_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "ops_tests.rs"]
mod ops_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;

pub(crate) use left_panel::traverse_commit_focus;
pub use ops::{DiffFileMeta, checkout_branch, current_branch, list_branches, open_terminal};

pub(crate) fn diff_comment_modal<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    diff_view::diff_comment_editor(app)
}

pub(crate) fn discard_file_modal(app: &App) -> Option<Element<'_, Message>> {
    domain::discard_file_modal(app.desktop_locale, app.file_to_discard.as_deref()).map(|element| element.map(|event| Message::Git(match event {
        domain::PanelMessage::CancelDiscardFile => message::GitMessage::CancelDiscardFile,
        domain::PanelMessage::DiscardFile(file) => message::GitMessage::DiscardFile(file),
    })))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &App) -> Option<String> {
    ops::git_repo_path_for_app(app)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn git_repo_path_for_app(_app: &App) -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get_diff_file_metas_for_repo_path(path: &str) -> Vec<DiffFileMeta> {
    ops::get_diff_file_metas_for_repo_path(path)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn get_diff_file_metas_for_repo_path(_path: &str) -> Vec<DiffFileMeta> {
    vec![]
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_diff_content_for_repo_path(path: &str, meta: &DiffFileMeta) -> (String, String) {
    ops::load_diff_content_for_repo_path(path, meta)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_diff_content_for_repo_path(
    _path: &str,
    _meta: &DiffFileMeta,
) -> (String, String) {
    (String::new(), String::new())
}

/// 获取指定仓库路径下的所有变更文件路径
///
/// # 参数
///
/// * `repo_path` - Git 仓库的根目录路径
///
/// # 返回值
///
/// 返回变更文件的相对路径列表
///
/// # 示例
///
/// ```ignore
/// let files = changed_files_in_repo("/path/to/repo");
/// // 返回: vec!["src/main.rs", "docs/readme.md"]
/// ```
pub fn changed_files_in_repo(repo_path: &str) -> Vec<String> {
    ops::get_changed_file_paths(repo_path)
}

/// 获取当前应用项目中的所有变更文件路径
///
/// # 参数
///
/// * `app` - 应用状态引用
///
/// # 返回值
///
/// 返回变更文件的相对路径列表。如果未设置项目路径，返回空列表。
///
/// # 示例
///
/// ```ignore
/// let files = changed_files(&app);
/// // 根据项目路径获取变更文件列表
/// ```
pub fn changed_files(app: &App) -> Vec<String> {
    let Some(path) = app.project_path.as_deref() else {
        return vec![];
    };
    ops::get_changed_file_paths(path)
}

use desktop_git_ui::git_panel::panel as domain;
use domain::{is_dark_theme, get_diff_colors};
pub fn embedded_custom_text_diff_view(
    app: &App,
    title: String,
    file: Option<String>,
    before: String,
    after: String,
    close_message: Option<Message>,
) -> Element<'_, Message> {
    let effective_theme =
        if is_dark_theme(&app.app_theme) { DiffTheme::Monokai } else { DiffTheme::GitHub };
    let (
        bg_default,
        _fg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
        _header_bg,
        _header_fg,
    ) = get_diff_colors(effective_theme);

    diff_view::view_custom_text_diff(
        app,
        title,
        file,
        before,
        after,
        close_message,
        effective_theme,
        bg_default,
        add_line_bg,
        add_word_bg,
        del_line_bg,
        del_word_bg,
    )
}

/// 将桌面字段借用为本域视图输入。
fn view_input(app: &App) -> domain::ViewInput<'_> { domain::ViewInput {
    git_diff_contents: &app.git_diff_contents,
    app_theme: &app.app_theme,
    desktop_locale: app.desktop_locale,
    expanded_files_set: &app.expanded_files_set,
    git_custom_diff_after_editor: &app.git_custom_diff_after_editor,
    git_custom_diff_before_editor: &app.git_custom_diff_before_editor,
    git_custom_diff_hide_inputs: app.git_custom_diff_hide_inputs,
    git_custom_diff_title: &app.git_custom_diff_title,
    git_diff_contents_loading: &app.git_diff_contents_loading,
    git_diff_file_metas: &app.git_diff_file_metas,
    git_diff_scroll_id: &app.git_diff_scroll_id,
    git_filter_deleted: app.git_filter_deleted,
    git_filter_excluded: app.git_filter_excluded,
    git_filter_included: app.git_filter_included,
    git_filter_modified: app.git_filter_modified,
    git_filter_new: app.git_filter_new,
    git_filter_query: &app.git_filter_query,
    git_focused_file: &app.git_focused_file,
    show_git_custom_diff_modal: app.show_git_custom_diff_modal,
    show_git_diff_summary: app.show_git_diff_summary,
    show_git_filter_options: app.show_git_filter_options,
    staged_files_selected: &app.staged_files_selected,
    staged_hunks_selected: &app.staged_hunks_selected,
    staged_lines_selected: &app.staged_lines_selected,
    staged_old_lines_selected: &app.staged_old_lines_selected,
    git_diff_scroll_viewport_h: app.git_diff_scroll_viewport_h,
    git_diff_scroll_offset_y: app.git_diff_scroll_offset_y,
    diff_state: diff_view::diff_view_state(app), render_ctx: diff_view::diff_render_ctx(app),
    has_chat_diff: app.chat_text_diff.is_some(), file_menu: app.git_diff_file_menu.as_ref().map(|menu| menu.file.as_str()),
} }
pub fn view(app: &App) -> Element<'_, Message> {
    if app.git_repository.history && git_repo_path_for_app(app).is_some() {
        return with_git_overlays(app, column![header::view(app, vec![]), repository::toolbar(app), repository::view(app)].spacing(10).height(Length::Fill).into());
    }
    use domain::ViewMessage as Local;
    let input = view_input(app);
    let projection = domain::project_files(&input);
    let custom_diff = app.show_git_custom_diff_modal.then(|| embedded_custom_text_diff_view(app, app.git_custom_diff_title.clone(), None, app.git_custom_diff_before_editor.text(), app.git_custom_diff_after_editor.text(), None).map(Local::External));
    let chat_diff = app.chat_text_diff.as_ref().map(|d| embedded_custom_text_diff_view(app, d.title.clone(), Some(d.file.clone()), d.before.clone(), d.after.clone(), Some(Message::Git(message::GitMessage::CloseChatTextDiff))).map(Local::External));
    let slots = domain::ViewSlots {
        header: header::view(app, projection.diff_files.iter().map(|meta| meta.path.clone()).collect()).map(Local::External),
        filter_options: app.show_git_filter_options.then(|| filter_options::view(app, projection.included_count, projection.excluded_count, projection.new_count, projection.modified_count, projection.deleted_count).map(Local::External)),
        toolbar: repository::toolbar(app).map(Local::External), left_panel: app.show_git_diff_summary.then(|| left_panel::view(app).map(Local::External)), custom_diff, chat_diff,
    };
    let base = domain::view(&input, projection, slots).map(|event| match event {
        Local::External(event) => event,
        Local::Diff(event) => diff_view::map_diff_message(event),
        Local::CloseCustomDiffModal => Message::Git(message::GitMessage::CloseCustomDiffModal),
        Local::CustomDiffAfterEditorAction(value) => Message::Git(message::GitMessage::CustomDiffAfterEditorAction(value)),
        Local::CustomDiffBeforeEditorAction(value) => Message::Git(message::GitMessage::CustomDiffBeforeEditorAction(value)),
        Local::CustomDiffSwap => Message::Git(message::GitMessage::CustomDiffSwap),
        Local::CustomDiffTitleChanged(value) => Message::Git(message::GitMessage::CustomDiffTitleChanged(value)),
        Local::DiffDragSelectEnd => Message::Git(message::GitMessage::DiffDragSelectEnd),
        Local::DiffScrollChanged { offset_y, viewport_h } => Message::Git(message::GitMessage::DiffScrollChanged { offset_y, viewport_h }),
    });
    with_git_overlays(app, base)
}

fn with_git_overlays<'a>(app: &'a App, base_content: Element<'a, Message>) -> Element<'a, Message> {
    let mut base = base_content;

    if app.show_git_commit_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            crate::app::i18n::I18n::new(app.desktop_locale).__(left_panel::COMMIT_HELP_TITLE),
            left_panel::COMMIT_HELP_TEXT,
            Message::Git(message::GitMessage::CommitHelpClose),
        );
    }
    if app.show_git_filter_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            crate::app::i18n::I18n::new(app.desktop_locale).__(filter_options::FILTER_HELP_TITLE),
            filter_options::FILTER_HELP_TEXT,
            Message::Git(message::GitMessage::FilterHelpClose),
        );
    }

    if !app.show_git_copy_modal { return base; }
    use desktop_git_ui::git_panel::copy_modal::{self, CopyMessage as Local};
    let editor = app.git_copy_modal_use_color.then(|| app.git_copy_modal_code_editor.view().map(|event| Local::External(Message::Git(message::GitMessage::CopyModalCodeEditorEvent(event)))));
    copy_modal::copy_overlay(app.desktop_locale, app.git_copy_modal_use_color, &app.git_copy_modal_editor, editor, base.map(Local::External)).map(|event| match event {
        Local::External(event) => event,
        Local::ToggleCopyModalColored(value) => Message::Git(message::GitMessage::ToggleCopyModalColored(value)),
        Local::InsertCopyModalToChatCurrent => Message::Git(message::GitMessage::InsertCopyModalToChatCurrent),
        Local::CopyModalCopyCurrent => Message::Git(message::GitMessage::CopyModalCopyCurrent),
        Local::CloseCopyModal => Message::Git(message::GitMessage::CloseCopyModal),
        Local::CopyModalEditorAction(action) => Message::Git(message::GitMessage::CopyModalEditorAction(action)),
    })
}
