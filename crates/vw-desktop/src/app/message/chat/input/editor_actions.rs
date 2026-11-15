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
    desktop_session::message::chat::input::editor_actions::handle_message_editor_action(
        &mut app.chat_message_editors, idx, act,
    );
    Task::none()
}

/// 模块内可见函数，执行 handle_special_text_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_special_text_editor_action(
    app: &mut App,
    msg_idx: usize,
    text_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    desktop_session::message::chat::input::editor_actions::handle_special_text_editor_action(
        &mut app.chat_special_text_editors, msg_idx, text_idx, act,
    );
    Task::none()
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
    desktop_session::message::chat::input::editor_actions::handle_tool_text_editor_action(
        &mut app.chat_tool_text_editors, msg_idx, tool_idx, text_idx, act,
    );
    Task::none()
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
    desktop_session::message::chat::input::editor_actions::handle_markdown_block_editor_action(
        &mut app.chat_markdown_block_editors, crate::app::components::chat_panel::message_view::markdown_block_key(msg_idx, text_idx, block_idx), act,
    );
    Task::none()
}

/// 模块内可见函数，执行 handle_think_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_think_editor_action(
    app: &mut App,
    msg_idx: usize,
    think_idx: usize,
    act: text_editor::Action,
) -> Task<Message> {
    desktop_session::message::chat::input::editor_actions::handle_think_editor_action(
        &mut app.chat_think_editors, msg_idx, think_idx, act,
    );
    Task::none()
}

#[cfg(test)]
#[path = "editor_actions_tests.rs"]
mod editor_actions_tests;
