//! 终端标签页重命名模块
//!
//! 本模块提供终端标签页标题编辑功能的更新逻辑，包括：
//! - 启动重命名编辑模式
//! - 处理编辑过程中的文本变化
//! - 保存或取消重命名操作
//!
//! 重命名流程采用内联编辑模式，用户在原标签标题位置直接编辑，
//! 支持提交保存或取消操作回滚到原标题。

use super::TerminalMessage;
use crate::app::{App, Message, TerminalState};
use iced::Task;

fn start_edit(title: &str, edit_title: &mut Option<String>) {
    *edit_title = Some(title.to_string());
}

fn change_edit(edit_title: &mut Option<String>, value: String) {
    *edit_title = Some(value);
}

fn save_edit(title: &mut String, edit_title: &mut Option<String>) {
    if let Some(new) = edit_title.take() {
        let name = new.trim().to_string();
        if !name.is_empty() {
            *title = name;
        }
    }
}

fn cancel_edit(edit_title: &mut Option<String>) {
    *edit_title = None;
}

/// 处理终端标签页重命名相关的消息更新
///
/// 该函数是终端重命名功能的核心处理器，根据接收到的消息类型
/// 执行相应的状态更新操作。
///
/// # 参数
///
/// - `app`: 应用程序状态的可变引用，用于访问和修改终端相关数据
/// - `message`: 终端消息枚举，包含重命名操作的具体指令和数据
///
/// # 返回值
///
/// 返回 `Task<Message>`，当前所有重命名操作均不需要异步任务，
/// 始终返回 `Task::none()`
///
/// # 处理的消息类型
///
/// - `RenameStart(id)`: 启动指定标签页的重命名编辑模式
/// - `RenameChanged(id, value)`: 更新编辑中的标题文本
/// - `RenameSave(id)`: 保存编辑后的标题
/// - `RenameCancel(id)`: 取消编辑，丢弃修改
///
/// # 示例
///
/// ```ignore
/// // 在用户右键点击标签页选择"重命名"时触发
/// let task = update(&mut app, TerminalMessage::RenameStart(tab_id));
/// // 用户输入过程中持续触发
/// let task = update(&mut app, TerminalMessage::RenameChanged(tab_id, "新标题".to_string()));
/// // 用户按回车确认保存
/// let task = update(&mut app, TerminalMessage::RenameSave(tab_id));
/// ```
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    update_terminal(&mut app.terminal, message)
}

fn update_terminal(terminal: &mut TerminalState, message: TerminalMessage) -> Task<Message> {
    match message {
        // 启动重命名编辑模式
        TerminalMessage::RenameStart(id) => {
            // 关闭上下文菜单，避免与编辑界面冲突
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 选中目标标签页，确保其处于活动状态
            terminal.select_terminal(id);

            // 初始化编辑状态，将当前标题作为编辑初始值
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                start_edit(&tab.title, &mut tab.edit_title);
            }
            Task::none()
        }

        // 处理编辑过程中的文本变化
        TerminalMessage::RenameChanged(id, v) => {
            // 更新临时编辑缓冲区的内容
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                change_edit(&mut tab.edit_title, v);
            }
            Task::none()
        }

        // 保存编辑后的标题
        TerminalMessage::RenameSave(id) => {
            // 关闭上下文菜单
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 提取编辑缓冲区内容并应用
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                save_edit(&mut tab.title, &mut tab.edit_title);
            }
            Task::none()
        }

        // 取消编辑操作
        TerminalMessage::RenameCancel(id) => {
            // 关闭上下文菜单
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 清除编辑状态，丢弃所有修改，保留原标题
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                cancel_edit(&mut tab.edit_title);
            }
            Task::none()
        }

        // 其他消息类型在本模块不处理，返回空任务
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "rename_tests.rs"]
mod rename_tests;
