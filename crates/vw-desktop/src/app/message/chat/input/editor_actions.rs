//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use crate::app::{App, Message};
use iced::{Task, widget::text_editor};

/// 模块内可见函数，执行 handle_message_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_message_editor_action(
    app: &mut App,
    idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    if let Some(content) = app.chat_message_editors.get_mut(idx) {
        match act {
            text_editor::Action::Edit(_) => Task::none(),
            other => {
                content.perform(other);
                Task::none()
            }
        }
    } else {
        Task::none()
    }
}

/// 模块内可见函数，执行 handle_special_text_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_special_text_editor_action(
    app: &mut App,
    msg_idx: usize,
    text_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    let key = ((msg_idx as u64) << 32) | (text_idx as u64);
    if let Some(content) = app.chat_special_text_editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => Task::none(),
            other => {
                content.perform(other);
                Task::none()
            }
        }
    } else {
        Task::none()
    }
}

/// 模块内可见函数，执行 handle_tool_text_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_tool_text_editor_action(
    app: &mut App,
    msg_idx: usize,
    tool_idx: usize,
    text_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    let key = ((msg_idx as u128) << 64) | ((tool_idx as u128) << 32) | (text_idx as u128);
    if let Some(content) = app.chat_tool_text_editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => Task::none(),
            other => {
                content.perform(other);
                Task::none()
            }
        }
    } else {
        Task::none()
    }
}

/// 模块内可见函数，执行 handle_markdown_block_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_markdown_block_editor_action(
    app: &mut App,
    msg_idx: usize,
    text_idx: usize,
    block_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    let key = crate::app::components::chat_panel::message_view::markdown_block_key(
        msg_idx, text_idx, block_idx,
    );
    if let Some(content) = app.chat_markdown_block_editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => Task::none(),
            other => {
                content.perform(other);
                Task::none()
            }
        }
    } else {
        Task::none()
    }
}

/// 模块内可见函数，执行 handle_think_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_think_editor_action(
    app: &mut App,
    msg_idx: usize,
    think_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    let key = ((msg_idx as u64) << 32) | (think_idx as u64);
    if let Some(content) = app.chat_think_editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => Task::none(),
            other => {
                content.perform(other);
                Task::none()
            }
        }
    } else {
        Task::none()
    }
}
#[cfg(test)]
#[path = "editor_actions_tests.rs"]
mod editor_actions_tests;
