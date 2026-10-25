//! 处理渠道设置子模块的状态变更、字段转换和持久化。

mod changes;
mod helpers;
pub(crate) mod persist;
mod projects;
mod toggles;
mod updates;

use crate::app::{App, Message};
use iced::Task;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    updates::update(app, message)
}

#[cfg(test)]
mod tests;
