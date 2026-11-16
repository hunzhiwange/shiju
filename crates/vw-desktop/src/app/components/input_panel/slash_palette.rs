//! Desktop slash palette projection and event mapping.
use crate::app::{App, Message, message};
use desktop_chat_input::input_panel::slash_palette::{self, Message as Event};
pub fn slash_palette_overlay(app: &App) -> iced::Element<'_, Message> {
    slash_palette::slash_palette_overlay(
        crate::app::i18n::I18n::new(app.desktop_locale),
        &crate::app::message::chat::input::ranked_slash_palette_sections(app),
        &app.input_suggestions.slash_palette_query, app.input_suggestions.slash_palette_selected_index,
        app.slash_palette_scroll_id.clone(),
    ).map(|event| Message::Chat(match event {
        Event::Select { kind, id } => message::ChatMessage::SlashPaletteSelect { kind, id },
        Event::Expand(kind) => message::ChatMessage::SlashPaletteExpandKind(kind),
    }))
}
