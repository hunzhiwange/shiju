//! 编辑器消息处理模块
//!
//! 本模块负责处理代码编辑器的所有交互消息，包括：
//! - 编辑器设置（字体、字号、行高、语言、主题）
//! - 搜索替换功能
//! - 剪贴板操作（复制、剪切、粘贴、删除）
//! - 撤销/重做操作
//!
//! 已提交的编辑器设置会异步保存到网关。

use crate::app::{App, Message};
use iced::{Task, Theme};
use iced_code_editor::i18n::Language;
use iced_code_editor::theme;

pub(crate) mod metrics;
pub(crate) mod persistence;

use persistence::EditorSettingsPatch;

/// 当前文件外围面板；单一状态保证操作、设置与帮助互斥。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPopover {
    Actions,
    Settings,
    Help,
}

/// 当前实例的公开布尔选项，不持久化到全局配置。
#[derive(Debug, Clone, Copy)]
pub enum EditorOption {
    LineNumbers,
    Wrap,
    Whitespace,
    IndentGuides,
    ColorPreviews,
    StickyScroll,
    BracketMatch,
    BracketColors,
    Folding,
    AutoIndent,
    AutoBrackets,
    Vim,
}
impl EditorOption {
    pub const ALL: [Self; 12] = [
        Self::LineNumbers,
        Self::Wrap,
        Self::Whitespace,
        Self::IndentGuides,
        Self::ColorPreviews,
        Self::StickyScroll,
        Self::BracketMatch,
        Self::BracketColors,
        Self::Folding,
        Self::AutoIndent,
        Self::AutoBrackets,
        Self::Vim,
    ];
    pub fn enabled(self, editor: &iced_code_editor::CodeEditor) -> bool {
        match self {
            Self::LineNumbers => editor.line_numbers_enabled(),
            Self::Wrap => editor.wrap_enabled(),
            Self::Whitespace => editor.show_whitespace(),
            Self::IndentGuides => editor.show_indent_guides(),
            Self::ColorPreviews => editor.show_color_previews(),
            Self::StickyScroll => editor.sticky_scroll_enabled(),
            Self::BracketMatch => editor.bracket_match_highlight_enabled(),
            Self::BracketColors => editor.bracket_pair_colorization_enabled(),
            Self::Folding => editor.folding_enabled(),
            Self::AutoIndent => editor.auto_indent_enabled(),
            Self::AutoBrackets => editor.auto_close_brackets(),
            Self::Vim => editor.vim_enabled(),
        }
    }
    pub fn set(self, editor: &mut iced_code_editor::CodeEditor, enabled: bool) {
        match self {
            Self::LineNumbers => editor.set_line_numbers_enabled(enabled),
            Self::Wrap => editor.set_wrap_enabled(enabled),
            Self::Whitespace => editor.set_show_whitespace(enabled),
            Self::IndentGuides => editor.set_show_indent_guides(enabled),
            Self::ColorPreviews => editor.set_show_color_previews(enabled),
            Self::StickyScroll => editor.set_sticky_scroll_enabled(enabled),
            Self::BracketMatch => editor.set_bracket_match_highlight_enabled(enabled),
            Self::BracketColors => editor.set_bracket_pair_colorization_enabled(enabled),
            Self::Folding => editor.set_folding_enabled(enabled),
            Self::AutoIndent => editor.set_auto_indent_enabled(enabled),
            Self::AutoBrackets => editor.set_auto_close_brackets(enabled),
            Self::Vim => editor.set_vim_enabled(enabled),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::LineNumbers => "行号",
            Self::Wrap => "自动换行",
            Self::Whitespace => "空白",
            Self::IndentGuides => "缩进线",
            Self::ColorPreviews => "颜色预览",
            Self::StickyScroll => "粘性滚动",
            Self::BracketMatch => "括号匹配",
            Self::BracketColors => "彩色括号",
            Self::Folding => "代码折叠",
            Self::AutoIndent => "自动缩进",
            Self::AutoBrackets => "自动括号",
            Self::Vim => "Vim",
        }
    }
}

/// 编辑器消息枚举
///
/// 定义了编辑器支持的所有交互消息类型，用于处理用户操作和状态变更。
#[derive(Debug, Clone)]
pub enum EditorMessage {
    TogglePopover(EditorPopover),
    ClosePopover,
    SetOption(EditorOption, bool),
    SetIndentStyle(iced_code_editor::IndentStyle),
    /// 切换设置面板显示状态
    ToggleSettings,

    /// 打开搜索对话框
    OpenSearch,

    /// 打开搜索替换对话框
    OpenReplace,

    /// 关闭搜索对话框
    CloseSearch,

    /// 字体变更消息（参数：字体名称）
    FontChanged(String),

