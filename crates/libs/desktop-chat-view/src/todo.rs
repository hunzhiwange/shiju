//! Todo panel presentation and session-local items.
use desktop_ui_controls::todo_panel::TodoPanelPlacement;
pub struct State {
    /// Todo 面板是否展开
    pub chat_todo_expanded: bool,
    /// Todo 面板动画进度
    pub chat_todo_anim: f32,
    /// Todo 面板显示位置
    pub chat_todo_placement: TodoPanelPlacement,
    /// Todo 缓存所属会话 ID
    pub chat_todo_session_id: Option<String>,
    /// 当前活跃会话的 Todo 缓存
    pub chat_todo_items: Vec<todo_types::Todo>,
}
impl State {
    pub fn new() -> Self {
        Self {
            chat_todo_expanded: true,
            chat_todo_anim: 1.0,
            chat_todo_placement: TodoPanelPlacement::ChatTopRight,
            chat_todo_session_id: None,
            chat_todo_items: vec![],
        }
    }
}
