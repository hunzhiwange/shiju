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

use crate::DiffTheme;
use crate::message_git::messages as message;
use crate::page::help::with_settings_help_modal;
use iced::Element;
use iced::Length;
use iced::widget::column;

mod diff_view;
mod filter_options;
mod header;
mod left_panel;
mod ops;
mod repository;
pub(crate) mod repository_ops;
mod utils;
mod view_messages;

#[cfg(test)]
pub(crate) use left_panel::traverse_commit_focus;
pub use ops::{DiffFileMeta, checkout_branch, current_branch, list_branches};

pub fn diff_comment_modal<'a>(app: &ViewContext<'a>) -> Option<Element<'a, Message>> {
    diff_view::diff_comment_editor(app)
}

pub fn discard_file_modal<'a>(app: &ViewContext<'a>) -> Option<Element<'a, Message>> {
    domain::discard_file_modal(app.desktop_locale, app.git.file_to_discard.as_deref()).map(
        |element| {
            element.map(|event| {
                Message::Git(match event {
                    domain::PanelMessage::CancelDiscardFile => {
                        message::GitMessage::CancelDiscardFile
                    }
                    domain::PanelMessage::DiscardFile(file) => {
                        message::GitMessage::DiscardFile(file)
                    }
                })
            })
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &ViewContext) -> Option<String> {
    ops::git_repo_path_for_app(app)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn git_repo_path_for_app(_app: &ViewContext) -> Option<String> {
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
pub fn changed_files(app: &ViewContext) -> Vec<String> {
    let Some(path) = app.project_path.as_deref() else {
        return vec![];
    };
    ops::get_changed_file_paths(path)
}

use crate::git_panel::panel as domain;
use domain::get_diff_colors;
use domain::is_dark_theme;
pub fn embedded_custom_text_diff_view<'a>(
    app: &ViewContext<'a>,
    title: String,
    file: Option<String>,
    before: String,
    after: String,
    close_message: Option<Message>,
) -> Element<'a, Message> {
    let effective_theme =
        if is_dark_theme(app.app_theme) { DiffTheme::Monokai } else { DiffTheme::GitHub };
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
fn view_input<'a>(app: &ViewContext<'a>) -> domain::ViewInput<'a> {
    domain::ViewInput {
        git_diff_contents: &app.git.git_diff_contents,
        app_theme: app.app_theme,
        desktop_locale: app.desktop_locale,
        expanded_files_set: &app.git.expanded_files_set,
        git_custom_diff_after_editor: &app.git.git_custom_diff_after_editor,
        git_custom_diff_before_editor: &app.git.git_custom_diff_before_editor,
        git_custom_diff_hide_inputs: app.git.git_custom_diff_hide_inputs,
        git_custom_diff_title: &app.git.git_custom_diff_title,
        git_diff_contents_loading: &app.git.git_diff_contents_loading,
        git_diff_file_metas: &app.git.git_diff_file_metas,
        git_diff_scroll_id: &app.git.git_diff_scroll_id,
        git_filter_deleted: app.git.git_filter_deleted,
        git_filter_excluded: app.git.git_filter_excluded,
        git_filter_included: app.git.git_filter_included,
        git_filter_modified: app.git.git_filter_modified,
        git_filter_new: app.git.git_filter_new,
        git_filter_query: &app.git.git_filter_query,
        git_focused_file: &app.git.git_focused_file,
        show_git_custom_diff_modal: app.git.show_git_custom_diff_modal,
        show_git_diff_summary: app.git.show_git_diff_summary,
        show_git_filter_options: app.git.show_git_filter_options,
        staged_files_selected: &app.git.staged_files_selected,
        staged_hunks_selected: &app.git.staged_hunks_selected,
        staged_lines_selected: &app.git.staged_lines_selected,
        staged_old_lines_selected: &app.git.staged_old_lines_selected,
        git_diff_scroll_viewport_h: app.git.git_diff_scroll_viewport_h,
        git_diff_scroll_offset_y: app.git.git_diff_scroll_offset_y,
        diff_state: diff_view::diff_view_state(app),
        render_ctx: diff_view::diff_render_ctx(app),
        has_chat_diff: app.git.chat_text_diff.is_some(),
        file_menu: app.git.git_diff_file_menu.as_ref().map(|menu| menu.file.as_str()),
    }
}
pub fn view<'a>(app: &ViewContext<'a>) -> Element<'a, Message> {
    if app.git.git_repository.history && git_repo_path_for_app(app).is_some() {
        return with_git_overlays(
            app,
            column![header::view(app, vec![]), repository::toolbar(app), repository::view(app)]
                .spacing(10)
                .height(Length::Fill)
                .into(),
        );
    }
    use domain::ViewMessage as Local;
    let input = view_input(app);
    let projection = domain::project_files(&input);
    let custom_diff = app.git.show_git_custom_diff_modal.then(|| {
        embedded_custom_text_diff_view(
            app,
            app.git.git_custom_diff_title.clone(),
            None,
            app.git.git_custom_diff_before_editor.text(),
            app.git.git_custom_diff_after_editor.text(),
            None,
        )
        .map(Local::External)
    });
    let chat_diff = app.git.chat_text_diff.as_ref().map(|d| {
        embedded_custom_text_diff_view(
            app,
            d.title.clone(),
            Some(d.file.clone()),
            d.before.clone(),
            d.after.clone(),
            Some(Message::Git(message::GitMessage::CloseChatTextDiff)),
        )
        .map(Local::External)
    });
    let slots = domain::ViewSlots {
        header: header::view(
            app,
            projection.diff_files.iter().map(|meta| meta.path.clone()).collect(),
        )
        .map(Local::External),
        filter_options: app.git.show_git_filter_options.then(|| {
            filter_options::view(
                app,
                projection.included_count,
                projection.excluded_count,
                projection.new_count,
                projection.modified_count,
                projection.deleted_count,
            )
            .map(Local::External)
        }),
        toolbar: repository::toolbar(app).map(Local::External),
        left_panel: app
            .git
            .show_git_diff_summary
            .then(|| left_panel::view(app).map(Local::External)),
        custom_diff,
        chat_diff,
    };
    let base = domain::view(&input, projection, slots).map(|event| match event {
        Local::External(event) => event,
        Local::Diff(event) => diff_view::map_diff_message(event),
        Local::CloseCustomDiffModal => Message::Git(message::GitMessage::CloseCustomDiffModal),
        Local::CustomDiffAfterEditorAction(value) => {
            Message::Git(message::GitMessage::CustomDiffAfterEditorAction(value))
        }
        Local::CustomDiffBeforeEditorAction(value) => {
            Message::Git(message::GitMessage::CustomDiffBeforeEditorAction(value))
        }
        Local::CustomDiffSwap => Message::Git(message::GitMessage::CustomDiffSwap),
        Local::CustomDiffTitleChanged(value) => {
            Message::Git(message::GitMessage::CustomDiffTitleChanged(value))
        }
        Local::DiffDragSelectEnd => Message::Git(message::GitMessage::DiffDragSelectEnd),
        Local::DiffScrollChanged { offset_y, viewport_h } => {
            Message::Git(message::GitMessage::DiffScrollChanged { offset_y, viewport_h })
        }
    });
    with_git_overlays(app, base)
}

