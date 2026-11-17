pub use desktop_chat_view::utils::ui::{bold_font, capped_scroll_height, chat_context_target_key, copy_tooltip_content};
use crate::app::{Message, message::ChatMessage};
use crate::app::components::text_editor_context_menu::TextContextAction;
pub fn chat_context_menu<'a>(i18n: crate::app::i18n::I18n, is_open: bool) -> Option<iced::Element<'a, Message>> {
    desktop_chat_view::utils::ui::chat_context_menu(i18n, is_open).map(|menu| menu.map(|action| Message::Chat(match action {
        TextContextAction::Copy => ChatMessage::CopyContextMenuText,
        TextContextAction::Append => ChatMessage::AppendContextMenuText,
        TextContextAction::Baidu => ChatMessage::SearchContextMenuWithBaidu,
        TextContextAction::Google => ChatMessage::SearchContextMenuWithGoogle,
        TextContextAction::Bing => ChatMessage::SearchContextMenuWithBing,
    })))
}

#[cfg(test)]
pub use desktop_chat_view::utils::ui::chat_context_menu_button_style;
#[cfg(test)]
pub use desktop_chat_view::utils::ui::chat_context_menu_style;
#[cfg(test)]
pub use desktop_chat_view::utils::ui::copy_tooltip_style;
#[cfg(test)]
pub use desktop_chat_view::utils::ui::text_context_menu;
