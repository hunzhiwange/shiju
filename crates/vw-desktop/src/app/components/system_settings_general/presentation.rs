//! 常规设置试版的分组与交互样式，不影响其他设置页面。

use crate::app::Message;
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style,
};
use iced::widget::{button, column, container, text};
use iced::{Border, Element, Length, Theme};

pub(super) fn group_heading<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(17), text(description).size(13).style(settings_muted_text_style),]
        .spacing(6)
        .into()
}

pub(super) fn group_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into()).padding([0, 22]).width(Length::Fill).style(|theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: Border {
                color: palette.background.strong.color.scale_alpha(0.55),
                width: 1.0,
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    })
}

pub(super) fn tab_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    let palette = theme.extended_palette();
    style.border = Border { radius: 10.0.into(), ..Default::default() };
    style.background = if active || matches!(status, button::Status::Pressed) {
        Some(palette.background.strong.color.scale_alpha(0.5).into())
    } else if matches!(status, button::Status::Hovered) {
        Some(palette.background.weak.color.into())
    } else {
        None
    };
    style.text_color = if active { theme.palette().text } else { palette.background.strong.text };
    style
}
