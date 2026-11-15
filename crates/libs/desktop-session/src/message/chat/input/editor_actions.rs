//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use std::collections::HashMap;
use iced::widget::text_editor;

/// 模块内可见函数，执行 handle_message_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_message_editor_action(
    editors: &mut [text_editor::Content],
    idx: usize,
    act: text_editor::Action,
) {
    if let Some(content) = editors.get_mut(idx) {
        match act {
            text_editor::Action::Edit(_) => (),
            other => {
                content.perform(other);
            }
        }
    }
}

/// 模块内可见函数，执行 handle_special_text_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_special_text_editor_action(
    editors: &mut HashMap<u64, text_editor::Content>,
    msg_idx: usize,
    text_idx: usize,
    act: text_editor::Action,
) {
    let key = ((msg_idx as u64) << 32) | (text_idx as u64);
    if let Some(content) = editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => (),
            other => {
                content.perform(other);
            }
        }
    }
}

/// 模块内可见函数，执行 handle_tool_text_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_tool_text_editor_action(
    editors: &mut HashMap<u128, text_editor::Content>,
    msg_idx: usize,
    tool_idx: usize,
    text_idx: usize,
    act: text_editor::Action,
) {
    let key = ((msg_idx as u128) << 64) | ((tool_idx as u128) << 32) | (text_idx as u128);
    if let Some(content) = editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => (),
            other => {
                content.perform(other);
            }
        }
    }
}

/// 模块内可见函数，执行 handle_markdown_block_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_markdown_block_editor_action(
    editors: &mut HashMap<u128, text_editor::Content>,
    key: u128,
    act: text_editor::Action,
) {
    if let Some(content) = editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => (),
            other => {
                content.perform(other);
            }
        }
    }
}

/// 模块内可见函数，执行 handle_think_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn handle_think_editor_action(
    editors: &mut HashMap<u64, text_editor::Content>,
    msg_idx: usize,
    think_idx: usize,
    act: text_editor::Action,
) {
    let key = ((msg_idx as u64) << 32) | (think_idx as u64);
    if let Some(content) = editors.get_mut(&key) {
        match act {
            text_editor::Action::Edit(_) => (),
            other => {
                content.perform(other);
            }
        }
    }
}
#[cfg(test)]
#[path = "editor_actions_tests.rs"]
mod editor_actions_tests;
