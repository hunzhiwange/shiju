//! 伙伴尺寸弹窗的宿主状态投影。
use crate::app::{App, Message, message::ViewMessage, state::TaskPetAvatarKind};
use iced::Element;
use desktop_settings_appearance::companion::size_modal as domain;
pub(crate) fn with_size_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
 domain::with_size_modal(&domain::View { desktop_locale: app.desktop_locale, task_pet_size_draft: app.task_pet_size_draft, pending: app.task_pet_size_pending.is_some(), task_pet_avatar_kind: app.task_pet_avatar_kind, none: TaskPetAvatarKind::None }, map_message, super::super::task_pet::avatar_preview, base)
}
fn map_message(action: domain::Action) -> Message { Message::View(match action {
 domain::Action::TaskPetSizeCancel => ViewMessage::TaskPetSizeCancel,
 domain::Action::TaskPetSizePreview(v) => ViewMessage::TaskPetSizePreview(v),
 domain::Action::TaskPetSizeReset => ViewMessage::TaskPetSizeReset,
 domain::Action::TaskPetSizeSave => ViewMessage::TaskPetSizeSave,
}) }
#[cfg(test)]
#[path = "size_modal_tests.rs"]
mod tests;
