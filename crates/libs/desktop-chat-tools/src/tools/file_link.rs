//! 聊天工具中的可点击文件名。

use std::path::Path;

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{container, rich_text, span, text};
use iced::{Background, Border, Color, Element, Theme};

use crate::card_input::Message;

pub fn file_name_only(path: &str) -> String {
    Path::new(path).file_name().and_then(|value| value.to_str()).unwrap_or(path).to_string()
}

pub fn file_name_link<'a, Host: Clone + 'a>(display_path: &str, full_path: &str) -> Element<'a, Message<Host>> {
    let file_name = file_name_only(display_path);
    let open_path = full_path.to_string();
    let link = rich_text([span(file_name).underline(true).link(open_path)])
        .size(14)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().primary.base.color.scale_alpha(0.92)),
        })
        .on_link_click(|path| Message::OpenPreview(path));

    Tooltip::new(link, file_path_tooltip(full_path.to_string()), TooltipPosition::Top).gap(6).into()
}

pub fn file_path_tooltip<'a, Host: Clone + 'a>(full_path: String) -> Element<'a, Message<Host>> {
    container(text(full_path).size(12))
        .padding([6, 8])
        .style(|_theme: &Theme| iced::widget::container::Style {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.94))),
            border: Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}