fn with_git_overlays<'a>(
    app: &ViewContext<'a>,
    base_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut base = base_content;

    if app.git.show_git_commit_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            desktop_i18n::I18n::new(app.desktop_locale).__(left_panel::COMMIT_HELP_TITLE),
            left_panel::COMMIT_HELP_TEXT,
            Message::Git(message::GitMessage::CommitHelpClose),
        );
    }
    if app.git.show_git_filter_help_modal {
        base = with_settings_help_modal(
            app,
            base,
            desktop_i18n::I18n::new(app.desktop_locale).__(filter_options::FILTER_HELP_TITLE),
            filter_options::FILTER_HELP_TEXT,
            Message::Git(message::GitMessage::FilterHelpClose),
        );
    }

    if !app.git.show_git_copy_modal {
        return base;
    }
    use crate::git_panel::copy_modal::{self, CopyMessage as Local};
    let editor = app.git.git_copy_modal_use_color.then(|| {
        app.git.git_copy_modal_code_editor.view().map(|event| {
            Local::External(Message::Git(message::GitMessage::CopyModalCodeEditorEvent(event)))
        })
    });
    copy_modal::copy_overlay(
        app.desktop_locale,
        app.git.git_copy_modal_use_color,
        &app.git.git_copy_modal_editor,
        editor,
        base.map(Local::External),
    )
    .map(|event| match event {
        Local::External(event) => event,
        Local::ToggleCopyModalColored(value) => {
            Message::Git(message::GitMessage::ToggleCopyModalColored(value))
        }
        Local::InsertCopyModalToChatCurrent => {
            Message::Git(message::GitMessage::InsertCopyModalToChatCurrent)
        }
        Local::CopyModalCopyCurrent => Message::Git(message::GitMessage::CopyModalCopyCurrent),
        Local::CloseCopyModal => Message::Git(message::GitMessage::CloseCopyModal),
        Local::CopyModalEditorAction(action) => {
            Message::Git(message::GitMessage::CopyModalEditorAction(action))
        }
    })
}

mod help;
#[derive(Debug, Clone)]
pub enum Message {
    Git(crate::message_git::messages::GitMessage),
    None,
    OpenTerminal,
    ToggleSummary,
    CopyCode(String),
    AppendChat(String),
    OpenUrl(String),
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::AppendChat(text),
            TextContextEffect::OpenUrl(url) => Self::OpenUrl(url),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
#[derive(Clone)]
pub struct ViewContext<'a> {
    pub git: &'a crate::state::State,
    pub app_theme: &'a iced::Theme,
    pub desktop_locale: config_ui::DesktopLocale,
    pub project_path: &'a Option<String>,
    pub repo_path: Option<String>,
    pub context_path: Option<String>,
    pub code_mode: bool,
    pub refresh_frame: usize,
    pub last_copied_code_hash: Option<u64>,
}

#[cfg(test)]
#[path = "repository_tests.rs"]
pub(crate) mod repository_tests;

#[cfg(test)]
#[path = "header_tests.rs"]
pub(crate) mod header_tests;

#[cfg(test)]
#[path = "left_panel_tests.rs"]
pub(crate) mod left_panel_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
pub(crate) mod mod_tests;

#[cfg(test)]
#[path = "tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "ops_tests.rs"]
pub(crate) mod ops_tests;
