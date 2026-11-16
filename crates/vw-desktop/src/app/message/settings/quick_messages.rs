//! Quick-message persistence and desktop feedback adapter.
use crate::app::{App, Message};
use iced::Task;
use super::{SettingsMessage, QuickMessagesMessage};
use desktop_settings_appearance::quick_messages_update::{self as domain, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::QuickMessages(message) = message else { return Task::none(); };
    #[cfg(not(target_arch = "wasm32"))]
    if !matches!(&message, QuickMessagesMessage::Saved(_)) { app.note_startup_setting_edit("quick_messages"); }
    match domain::update(&mut app.quick_messages, message) {
        Effect::None => Task::none(),
        Effect::Save(items) => Task::perform(
            desktop_settings_appearance::general_update::save_quick_messages(super::general::config_access(), items),
            |result| Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Saved(result))),
        ),
        Effect::Toast(kind, source, arguments) => app.show_localized_toast(super::themes::toast_kind(kind), source, arguments),
    }
}
