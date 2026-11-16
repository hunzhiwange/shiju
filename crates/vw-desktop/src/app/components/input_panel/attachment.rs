//! 桌面附件消息映射。
use crate::app::{App, Message, message};
use iced::Element;
use desktop_chat_input::input_panel::attachment as shared;
pub(crate) use shared::{AttachmentDisplayKind, AttachmentDisplayItem, parse_attachment_markers};

fn map_message(message: shared::Message) -> Message {
    match message {
        shared::Message::Pick => Message::Project(message::ProjectMessage::AttachmentFilesPick),
        shared::Message::Open(path) => Message::Preview(message::PreviewMessage::Open(path)),
        shared::Message::Remove(path) => Message::Project(message::ProjectMessage::RemoveAttachedFile(path)),
    }
}
pub fn attachment_button(app: &App) -> Element<'_, Message> {
    shared::attachment_button(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &app.session_state.files,
        app.multimodal_settings.max_images as usize, app.multimodal_settings.max_image_size_mb as usize,
        super::styles::round_icon_button_style,
    ).map(map_message)
}
pub fn attachment_strip(app: &App) -> Element<'_, Message> {
    shared::attachment_strip(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &app.session_state.files).map(map_message)
}
pub(crate) fn attachment_preview_strip(
    i18n: crate::app::i18n::I18n, items: Vec<AttachmentDisplayItem>,
) -> Element<'static, Message> {
    shared::attachment_preview_strip(i18n, items).map(map_message)
}
