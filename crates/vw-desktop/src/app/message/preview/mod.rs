//! 预览模块 - 文件预览与编辑功能的消息定义与路由
//!
//! 本模块是预览子系统的核心入口，负责：
//! - 定义所有与文件预览、编辑、搜索相关的消息类型
//! - 将消息路由分发到对应的子模块（tab、search、editor、lsp）
//!
//! ## 子模块职责
//!
//! - [`editor`]: 编辑器核心功能（文本编辑、选择、上下文菜单）
//! - [`search`]: 搜索与跳转功能（文本搜索、行号跳转）
//! - [`tab`]: 标签页管理（打开、关闭、导航、追踪历史）
//! - [`lsp`]: 语言服务器协议集成（仅非 WASM 平台）
//!
//! ## 消息路由策略
//!
//! [`update`] 函数根据消息类型将请求分发到对应子模块：
//! - 标签页操作 → [`tab::update`]
//! - 搜索/跳转操作 → [`search::update`]
//! - 编辑器操作 → [`editor::update`]

use crate::app::{App, Message};
use iced::Task;

pub mod editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;
pub mod search;
mod snapshot;
pub mod tab;

pub use desktop_editor::preview::message::PreviewMessage;

pub(crate) fn dismiss_preview_popup_menus(app: &mut App) {
    app.show_preview_context_menu = false;
    app.preview_context_menu_pos = None;
    app.preview_nav_popup = None;
    app.preview_tab_menu_path = None;
    app.preview_tab_menu_pos = None;
}

/// 处理预览模块的消息
///
/// 根据消息类型将请求路由到对应的子模块处理器。
/// 使用模式匹配进行分组，确保同类消息由同一处理器处理。
///
/// # 参数
///
/// - `app`: 应用状态的可变引用，用于读取和修改预览相关状态
/// - `message`: 预览消息枚举，定义具体的操作请求
///
/// # 返回值
///
/// 返回 `Task<Message>`，可能包含需要执行的后续命令（如异步文件操作、LSP 请求等）。
///
/// # 路由规则
///
/// | 消息类型 | 目标处理器 |
/// |---------|-----------|
/// | 标签页操作（打开/关闭/选择/导航） | [`tab::update`] |
/// | 搜索与跳转操作 | [`search::update`] |
/// | 编辑器与上下文菜单操作 | [`editor::update`] |
/// | LSP 相关操作（仅非 WASM） | [`editor::update`] |
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
    match message {
        PreviewMessage::OpenSnapshot { key, path, title, content } => {
            snapshot::open(app, key, path, title, content)
        }
        // 标签页管理相关消息：文件打开、关闭、选择、导航历史、保存
        PreviewMessage::Open(_)
        | PreviewMessage::OpenLoaded { .. }
        | PreviewMessage::OpenFailed { .. }
        | PreviewMessage::Select(_)
        | PreviewMessage::Close(_)
        | PreviewMessage::TraceBack
        | PreviewMessage::TraceForward
        | PreviewMessage::SaveFile
        | PreviewMessage::SaveFilePath { .. }
        | PreviewMessage::SaveFileFinished { .. }
        | PreviewMessage::AutoSaveDelayElapsed { .. }
        | PreviewMessage::WindowUnfocused
        | PreviewMessage::AutoSaveModeChanged(_)
        | PreviewMessage::CloseNavPopup
        | PreviewMessage::NavPopupSelect(_, _)
        | PreviewMessage::PathSegmentClicked(_, _)
        | PreviewMessage::TabRightClicked(_, _, _)
        | PreviewMessage::TabMenuClose
        | PreviewMessage::TabMenuCloseLeft(_)
        | PreviewMessage::TabMenuCloseRight(_)
        | PreviewMessage::TabMenuCloseAll => tab::update(app, message),

        // 搜索与跳转相关消息：文本搜索、行号跳转
        PreviewMessage::SearchChanged(_)
        | PreviewMessage::SearchNext
        | PreviewMessage::SearchPrev
        | PreviewMessage::GotoLineChanged(_)
        | PreviewMessage::GotoLineSubmit
        | PreviewMessage::SearchToggle
        | PreviewMessage::SearchEditorAction(_)
        | PreviewMessage::CopySearchMatches => search::update(app, message),

        // 编辑器核心功能消息：选择模式、编辑动作、上下文菜单
        PreviewMessage::SelectStartLineChanged(_)
        | PreviewMessage::SelectStartColChanged(_)
        | PreviewMessage::SelectEndLineChanged(_)
        | PreviewMessage::SelectEndColChanged(_)
        | PreviewMessage::ToggleSelectMode(_)
        | PreviewMessage::SelectionEditorAction(_)
        | PreviewMessage::ReadOnlyEditorAction(_)
        | PreviewMessage::EditorEvent(_)
        | PreviewMessage::ContextMenuOpen(_, _, _, _, _)
        | PreviewMessage::ContextMenuOpenForActiveEditor(_, _)
        | PreviewMessage::ContextMenuCopy
        | PreviewMessage::ContextMenuCut
        | PreviewMessage::ContextMenuPaste
        | PreviewMessage::ContextMenuDelete
        | PreviewMessage::ContextMenuClose
        | PreviewMessage::FullscreenOverlayEntered
        | PreviewMessage::FullscreenOverlayExited => editor::update(app, message),

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::EditorMouseEntered | PreviewMessage::EditorMouseExited => {
            editor::update(app, message)
        }

        // LSP 功能消息（仅非 WASM 平台）：悬停提示、自动补全
        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspHoverEntered
        | PreviewMessage::LspHoverExited
        | PreviewMessage::LspCompletionSelected(_)
        | PreviewMessage::LspCompletionClosed
        | PreviewMessage::LspCompletionNavigateUp
        | PreviewMessage::LspCompletionNavigateDown
        | PreviewMessage::LspCompletionConfirm
        | PreviewMessage::LspToggleDisabled(_) => editor::update(app, message),
    }
}

/// LSP 周期性任务处理（仅非 WASM 平台）
///
/// 在每个应用周期中调用，用于处理 LSP 服务器的后台任务，
/// 如处理待响应的补全请求、诊断更新等。
///
/// # 参数
///
/// - `app`: 应用状态的可变引用
///
/// # 返回值
///
/// 返回 `Task<Message>`，可能包含需要执行的 LSP 相关命令。
///
/// # 平台说明
///
/// 此函数仅在非 WASM 目标平台可用，因为 LSP 功能依赖文件系统和进程通信，
/// 这些在 WebAssembly 环境中受限。
#[cfg(not(target_arch = "wasm32"))]
pub fn tick_lsp(app: &mut App) -> Task<Message> {
    lsp::tick(app)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "lsp_tests.rs"]
mod lsp_tests;

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
