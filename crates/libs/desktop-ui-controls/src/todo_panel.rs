/// Todo 面板在会话界面中的承载位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoPanelPlacement {
    InputBottom,
    ChatTopRight,
}

impl TodoPanelPlacement {
    pub fn label(self) -> &'static str {
        match self {
            TodoPanelPlacement::InputBottom => "输入底部",
            TodoPanelPlacement::ChatTopRight => "右上角",
        }
    }
}