    /// 预览滑块值，等释放或停顿后再更新编辑器度量。
    FontSizePreview(f32),
    LineHeightPreview(f32),
    CommitMetrics,
    CommitMetricsDue(u64),

    /// 编辑器界面语言变更（参数：语言类型）
    LanguageChanged(Language),

    /// 编辑器主题变更（参数：Iced 主题）
    ThemeChanged(Theme),

    /// 切换是否跟随系统主题（参数：是否跟随）
    ToggleFollowSystemTheme(bool),

    /// 切换自动行高调整（参数：是否自动调整）
    ToggleAutoLineHeight(bool),

    /// 网关保存结果
    ConfigSaved(Result<(), String>),

    /// 撤销操作
    Undo,

    /// 重做操作
    Redo,

    /// 复制选中内容到剪贴板
    Copy,

    /// 剪切选中内容到剪贴板
    Cut,

    /// 从剪贴板粘贴内容
    Paste,

    /// 剪贴板内容接收回调（参数：剪贴板文本内容，None 表示无内容）
    ClipboardContentReceived(Option<String>),

    /// 删除选中内容
    Delete,
}

/// 处理编辑器消息并更新应用状态
///
/// 该函数是编辑器消息的核心处理器，根据不同的消息类型执行相应的操作：
/// - 更新编辑器配置（字体、字号、行高、主题等）
/// - 执行编辑操作（撤销、重做、复制、剪切、粘贴等）
/// - 管理搜索对话框状态
///
/// # 参数
///
/// * `app` - 应用状态的可变引用，包含所有编辑器相关配置
/// * `message` - 待处理的编辑器消息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，可能包含后续需要执行的异步任务：
/// - 搜索对话框操作返回编辑器事件映射任务
/// - 粘贴操作返回剪贴板读取任务
/// - 其他操作通常返回 `Task::none()`
pub fn update(app: &mut App, message: EditorMessage) -> Task<Message> {
    if app.shutdown_requested && !matches!(message, EditorMessage::ConfigSaved(_)) {
        return Task::none();
    }
    if app
        .active_preview_path
        .as_ref()
        .is_some_and(|path| app.preview_tabs.iter().any(|tab| tab.path == *path && tab.read_only))
        && matches!(
            message,
            EditorMessage::OpenReplace
                | EditorMessage::Undo
                | EditorMessage::Redo
                | EditorMessage::Cut
                | EditorMessage::Paste
                | EditorMessage::ClipboardContentReceived(_)
                | EditorMessage::Delete
        )
    {
        return Task::none();
    }
    if !matches!(message, EditorMessage::TogglePopover(_) | EditorMessage::ClosePopover) {
        app.editor_popover = None;
    }
    match message {
        EditorMessage::SetOption(option, enabled) => {
            if let Some(tab) = app
                .preview_tabs
                .iter_mut()
                .find(|tab| Some(&tab.path) == app.active_preview_path.as_ref())
            {
                option.set(&mut tab.editor.inner, enabled);
            }
            Task::none()
        }
        EditorMessage::SetIndentStyle(style) => {
            if let Some(tab) = app
                .preview_tabs
                .iter_mut()
                .find(|tab| Some(&tab.path) == app.active_preview_path.as_ref())
            {
                tab.editor.inner.set_indent_style(style);
            }
            Task::none()
        }

        EditorMessage::TogglePopover(kind) => {
            let has_file = app
                .active_preview_path
                .as_ref()
                .is_some_and(|path| app.preview_tabs.iter().any(|tab| tab.path == *path));
            app.editor_popover =
                if has_file && app.editor_popover != Some(kind) { Some(kind) } else { None };
            app.show_preview_context_menu = false;
            app.preview_nav_popup = None;
            app.show_preview_settings = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.lsp_overlay.hover_visible = false;
                app.lsp_overlay.completion_visible = false;
                app.lsp_overlay_path = None;
            }
            if let Some(path) = app.active_preview_path.as_ref()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|tab| tab.path == *path)
            {
                return tab
                    .editor
                    .inner
                    .update(&iced_code_editor::Message::CloseCommandPalette)
                    .map(|event| {
                        Message::Preview(crate::app::message::PreviewMessage::EditorEvent(event))
                    });
            }
            Task::none()
        }
        EditorMessage::ClosePopover => {
            app.editor_popover = None;
            Task::none()
        }
        // 切换预览设置面板的显示/隐藏状态
        EditorMessage::ToggleSettings => {
            app.show_preview_settings = !app.show_preview_settings;
            if app.show_preview_settings { Task::none() } else { metrics::commit(app, None) }
        }

        // 打开搜索对话框（仅限当前激活的预览标签页）
        EditorMessage::OpenSearch => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.open_search_dialog().map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 打开搜索替换对话框（仅限当前激活的预览标签页）
        EditorMessage::OpenReplace => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.open_search_replace_dialog().map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 关闭搜索对话框（仅限当前激活的预览标签页）
        EditorMessage::CloseSearch => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.close_search_dialog().map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 字体变更（当前未实现具体逻辑）
        EditorMessage::FontChanged(_) => Task::none(),

        EditorMessage::FontSizePreview(size) => {
            app.editor_metrics_preview.font_size = Some(size);
            app.editor_metrics_preview.schedule_commit()
        }
        EditorMessage::LineHeightPreview(height) => {
            app.editor_metrics_preview.line_height = Some(height);
            app.editor_metrics_preview.schedule_commit()
        }
        EditorMessage::CommitMetrics => metrics::commit(app, None),
        EditorMessage::CommitMetricsDue(generation) => {
            if app.editor_metrics_preview.generation == generation {
                metrics::commit(app, None)
            } else {
                Task::none()
            }
        }
        EditorMessage::ToggleAutoLineHeight(auto) => metrics::commit(app, Some(auto)),

        // 编辑器界面语言变更：更新所有编辑器实例的语言设置
        EditorMessage::LanguageChanged(lang) => {
            app.current_language = lang;

            // 更新所有预览标签页的编辑器语言
            for tab in app.preview_tabs.iter_mut() {
                tab.editor.set_ui_language(lang);
            }

            // 更新 Git 复制模态框中的代码编辑器语言
            app.git_copy_modal_code_editor.set_language(lang);
            Task::none()
        }

        // 编辑器主题变更：更新所有编辑器实例并持久化配置
        EditorMessage::ThemeChanged(theme) => {
            // 更新当前主题设置，并禁用跟随系统主题
            app.editor_theme = theme.clone();
            app.editor_follow_system_theme = false;

            // 更新所有预览标签页的编辑器主题
            for tab in app.preview_tabs.iter_mut() {
                tab.editor.set_theme(theme.clone());
            }

            // 更新 Git 复制模态框主题
            app.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&theme));

            persistence::enqueue(app, EditorSettingsPatch::theme(theme.to_string()))
        }

        // 切换是否跟随系统主题
        EditorMessage::ToggleFollowSystemTheme(follow) => {
            app.editor_follow_system_theme = follow;

            // 获取有效主题（考虑系统主题设置）
            let editor_theme = app.effective_editor_theme();

            // 更新所有预览标签页的编辑器主题
            for tab in app.preview_tabs.iter_mut() {
                tab.editor.set_theme(editor_theme.clone());
            }

            // 更新 Git 复制模态框主题
            app.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&editor_theme));

            persistence::enqueue(app, EditorSettingsPatch::follow_system_theme(follow))
        }

        EditorMessage::ConfigSaved(result) => persistence::save_complete(app, result),

        // 撤销操作（仅限当前激活的预览标签页）
        EditorMessage::Undo => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.inner.update(&iced_code_editor::Message::Undo).map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 重做操作（仅限当前激活的预览标签页）
        EditorMessage::Redo => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.inner.update(&iced_code_editor::Message::Redo).map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 复制选中内容到剪贴板（仅限当前激活的预览标签页）
        EditorMessage::Copy => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.inner.update(&iced_code_editor::Message::Copy).map(|e| {
                    Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(e))
                });
            }
            Task::none()
        }

        // 剪切选中内容到剪贴板（仅限当前激活的预览标签页）
        // 剪切 = 复制 + 删除选中内容
        EditorMessage::Cut => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                let _ = tab.editor.inner.update(&iced_code_editor::Message::Copy);
                return tab.editor.inner.update(&iced_code_editor::Message::DeleteSelection).map(
                    |e| {
                        Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(
                            e,
                        ))
                    },
                );
            }
            Task::none()
        }

        // 粘贴操作：异步读取剪贴板内容
        EditorMessage::Paste => iced::clipboard::read()
            .map(|content| Message::Editor(EditorMessage::ClipboardContentReceived(content))),

        // 处理剪贴板内容接收回调（有内容时粘贴）
        EditorMessage::ClipboardContentReceived(Some(content)) => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.inner.update(&iced_code_editor::Message::Paste(content)).map(
                    |e| {
                        Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(
                            e,
                        ))
                    },
                );
            }
            Task::none()
        }

        // 剪贴板内容为空，不执行任何操作
        EditorMessage::ClipboardContentReceived(None) => Task::none(),

        // 删除选中内容（仅限当前激活的预览标签页）
        EditorMessage::Delete => {
            if let Some(path) = app.active_preview_path.clone()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                return tab.editor.inner.update(&iced_code_editor::Message::DeleteSelection).map(
                    |e| {
                        Message::Preview(crate::app::message::preview::PreviewMessage::EditorEvent(
                            e,
                        ))
                    },
                );
            }
            Task::none()
        }
    }
}
