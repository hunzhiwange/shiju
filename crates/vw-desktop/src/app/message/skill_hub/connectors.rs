//! 专家领域更新的桌面映射。
use crate::app::{App, Message};
use iced::Task;
pub(crate) use desktop_expert_hub::message::connectors::ConnectorMessage;
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::connectors::ensure_loaded(input))
}
pub(crate) fn load(app: &mut App) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::connectors::load(input))
}
pub(crate) fn update(app: &mut App, message: ConnectorMessage) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::connectors::update(input, message))
}
