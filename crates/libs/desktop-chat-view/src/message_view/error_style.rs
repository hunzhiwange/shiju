use crate::presentation::is_dark_theme;
use iced::{Color, Theme};
pub fn subtle_card_shadow(theme: &Theme) -> iced::Shadow {
    iced::Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.10 } else { 0.03 }),
        offset: iced::Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    }
}
