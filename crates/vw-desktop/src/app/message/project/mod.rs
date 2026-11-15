//! 汇总项目消息处理子模块，并分发项目、会话和配置相关消息。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::{App, Message};
use iced::Task;

mod basic;
pub(crate) mod file_tree;
mod find;
pub(crate) mod helpers;
mod session;

#[cfg(target_arch = "wasm32")]
pub(crate) use helpers::refresh_file_index;
pub(crate) use helpers::{
    prepare_session_ui_chunks_task, prepare_session_ui_task, prepare_stream_message_ui_task,
};
pub(crate) use session::{
    cache_external_session, open_existing_checkout, open_project_session, open_session_draft,
};

pub use desktop_workspace::project_message::message::{
    FileTreeAction, LoadedProjectInfo, PreparedSessionOpenSource,
};

/// 壳指定会话加载与渲染载荷；项目消息由工作区域库拥有。
pub type ProjectMessage = desktop_workspace::project_message::message::ProjectMessage<
    crate::app::session::PreparedSessionOpen,
    crate::app::session::chat_timeline::TimelinePage,
    crate::app::session::PreparedChatUiPhase,
>;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub fn update(app: &mut App, message: ProjectMessage) -> Task<Message> {
    if let Some(task) = basic::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = find::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = file_tree::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = session::handle(app, message) {
        return task;
    }
    Task::none()
}
