//! Pointer geometry shared by desktop layout adapters.
pub const MIN_WORK_SIDEBAR_WIDTH: f32 = 220.0;
pub const MAX_WORK_SIDEBAR_WIDTH: f32 = 520.0;
pub const MIN_WORK_SPLIT_RATIO: f32 = 0.25;
pub const MAX_WORK_SPLIT_RATIO: f32 = 0.75;

/// Capture the first pointer position; subsequent positions resize within the given bounds.
pub fn drag_value(anchor: &mut Option<f32>, position: f32, start: f32, scale: f32, min: f32, max: f32) -> Option<f32> {
    let Some(origin) = *anchor else {
        *anchor = Some(position);
        return None;
    };
    Some((start + (position - origin) / scale).clamp(min, max))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalCommand { Search, Copy }

pub fn global_command(key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> Option<GlobalCommand> {
    if !modifiers.command() { return None; }
    match key {
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("p") => Some(GlobalCommand::Search),
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") => Some(GlobalCommand::Copy),
        _ => None,
    }
}

#[cfg(test)]
#[path = "view_layout_tests.rs"]
mod tests;

/// Display a translated error over the current window contents.
pub fn with_error<'a, M: Clone + 'a>(content: iced::Element<'a, M>, i18n: desktop_i18n::I18n, error: String, close: M) -> iced::Element<'a, M> {
            use iced::widget::{button, column, container, row, text};
            use iced::{Background, Border, Color, Length};

            let card =
                container(
                    column![
                        text(i18n.__("操作失败"))
                            .size(20)
                            .color(Color::from_rgb(0.86, 0.2, 0.2)),
                        text(error)
                        .size(14)
                        .width(Length::Fill),
                        row![
                            container(text("")).width(Length::Fill),
                            button(
                                text(i18n.__("知道了"))
                                    .size(14)
                            )
                            .on_press(close)
                            .padding([8, 12])
                            .style(|theme, status| {
                                let mut style = button::primary(theme, status);
                                style.border.radius = 8.0.into();
                                style
                            })
                        ]
                        .align_y(iced::Alignment::Center)
                    ]
                    .spacing(12),
                )
                .width(Length::Fixed(560.0))
                .padding(16)
                .style(|theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(Background::Color(palette.background.base.color)),
                        text_color: Some(palette.background.base.text),
                        border: Border {
                            radius: 12.0.into(),
                            width: 1.0,
                            color: palette.secondary.weak.color,
                        },
                        ..Default::default()
                    }
                });

            iced::widget::stack![
                content,
                container(
                    container(card)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
                    ..Default::default()
                })
            ]
            .into()
}

#[path = "view_layout_drag.rs"]
pub mod drag;
