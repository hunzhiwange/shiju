//! Git diff 的领域消息接线。
use super::GitMessage;
use crate::app::{App, Message};
pub(super) fn update(app: &mut App, message: GitMessage) -> iced::Task<Message> {
    super::effects::update(app, message, desktop_git_ui::message_git::controller::diff::update)
}
