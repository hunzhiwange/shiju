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
use iced::widget::text_editor::Action;
use iced_code_editor::Message as EditorMessage;

pub mod editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;
pub mod search;
pub mod tab;

/// 预览模块消息枚举
///
/// 定义了文件预览器支持的所有用户交互与系统事件。
/// 消息按功能领域分组，由 [`update`] 函数路由到对应处理器。
#[derive(Debug, Clone)]
pub enum PreviewMessage {
    /// 打开指定路径的文件
    ///
    /// # 参数
    /// - `String`: 文件绝对路径
    Open(String),

    /// 文件内容加载完成
    ///
    /// 由异步加载任务在完成后发送，携带文件内容。
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `content`: 文件内容
    /// - `truncated`: 内容是否因过大而被截断
    OpenLoaded { path: String, content: String, truncated: bool },

    /// 切换到指定路径的标签页（已打开则聚焦，否则打开）
    ///
    /// # 参数
    /// - `String`: 文件路径
    Select(String),

    /// 关闭指定路径的标签页
    ///
    /// # 参数
    /// - `String`: 文件路径
    Close(String),

    /// 搜索框内容变更
    ///
    /// # 参数
    /// - `String`: 当前搜索文本
    SearchChanged(String),

    /// 跳转到下一个搜索匹配
    SearchNext,

    /// 跳转到上一个搜索匹配
    SearchPrev,

    /// 跳转行号输入框内容变更
    ///
    /// # 参数
    /// - `String`: 用户输入的行号文本
    GotoLineChanged(String),

    /// 提交跳转行号请求
    GotoLineSubmit,

    /// 选择模式 - 起始行号变更
    ///
    /// # 参数
    /// - `String`: 起始行号文本
    SelectStartLineChanged(String),

    /// 选择模式 - 起始列号变更
    ///
    /// # 参数
    /// - `String`: 起始列号文本
    SelectStartColChanged(String),

    /// 选择模式 - 结束行号变更
    ///
    /// # 参数
    /// - `String`: 结束行号文本
    SelectEndLineChanged(String),

    /// 选择模式 - 结束列号变更
    ///
    /// # 参数
    /// - `String`: 结束列号文本
    SelectEndColChanged(String),

    /// 切换选择模式的开关状态
    ///
    /// # 参数
    /// - `bool`: 是否启用选择模式
    ToggleSelectMode(bool),

    /// 可编辑区域的编辑器动作
    ///
    /// 用于主编辑区的文本操作。
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    SelectionEditorAction(Action),

    /// 只读区域的编辑器动作
    ///
    /// 用于预览区的只读文本操作。
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    ReadOnlyEditorAction(Action),

    /// 代码编辑器内部事件
    ///
    /// 来自 iced_code_editor 的底层事件。
    ///
    /// # 参数
    /// - `EditorMessage`: 编辑器事件
    EditorEvent(EditorMessage),

    /// 切换搜索面板的显示/隐藏状态
    SearchToggle,

    /// 搜索输入框的编辑器动作
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    SearchEditorAction(Action),

    /// 复制所有搜索匹配的文本到剪贴板
    CopySearchMatches,

    /// 在指定位置打开上下文菜单
    ///
    /// # 参数
    /// - `String`: 目标文件路径
    /// - `usize`: 菜单关联的行号
    /// - `usize`: 菜单关联的列号
    /// - `f32`: 菜单 X 坐标（像素）
    /// - `f32`: 菜单 Y 坐标（像素）
    ContextMenuOpen(String, usize, usize, f32, f32),

    /// 为当前活动编辑器打开上下文菜单
    ///
    /// # 参数
    /// - `f32`: 菜单 X 坐标
    /// - `f32`: 菜单 Y 坐标
    ContextMenuOpenForActiveEditor(f32, f32),

    /// 执行上下文菜单的复制操作
    ContextMenuCopy,

    /// 执行上下文菜单的剪切操作
    ContextMenuCut,

