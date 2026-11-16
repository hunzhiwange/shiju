use crate::app::{App, Message, message::ViewMessage};
use desktop_task_board::{pet_view::{PetView, PetMessage}, pet_state::TaskPetAvatarKind};
fn view(app: &App) -> PetView<'_> {
    PetView {
        desktop_locale: app.desktop_locale,
        task_pet_items: &app.task_pet_items,
        task_pet_collapsed: app.task_pet_collapsed,
        task_pet_avatar_kind: app.task_pet_avatar_kind,
        task_pet_context_menu_pos: app.companion_navigation.task_pet_context_menu_pos,
        task_pet_robot_hovered: app.companion_navigation.task_pet_robot_hovered,
        status_animation_frame: app.status_animation_frame,
        task_pet_reply_request_id: app.task_pet_reply_request_id,
        task_pet_hovered_request_id: app.task_pet_hovered_request_id,
        task_pet_reply_input: &app.task_pet_reply_input,
        task_pet_scroll_id: &app.task_pet_scroll_id,
        render_collapsed: app.task_pet_render_collapsed(),
        preview_scale: app.task_pet_preview_scale(),
        avatar_bounds: app.task_pet_avatar_bounds(),
        active_count: app.task_pet_active_count(),
        is_walking: app.task_pet_is_walking(),
        facing_left: app.task_pet_facing_left(),
    }
}
fn map(message: PetMessage) -> Message {
    Message::View(match message {
        PetMessage::TaskPetWriteMessage => ViewMessage::TaskPetWriteMessage,
        PetMessage::TaskPetToggleCollapsed => ViewMessage::TaskPetToggleCollapsed,
        PetMessage::TaskPetDragStarted => ViewMessage::TaskPetDragStarted,
        PetMessage::TaskPetRobotHover(a) => ViewMessage::TaskPetRobotHover(a),
        PetMessage::TaskPetContextMenuOpened(a, b) => ViewMessage::TaskPetContextMenuOpened(a, b),
        PetMessage::TaskPetContextMenuClosed => ViewMessage::TaskPetContextMenuClosed,
        PetMessage::TaskPetOpenSettings => ViewMessage::TaskPetOpenSettings,
        PetMessage::TaskPetSizeOpen => ViewMessage::TaskPetSizeOpen,
        PetMessage::TaskPetHideWindow => ViewMessage::TaskPetHideWindow,
        PetMessage::TaskPetRemove(a) => ViewMessage::TaskPetRemove(a),
        PetMessage::TaskPetReplyPressed(a) => ViewMessage::TaskPetReplyPressed(a),
        PetMessage::TaskPetReplyInputChanged(a) => ViewMessage::TaskPetReplyInputChanged(a),
        PetMessage::TaskPetReplySubmit => ViewMessage::TaskPetReplySubmit,
        PetMessage::TaskPetHover(a) => ViewMessage::TaskPetHover(a),
        PetMessage::TaskPetItemClicked(a) => ViewMessage::TaskPetItemClicked(a),
    })
}
pub fn window(app: &App) -> iced::Element<'_, Message> {
    desktop_task_board::task_pet::window(view(app)).map(map)
}
pub(super) fn avatar_preview(avatar: TaskPetAvatarKind, size: f32) -> iced::Element<'static, Message> {
    desktop_task_board::task_pet::avatar_preview(avatar, size).map(map)
}

#[cfg(test)]
#[path = "task_pet_tests.rs"]
mod task_pet_tests;
