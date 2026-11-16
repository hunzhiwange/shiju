use session_ui_types::{ChatMessage, TokenUsage};
#[derive(Default)]
pub struct StatusFixture {
    pub chat: Vec<ChatMessage>,
    pub usage: TokenUsage,
    pub requesting: bool,
    pub started: u64,
    pub status_animation_frame: usize,
}
impl StatusFixture {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }
}
pub fn chat_global_status_view(app: &StatusFixture) -> iced::Element<'_, ()> {
    crate::global_status::chat_global_status_view(
        &app.chat,
        app.requesting,
        &app.usage,
        config_ui::DesktopLocale::default(),
        app.status_animation_frame,
        0,
    )
}
