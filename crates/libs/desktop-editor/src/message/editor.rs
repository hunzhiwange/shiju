//! 编辑器局部消息与活动编辑器更新。
use iced::{Task, Theme};
use iced_code_editor::i18n::Language;
use crate::components::editor::Editor;
pub use crate::components::preview_panel::view::options::{EditorOption, EditorPopover};
#[derive(Debug, Clone)]
pub enum EditorMessage {
    #[cfg(not(target_arch = "wasm32"))]
    InstallLsp(desktop_lsp::install::Server),
    #[cfg(not(target_arch = "wasm32"))]
    CopyLspCommand(desktop_lsp::install::Server),
    #[cfg(not(target_arch = "wasm32"))]
    LspInstalled(Result<(), String>),
    TogglePopover(EditorPopover),
    ClosePopover,
    SetOption(EditorOption, bool),
    SetIndentStyle(iced_code_editor::IndentStyle),
    /// 留空随视口，否则仅接受正整数列宽。
    SetWrapColumn(String),
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


#[derive(Debug, Clone)]
pub enum Event {
    Preview(iced_code_editor::Message),
    Message(EditorMessage),
}

/// 更新活动编辑器；跨域消息由宿主在调用前分派。
pub fn update(
    editor: Option<&mut Editor>,
    popover: &mut Option<EditorPopover>,
    show_settings: &mut bool,
    message: EditorMessage,
) -> Task<Event> {
    match message {
        EditorMessage::SetOption(option, enabled) => {
            if let Some(editor) = editor
            {
                option.set(&mut editor.inner, enabled);
            }
            Task::none()
        }
        EditorMessage::SetWrapColumn(value) => {
            let column = if value.is_empty() {
                None
            } else {
                match value.parse::<usize>() {
                    Ok(column) if column > 0 => Some(column),
                    _ => return Task::none(),
                }
            };
            if let Some(editor) = editor
                && editor.inner.wrap_enabled()
            {
                editor.inner.set_wrap_column(column);
            }
            Task::none()
        }
        EditorMessage::SetIndentStyle(style) => {
            if let Some(editor) = editor
            {
                editor.inner.set_indent_style(style);
            }
            Task::none()
        }

        EditorMessage::ClosePopover => {
            *popover = None;
            Task::none()
        }
        // 切换预览设置面板的显示/隐藏状态
        EditorMessage::OpenSearch => {
            *show_settings = false;
            if let Some(editor) = editor
            {
                return editor.open_search_dialog().map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 打开搜索替换对话框（仅限当前激活的预览标签页）
        EditorMessage::OpenReplace => {
            *show_settings = false;
            if let Some(editor) = editor
            {
                return editor.open_search_replace_dialog().map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 关闭搜索对话框（仅限当前激活的预览标签页）
        EditorMessage::CloseSearch => {
            if let Some(editor) = editor
            {
                return editor.close_search_dialog().map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 字体变更（当前未实现具体逻辑）
        EditorMessage::FontChanged(_) => Task::none(),

        EditorMessage::Undo => {
            if let Some(editor) = editor
            {
                return editor.inner.update(&iced_code_editor::Message::Undo).map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 重做操作（仅限当前激活的预览标签页）
        EditorMessage::Redo => {
            if let Some(editor) = editor
            {
                return editor.inner.update(&iced_code_editor::Message::Redo).map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 复制选中内容到剪贴板（仅限当前激活的预览标签页）
        EditorMessage::Copy => {
            if let Some(editor) = editor
            {
                return editor.inner.update(&iced_code_editor::Message::Copy).map(|e| {
                    Event::Preview(e)
                });
            }
            Task::none()
        }

        // 剪切选中内容到剪贴板（仅限当前激活的预览标签页）
        // 剪切 = 复制 + 删除选中内容
        EditorMessage::Cut => {
            if let Some(editor) = editor
            {
                let _ = editor.inner.update(&iced_code_editor::Message::Copy);
                return editor.inner.update(&iced_code_editor::Message::DeleteSelection).map(
                    |e| {
                        Event::Preview(e)
                    },
                );
            }
            Task::none()
        }

        // 粘贴操作：异步读取剪贴板内容
        EditorMessage::Paste => iced::clipboard::read()
            .map(|content| Event::Message(EditorMessage::ClipboardContentReceived(content))),

        // 处理剪贴板内容接收回调（有内容时粘贴）
        EditorMessage::ClipboardContentReceived(Some(content)) => {
            if let Some(editor) = editor
            {
                return editor.inner.update(&iced_code_editor::Message::Paste(content)).map(
                    |e| {
                        Event::Preview(e)
                    },
                );
            }
            Task::none()
        }

        // 剪贴板内容为空，不执行任何操作
        EditorMessage::ClipboardContentReceived(None) => Task::none(),

        // 删除选中内容（仅限当前激活的预览标签页）
        EditorMessage::Delete => {
            if let Some(editor) = editor
            {
                return editor.inner.update(&iced_code_editor::Message::DeleteSelection).map(
                    |e| {
                        Event::Preview(e)
                    },
                );
            }
            Task::none()
        }
        message => panic!("cross-domain editor message must be dispatched by the host: {message:?}"),
    }
}

/// 只读与关闭期间的输入限制，以及编辑器弹层清理规则。
pub fn prepare_message(shutdown: bool, read_only: bool, popover: &mut Option<EditorPopover>, message: &EditorMessage) -> bool {
    if shutdown && !matches!(message, EditorMessage::ConfigSaved(_)) { return false; }
    if read_only && matches!(message, EditorMessage::OpenReplace | EditorMessage::Undo | EditorMessage::Redo | EditorMessage::Cut | EditorMessage::Paste | EditorMessage::ClipboardContentReceived(_) | EditorMessage::Delete) { return false; }
    if !matches!(message, EditorMessage::TogglePopover(_) | EditorMessage::ClosePopover | EditorMessage::SetOption(_, _) | EditorMessage::SetIndentStyle(_) | EditorMessage::SetWrapColumn(_)) { *popover = None; }
    true
}

pub fn toggle_popover(popover: &mut Option<EditorPopover>, has_file: bool, kind: EditorPopover) {
    *popover = if has_file && *popover != Some(kind) { Some(kind) } else { None };
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;

/// 切换活动文件弹层并关闭互斥的编辑器菜单。
pub fn open_popover(
    editor: Option<&mut Editor>, popover: &mut Option<EditorPopover>,
    show_context_menu: &mut bool,
    nav_popup: &mut Option<(String, f32, f32, Vec<(String, bool)>)>,
    show_settings: &mut bool,
    #[cfg(not(target_arch = "wasm32"))] overlay: &mut iced_code_editor::LspOverlayState,
    #[cfg(not(target_arch = "wasm32"))] overlay_path: &mut Option<String>,
    kind: EditorPopover,
) -> Task<Event> {
    toggle_popover(popover, editor.is_some(), kind);
    *show_context_menu = false;
    *nav_popup = None;
    *show_settings = false;
    #[cfg(not(target_arch = "wasm32"))] {
        overlay.hover_visible = false;
        overlay.completion_visible = false;
        *overlay_path = None;
    }
    match editor {
        Some(editor) => editor.inner.update(&iced_code_editor::Message::CloseCommandPalette).map(Event::Preview),
        None => Task::none(),
    }
}

/// 把语言设置应用到当前领域的编辑器集合。
pub fn apply_language<'a>(editors: impl Iterator<Item = &'a mut Editor>, modal: &mut iced_code_editor::CodeEditor, language: Language) {
    for editor in editors { editor.set_ui_language(language); }
    modal.set_language(language);
}

/// 把有效主题应用到预览与 Git 复制编辑器。
pub fn apply_theme<'a>(editors: impl Iterator<Item = &'a mut Editor>, modal: &mut iced_code_editor::CodeEditor, theme: Theme) {
    for editor in editors { editor.set_theme(theme.clone()); }
    modal.set_theme(iced_code_editor::theme::from_iced_theme(&theme));
}
