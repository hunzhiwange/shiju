//! 原生端本地服务启动期间的根视图。

use iced::widget::{column, container, text};
use iced::{Background, Element, Length, Theme};

use crate::app::state::NativeStartupState;
use crate::app::Message;

pub(crate) fn view<'a>(
    state: &'a NativeStartupState,
    animation_frame: usize,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let palette = theme.extended_palette();
    let (title, detail) = match state {
        NativeStartupState::Starting => (
            format!(
                "{} 正在启动本地服务",
                super::status_animation::spinner_frame(animation_frame)
            ),
            "服务就绪后将自动进入主界面",
        ),
        NativeStartupState::Failed(error) => ("本地服务启动失败".to_string(), error.as_str()),
        NativeStartupState::Ready => ("本地服务已就绪".to_string(), "正在进入主界面"),
    };

    let content = column![
        text(title).size(24).color(palette.background.base.text),
        text(detail)
            .size(14)
            .color(palette.background.base.text.scale_alpha(0.72)),
    ]
    .spacing(12)
    .align_x(iced::Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.background.base.color)),
            ..Default::default()
        })
        .into()
}
