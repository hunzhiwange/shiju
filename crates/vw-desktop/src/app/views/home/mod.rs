//! 主页视图模块。
//!
//! 保留 `view()` 作为对外入口，并将内部职责拆分为独立子模块。

mod common;
mod header;
mod recent;

use iced::widget::{column, container};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::{App, Message};

/// 渲染主页视图。
pub fn view(app: &App) -> Element<'_, Message> {
    let header = header::render(app);
    let body_content = recent::render_body(app);

    container(column![header, body_content].width(Length::Fill).height(Length::Fill))
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
