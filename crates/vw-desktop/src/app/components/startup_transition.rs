//! 原生端本地服务启动期间的根视图。

use iced::widget::{column, container, progress_bar, text};
use iced::{Background, Element, Length, Theme};

use crate::app::Message;
use crate::app::state::NativeStartupState;

pub(crate) fn view<'a>(
    state: &'a NativeStartupState,
    animation_frame: usize,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let palette = theme.extended_palette();
    let (title, detail) = match state {
        NativeStartupState::Starting => (
            format!("{} 正在启动工作环境", super::status_animation::spinner_frame(animation_frame)),
            "服务就绪后将自动进入主界面",
        ),
        NativeStartupState::Ready => ("本地服务已就绪".to_string(), "正在进入主界面"),
    };
    let phase = animation_frame % 40;
    let progress = if phase < 20 { phase + 1 } else { 40 - phase } as f32 / 20.0;

    let content = column![
        text(title).size(24).color(palette.background.base.text),
        text(detail).size(14).color(palette.background.base.text.scale_alpha(0.72)),
        container(progress_bar(0.0..=1.0, progress).girth(Length::Fixed(4.0)))
            .width(Length::Fixed(240.0)),
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
