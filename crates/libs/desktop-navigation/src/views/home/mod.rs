//! 首页视图与局部导航消息。
pub mod common;
pub mod header;
pub mod recent;

use iced::widget::{column, container};
use iced::{Background, Color, Element, Length, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Header(header::Message),
    Recent(recent::Message),
}

pub fn view<'a>(state: &recent::State<'a>, scroll_id: iced::widget::Id) -> Element<'a, Message> {
    let header = header::render(desktop_i18n::I18n::new(state.desktop_locale), scroll_id)
        .map(Message::Header);
    let body = recent::render_body(state).map(Message::Recent);
    container(column![header, body].width(Length::Fill).height(Length::Fill))
        .style(home_container_style)
        .into()
}

fn home_container_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(if common::is_dark_theme(theme) {
            theme.extended_palette().background.base.color.scale_alpha(0.96)
        } else {
            Color::from_rgba8(246, 248, 252, 0.98)
        })),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
