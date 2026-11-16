pub type SettingsMessage = crate::messages::SettingsMessage<(), (), (), ()>;
#[derive(Debug, Clone)]
pub enum Message {
    None,
    GatewayHealthTick,
    Settings(SettingsMessage),
    CopyCode(String),
}
impl desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
