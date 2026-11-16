//! Companion window navigation, separate from native window resources.
#[derive(Default)]
pub struct State {
    pub task_pet_visible: bool,
    pub task_pet_robot_hovered: bool,
    pub task_pet_context_menu_pos: Option<(f32, f32)>,
}
pub enum Message {
    Show,
    Hide,
    OpenContext(f32, f32),
    CloseContext,
}
pub fn update(state: &mut State, message: Message) {
    match message {
        Message::Show => {
            state.task_pet_context_menu_pos = None;
            state.task_pet_visible = true;
        }
        Message::Hide => {
            state.task_pet_context_menu_pos = None;
            state.task_pet_visible = false;
            state.task_pet_robot_hovered = false;
        }
        Message::OpenContext(x, y) => state.task_pet_context_menu_pos = Some((x, y)),
        Message::CloseContext => state.task_pet_context_menu_pos = None,
    }
}
