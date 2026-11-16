use super::messages::SettingsMessage;
use crate::app::{App, Message};
use desktop_settings_memory::storage_update as domain;
use iced::Task;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Storage(message) = message else {
        return Task::none();
    };
    use desktop_settings_memory::messages::StorageMessage as S;
    use desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage;
    let help = match &message {
        S::HelpCopy(text) => Some(Message::copy(text.clone())),
        S::HelpAppend(text) => Some(Message::context(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text.clone()))),
        S::HelpOpenUrl(url) => Some(Message::context(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url.clone()))),
        S::HelpSelection(session, index, text_index, text) => Some(Message::selection(session.clone(), *index, *text_index, text.clone())),
        S::None => Some(Message::None),
        _ => None,
    };
    if let Some(help) = help { return Task::done(help); }
    let Some(config) = domain::prepare_save(&mut app.storage_settings, message) else {
        return Task::none();
    };
    crate::app::update_storage_config_async(move |storage| {
        storage.provider.config = config.provider.config
    })
}
#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;