    /// 执行上下文菜单的粘贴操作
    ContextMenuPaste,

    /// 执行上下文菜单的删除操作
    ContextMenuDelete,

    /// 关闭上下文菜单
    ContextMenuClose,

    /// 鼠标进入预览右上角全屏控件区域
    FullscreenOverlayEntered,

    /// 鼠标离开预览右上角全屏控件区域
    FullscreenOverlayExited,

    #[cfg(not(target_arch = "wasm32"))]
    EditorMouseEntered,

    #[cfg(not(target_arch = "wasm32"))]
    EditorMouseExited,

    /// LSP 悬停提示进入（仅非 WASM 平台）
    ///
    /// 当鼠标悬停在符号上时触发，请求 LSP 服务器提供类型信息。
    #[cfg(not(target_arch = "wasm32"))]
    LspHoverEntered,

    /// LSP 悬停提示退出（仅非 WASM 平台）
    ///
    /// 当鼠标离开符号区域时触发，用于关闭悬停提示。
    #[cfg(not(target_arch = "wasm32"))]
    LspHoverExited,

    /// 选择指定的 LSP 自动补全项（仅非 WASM 平台）
    ///
    /// # 参数
    /// - `usize`: 补全列表中的索引
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionSelected(usize),

    /// 关闭 LSP 自动补全面板（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionClosed,

    /// LSP 补全列表向上导航（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionNavigateUp,

    /// LSP 补全列表向下导航（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionNavigateDown,

    /// 确认当前选中的 LSP 补全项（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionConfirm,

    /// 切换 LSP 是否禁用（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspToggleDisabled(bool),

    /// 向后回溯导航历史
    ///
    /// 返回上一个访问过的位置（如跳转定义前的位置）。
    TraceBack,

    /// 向前前进导航历史
    ///
    /// 在执行过 [`TraceBack`] 后，前往下一个位置。
    TraceForward,

    /// 保存当前文件
    SaveFile,

    /// 保存指定路径的文件。
    SaveFilePath { path: String, notify: bool },

    /// 指定文件保存完成。
    SaveFileFinished { path: String, content: String, notify: bool, result: Result<(), String> },

    /// 自动保存延迟已到达。
    AutoSaveDelayElapsed { path: String, revision: u64 },

    /// 窗口失去焦点。
    WindowUnfocused,

    /// 修改预览编辑器自动保存模式。
    AutoSaveModeChanged(crate::app::PreviewAutoSaveMode),

    /// 路径段被点击
    ///
    /// 用于面包屑导航，显示该目录下的文件列表。
    ///
    /// # 参数
    /// - `String`: 被点击的路径段
    /// - `Option<(f32, f32)>`: 点击位置坐标（用于定位弹出菜单）
    PathSegmentClicked(String, Option<(f32, f32)>),

    /// 关闭导航弹出菜单
    CloseNavPopup,

    /// 选择导航弹出菜单中的项目
    ///
    /// # 参数
    /// - `String`: 选中的路径
    /// - `bool`: 是否在新标签页打开
    NavPopupSelect(String, bool),

    /// 标签页被右键点击
    ///
    /// # 参数
    /// - `String`: 标签页对应的文件路径
    /// - `f32`: 点击 X 坐标
    /// - `f32`: 点击 Y 坐标
    TabRightClicked(String, f32, f32),

    /// 关闭标签页右键菜单
    TabMenuClose,

    /// 关闭指定标签页左侧的所有标签页
    ///
    /// # 参数
    /// - `String`: 参考标签页的路径
    TabMenuCloseLeft(String),

    /// 关闭指定标签页右侧的所有标签页
    ///
    /// # 参数
    /// - `String`: 参考标签页的路径
    TabMenuCloseRight(String),

    /// 关闭所有标签页
    TabMenuCloseAll,
}

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
        // 标签页管理相关消息：文件打开、关闭、选择、导航历史、保存
        PreviewMessage::Open(_)
        | PreviewMessage::OpenLoaded { .. }
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
