//! 管理终端相关消息、配置和标签页状态更新。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use crate::app::{App, Message, Shell, TerminalTheme};
use iced::Task;

/// config 子模块，拆分当前领域的局部职责。
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
mod content_menu;
/// event 子模块，拆分当前领域的局部职责。
pub mod event;
/// rename 子模块，拆分当前领域的局部职责。
pub mod rename;
/// tabs 子模块，拆分当前领域的局部职责。
pub mod tabs;

#[cfg(not(target_arch = "wasm32"))]
pub use content_menu::ContentAction;

pub(crate) use tabs::reap_retired_task;

#[derive(Debug, Clone)]
/// 描述 TerminalMessage 支持的离散状态或消息分支。
pub enum TerminalMessage {
    ThemeSelected(TerminalTheme),
    FontSelected(String),
    FontSizeChanged(f32),
    FontSizePreview(f32),
    FontSizePreviewCommit,
    FontSizePreviewCommitDue(u64),
    ShellSelected(Shell),
    ConfigSaveDue(u64),
    ConfigSaved(Result<(), String>),
    Add,
    Select(u64),
    Close(u64),
    /// Drop terminals retired after iced cancelled their subscriptions.
    ReapRetired,
    #[cfg(not(target_arch = "wasm32"))]
    Event(iced_term::Event),
    #[cfg(target_arch = "wasm32")]
    Event(()),
    RenameStart(u64),
    RenameChanged(u64, String),
    RenameSave(u64),
    RenameCancel(u64),
    TabContextOpen(u64, f32, f32),
    TabContextClose,
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuOpen(u64, iced::Point),
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuClose,
    #[cfg(not(target_arch = "wasm32"))]
    ContentMenuAction(u64, ContentAction),
    #[cfg(not(target_arch = "wasm32"))]
    PasteResolved(iced::widget::Id, Option<String>),
    DragStarted,
    #[cfg(not(target_arch = "wasm32"))]
    ToggleFullscreen,
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let was_fullscreen = app.terminal.is_fullscreen;
    let task = match message {
        TerminalMessage::Add
        | TerminalMessage::Select(_)
        | TerminalMessage::Close(_)
        | TerminalMessage::ReapRetired
        | TerminalMessage::TabContextOpen(_, _, _)
        | TerminalMessage::TabContextClose => tabs::update(app, message),

        TerminalMessage::ThemeSelected(_)
        | TerminalMessage::FontSelected(_)
        | TerminalMessage::FontSizeChanged(_)
        | TerminalMessage::FontSizePreview(_)
        | TerminalMessage::FontSizePreviewCommit
        | TerminalMessage::FontSizePreviewCommitDue(_)
        | TerminalMessage::ShellSelected(_)
        | TerminalMessage::ConfigSaveDue(_)
        | TerminalMessage::ConfigSaved(_) => config::update(app, message),

        TerminalMessage::RenameStart(_)
        | TerminalMessage::RenameChanged(_, _)
        | TerminalMessage::RenameSave(_)
        | TerminalMessage::RenameCancel(_) => rename::update(app, message),

        TerminalMessage::Event(_) | TerminalMessage::DragStarted => event::update(app, message),
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::ToggleFullscreen => event::update(app, message),
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::ContentMenuOpen(..)
        | TerminalMessage::ContentMenuClose
        | TerminalMessage::ContentMenuAction(..)
        | TerminalMessage::PasteResolved(..) => content_menu::update(app, message),
    };
    #[cfg(not(target_arch = "wasm32"))]
    if was_fullscreen != app.terminal.is_fullscreen {
        return Task::batch([task, super::view::panel::sync_work_browser(app)]);
    }
    task
}
#[cfg(test)]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod focus_tests;
