use crate::app::{App, Message, message::ViewMessage};
use desktop_task_board::{
    pet_state::TaskPetAvatarKind,
    pet_view::{PetMessage, PetView},
};
fn view(app: &App) -> PetView<'_> {
    PetView {
        dock: app.pet_state.controls.view(app.task_pet_voice(), iced::time::Instant::now()),
        desktop_locale: app.general_settings.desktop_locale,
        task_pet_items: &app.pet_state.task_pet_items,
        task_pet_avatar_kind: app.pet_state.task_pet_avatar_kind,
        task_pet_context_menu_pos: app.companion_navigation.task_pet_context_menu_pos,
        task_pet_robot_hovered: app.companion_navigation.task_pet_robot_hovered,
        status_animation_frame: app.status_animation_frame,
        task_pet_reply_request_id: app.pet_state.task_pet_reply_request_id,
        task_pet_hovered_request_id: app.pet_state.task_pet_hovered_request_id,
        task_pet_reply_input: &app.pet_state.task_pet_reply_input,
        task_pet_scroll_id: &app.pet_state.task_pet_scroll_id,
        render_collapsed: app.pet_state.task_pet_collapsed,
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
        PetMessage::TaskPetDockHover(value) => ViewMessage::TaskPetDockHover(value),
        PetMessage::TaskPetComposerChanged(value) => ViewMessage::TaskPetComposerChanged(value),
        PetMessage::TaskPetComposerSubmit => ViewMessage::TaskPetComposerSubmit,
        PetMessage::TaskPetComposerClose => ViewMessage::TaskPetComposerClose,
        PetMessage::TaskPetStartVoice => ViewMessage::TaskPetStartVoice,
        PetMessage::TaskPetVoiceMic => ViewMessage::TaskPetVoiceMic,
        PetMessage::TaskPetVoiceOrb(action) => ViewMessage::TaskPetVoiceOrb(action),
        PetMessage::TaskPetToggleReminders => ViewMessage::TaskPetToggleReminders,
        PetMessage::TaskPetExpandReminders => ViewMessage::TaskPetExpandReminders,
        PetMessage::TaskPetOpenReminder(id) => ViewMessage::TaskPetOpenReminder(id),
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
    })
}
pub fn window(app: &App) -> iced::Element<'_, Message> {
    let dock = desktop_task_board::task_pet::window(view(app)).map(map);
    let Some(feedback) = app.task_pet_feedback() else {
        return dock;
    };
    use iced::widget::{
        column, container, scrollable,
        scrollable::{Direction, Scrollbar},
    };
    let toast =
        desktop_ui_controls::toast::view(Some(feedback), app.general_settings.desktop_locale, None);
    let bar = container(
        scrollable(container(toast).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    )
    .center_x(iced::Length::Fill)
    .center_y(desktop_task_board::pet_view::FEEDBACK_HEIGHT)
    .padding(7);
    column![bar, dock].into()
}
pub(super) fn avatar_preview(
    avatar: TaskPetAvatarKind,
    size: f32,
) -> iced::Element<'static, Message> {
    desktop_task_board::task_pet::avatar_preview(avatar, size).map(map)
}
