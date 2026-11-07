//! 管理终端相关消息、配置和标签页状态更新。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::TerminalMessage;
use crate::app::{App, FocusArea, Message};
use iced::Task;

pub(super) fn close_tab(app: &mut App, id: u64) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let was_active = app.terminal.active_id == Some(id);
    if !app.terminal.close_terminal(id) {
        return Task::none();
    }
    if app.terminal.tabs.is_empty() {
        app.terminal.is_visible = false;
        app.focus_area = FocusArea::None;
    }
    let reap = reap_retired_task(app);
    #[cfg(not(target_arch = "wasm32"))]
    if was_active {
        return Task::batch([reap, super::event::focus_active_terminal(app)]);
    }
    reap
}

/// Schedule dropping retired terminals after iced cancels their subscriptions.
pub(crate) fn reap_retired_task(app: &App) -> Task<Message> {
    if app.terminal.has_retired() {
        Task::done(Message::Terminal(TerminalMessage::ReapRetired))
    } else {
        Task::none()
    }
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        TerminalMessage::Add => {
            app.terminal.tab_context_menu_id = None;
            app.terminal.tab_context_menu_pos = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.terminal.content_menu = None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if app.terminal.add_terminal(app.project_path.as_ref().map(std::path::PathBuf::from)) {
                app.terminal.apply_app_theme(&app.app_theme);
                return super::event::focus_active_terminal(app);
            }
            Task::none()
        }
        TerminalMessage::Select(id) => {
            app.terminal.tab_context_menu_id = None;
            app.terminal.tab_context_menu_pos = None;
            app.focus_area = FocusArea::Terminal;
            app.terminal.select_terminal(id);
            #[cfg(not(target_arch = "wasm32"))]
            app.terminal.apply_app_theme(&app.app_theme);
            #[cfg(not(target_arch = "wasm32"))]
            return super::event::focus_active_terminal(app);
            #[cfg(target_arch = "wasm32")]
            Task::none()
        }
        TerminalMessage::Close(id) => {
            app.terminal.tab_context_menu_id = None;
            app.terminal.tab_context_menu_pos = None;
            if app.terminal.tabs.len() <= 1 {
                return Task::none();
            }
            close_tab(app, id)
        }
        TerminalMessage::ReapRetired => {
            app.terminal.reap_retired();
            Task::none()
        }
        TerminalMessage::TabContextOpen(id, x, y) => {
            app.focus_area = FocusArea::Terminal;
            app.terminal.select_terminal(id);
            #[cfg(not(target_arch = "wasm32"))]
            app.terminal.apply_app_theme(&app.app_theme);
            app.terminal.tab_context_menu_id = Some(id);
            app.terminal.tab_context_menu_pos = Some((x, y));
            #[cfg(not(target_arch = "wasm32"))]
            return super::event::focus_active_terminal(app);
            #[cfg(target_arch = "wasm32")]
            Task::none()
        }
        TerminalMessage::TabContextClose => {
            app.terminal.tab_context_menu_id = None;
            app.terminal.tab_context_menu_pos = None;
            Task::none()
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
