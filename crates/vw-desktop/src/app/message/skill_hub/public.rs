//! 专家领域更新的桌面映射。
use crate::app::{App, Message};
use iced::Task;
pub(crate) use desktop_expert_hub::message::public::PublicSkillMessage;
#[cfg(test)]
pub(crate) use desktop_expert_hub::message::public::{PublicRequest, reduce};
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::public::ensure_loaded(input))
}
pub(crate) fn update(app: &mut App, message: PublicSkillMessage) -> Task<Message> {
    super::run_domain(app, |input| desktop_expert_hub::message::public::update(input, message))
}
pub(crate) fn selectable_expert_skill(app: &App) -> Result<String, &'static str> {
    desktop_expert_hub::message::public::selectable_expert_skill(&app.expert_hub.public_market, app.project_path.as_deref(), app.open_tabs.iter().any(|tab| tab.project_path.as_deref() == app.project_path.as_deref() && tab.screen == crate::app::Screen::Project))
}
