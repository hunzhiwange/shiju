//! 专家领域更新的桌面映射。
use crate::app::{App, Message};
use iced::Task;
pub(crate) use desktop_expert_hub::message::knowledge::KnowledgeMessage;
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::knowledge::ensure_loaded(input))
}
pub(crate) fn load(app: &mut App) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::knowledge::load(input))
}
pub(crate) fn update(app: &mut App, message: KnowledgeMessage) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::knowledge::update(input, message))
}
