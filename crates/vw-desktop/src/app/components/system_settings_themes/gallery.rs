//! Theme gallery host adapter.
pub use desktop_settings_appearance::system_settings_themes::gallery::*;
use crate::app::{App, Message};
pub fn view(app: &App) -> iced::Element<'_, Message> { desktop_settings_appearance::system_settings_themes::gallery::view(&super::inputs(app)) }
pub fn refresh_gallery_state(app: &mut App) { desktop_settings_appearance::system_settings_themes::gallery::refresh_gallery_state(&mut app.themes_settings); }
