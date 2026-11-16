//! 伙伴设置的素材注入与局部动作映射。
use crate::app::{App, Message, message::view::ViewMessage, state::TaskPetAvatarKind};
use iced::Element;
#[cfg(test)] use desktop_i18n::I18n;
use desktop_settings_appearance::companion as domain;
fn avatars() -> [TaskPetAvatarKind; 4] { [TaskPetAvatarKind::None, TaskPetAvatarKind::Robot, TaskPetAvatarKind::Beauty, TaskPetAvatarKind::Handsome] }
pub fn content(app: &App) -> Element<'_, Message> {
 domain::content(&domain::View { desktop_locale: app.general_settings.desktop_locale, task_pet_visible: app.companion_navigation.task_pet_visible, task_pet_avatar_kind: app.pet_state.task_pet_avatar_kind, preview_scale: app.task_pet_preview_scale() }, avatars(), map_message, super::task_pet::avatar_preview)
}
fn map_message(action: domain::Action<TaskPetAvatarKind>) -> Message { Message::View(match action {
 domain::Action::TaskPetToggleWindow => ViewMessage::TaskPetToggleWindow,
 domain::Action::TaskPetAvatarSelected(v) => ViewMessage::TaskPetAvatarSelected(v),
 domain::Action::TaskPetSizeOpen => ViewMessage::TaskPetSizeOpen,
}) }
pub(crate) mod size_modal;
#[cfg(test)]
const COMPANIONS: [(TaskPetAvatarKind, &str, &str); 4] = [
    (TaskPetAvatarKind::None, domain::LABELS[0].0, domain::LABELS[0].1),
    (TaskPetAvatarKind::Robot, domain::LABELS[1].0, domain::LABELS[1].1),
    (TaskPetAvatarKind::Beauty, domain::LABELS[2].0, domain::LABELS[2].1),
    (TaskPetAvatarKind::Handsome, domain::LABELS[3].0, domain::LABELS[3].1),
];

#[cfg(test)]
fn preview_id(kind: TaskPetAvatarKind) -> &'static str { domain::preview_id(avatars().iter().position(|k| *k == kind).unwrap()) }

#[cfg(test)]
#[path = "system_settings_companion_tests.rs"]
mod tests;
