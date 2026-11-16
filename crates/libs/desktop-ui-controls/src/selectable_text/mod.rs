//! Selectable text, safe link targets and context menus shared by desktop domains.
pub mod markdown_links;
pub mod markdown_selection;
pub mod selection_menu;
use iced::{Color, Theme};
pub fn chat_text_selection_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgba8(0x8B, 0x93, 0x9C, 0.34)
    } else {
        Color::from_rgba8(0xD9, 0xDE, 0xE5, 0.92)
    }
}

fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}
