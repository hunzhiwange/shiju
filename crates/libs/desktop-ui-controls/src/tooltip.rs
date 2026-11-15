//! Shared tooltip appearance.
use iced::widget::container;
use iced::{Background, Border, Color, Theme};

pub fn tooltip_dark_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(12, 13, 15, 0.97))),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba8(255, 255, 255, 0.08),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.32),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}
