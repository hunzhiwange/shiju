//! 汇总项目会话消息处理子模块，并提供会话消息分发入口。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::message::project::ProjectMessage;
use crate::app::{App, Message};

mod common;
mod edit;
mod lifecycle;
mod open;
mod worktree;

pub(crate) use lifecycle::{
    cache_external_session, latest_assistant_model_label, loaded_chat_from_gateway_messages,
    steps_from_gateway_messages,
};

#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    if let Some(task) = open::handle(app, message.clone()) {
        return Some(task);
    }
    if let Some(task) = edit::handle(app, message.clone()) {
        return Some(task);
    }
    if let Some(task) = worktree::handle(app, message.clone()) {
        return Some(task);
    }
    lifecycle::handle(app, message)
}
