use crate::pet_state::TaskPetAvatarKind;
/// 宠物窗口的借用展示输入；窗口生命周期与运动时钟由宿主维护。
#[derive(Clone, Copy)]
pub struct PetView<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub task_pet_items: &'a [super::pet_state::TaskPetItem],
    pub task_pet_collapsed: bool,
    pub task_pet_avatar_kind: TaskPetAvatarKind,
    pub task_pet_context_menu_pos: Option<(f32, f32)>,
    pub task_pet_robot_hovered: bool,
    pub status_animation_frame: usize,
    pub task_pet_reply_request_id: Option<u64>,
    pub task_pet_hovered_request_id: Option<u64>,
    pub task_pet_reply_input: &'a str,
    pub task_pet_scroll_id: &'a iced::widget::Id,
    pub render_collapsed: bool,
    pub preview_scale: f32,
    pub avatar_bounds: f32,
    pub active_count: usize,
    pub is_walking: bool,
    pub facing_left: bool,
}
/// 窗口交互意图，由桌面映射为全局视图消息。
#[derive(Debug, Clone)]
pub enum PetMessage {
    TaskPetWriteMessage,
    TaskPetToggleCollapsed,
    TaskPetDragStarted,
    TaskPetRobotHover(bool),
    TaskPetContextMenuOpened(f32, f32),
    TaskPetContextMenuClosed,
    TaskPetOpenSettings,
    TaskPetSizeOpen,
    TaskPetHideWindow,
    TaskPetRemove(u64),
    TaskPetReplyPressed(u64),
    TaskPetReplyInputChanged(String),
    TaskPetReplySubmit,
    TaskPetHover(Option<u64>),
    TaskPetItemClicked(u64),
}
