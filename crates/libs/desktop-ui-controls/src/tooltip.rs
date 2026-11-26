//! Shared tooltip appearance.
use iced::widget::{column, container, text, tooltip};
use iced::{Background, Border, Color, Element, Font, Theme};

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

pub fn project_tooltip<'a, Message: 'a>(
    control: impl Into<Element<'a, Message>>,
    name: impl text::IntoFragment<'a>,
    path: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    project_tooltip_with_max_width(control, name, path, position, 360.0)
}

pub fn project_tooltip_with_max_width<'a, Message: 'a>(
    control: impl Into<Element<'a, Message>>,
    name: impl text::IntoFragment<'a>,
    path: &'a str,
    position: tooltip::Position,
    max_width: f32,
) -> Element<'a, Message> {
    let name = text(name)
        .size(13)
        .font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT })
        .shaping(text::Shaping::Advanced)
        .wrapping(text::Wrapping::WordOrGlyph);
    let path = text(path)
        .size(12)
        .color(Color::from_rgb8(197, 198, 202))
        .shaping(text::Shaping::Advanced)
        .wrapping(text::Wrapping::WordOrGlyph);
    let details = container(column![name, path].spacing(4)).max_width(max_width);

    tooltip(control, details, position).gap(8).padding(10).style(tooltip_dark_style).into()
}
