use crate::presentation::{chat_secondary_text_color, is_dark_theme};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

fn pending_permission_badge_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(79, 62, 21, 0.72)
        } else {
            Color::from_rgba8(255, 245, 213, 1.0)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(168, 132, 45, 0.72)
            } else {
                Color::from_rgba8(222, 187, 102, 0.92)
            },
            radius: 999.0.into(),
        },
        text_color: Some(chat_secondary_text_color(theme)),
        ..Default::default()
    }
}

fn pending_permission_tool_badge<'a, M: Clone + 'a>(
    label: String,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label = text(label).size(11).style(|theme: &Theme| iced::widget::text::Style {
        color: Some(if is_dark_theme(theme) {
            Color::from_rgba8(255, 235, 177, 0.96)
        } else {
            Color::from_rgba8(142, 103, 16, 1.0)
        }),
    });

    let content = container(label)
        .padding([2, 8])
        .style(|theme: &Theme| pending_permission_badge_style(theme));

    if let Some(on_press) = on_press {
        button(content)
            .padding(0)
            .style(|theme: &Theme, _status| iced::widget::button::Style {
                background: None,
                border: Border::default(),
                text_color: pending_permission_badge_style(theme)
                    .text_color
                    .unwrap_or_else(|| chat_secondary_text_color(theme)),
                ..Default::default()
            })
            .on_press(on_press)
            .into()
    } else {
        content.into()
    }
}

pub fn wrap_pending_permission_tool_card<'a, M: Clone + 'a>(
    content: Element<'a, M>,
    label: String,
    on_press: Option<M>,
    is_active: bool,
) -> Element<'a, M> {
    let card = column![
        row![
            container(Space::new()).width(Length::Fill),
            pending_permission_tool_badge(label, on_press)
        ]
        .align_y(Alignment::Center),
        content
    ]
    .spacing(6);

    if is_active {
        container(card)
            .padding([6, 8])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let is_dark = is_dark_theme(theme);
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba8(61, 49, 19, 0.24)
                    } else {
                        Color::from_rgba8(255, 248, 229, 0.92)
                    })),
                    border: Border {
                        width: 1.0,
                        color: if is_dark {
                            Color::from_rgba8(168, 132, 45, 0.86)
                        } else {
                            Color::from_rgba8(214, 169, 68, 0.96)
                        },
                        radius: 14.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    } else {
        card.into()
    }
}

pub fn should_highlight_pending_permission_tool(
    matched_request_id: Option<&str>,
    current_request_id: Option<&str>,
    matches_permission_tool: bool,
) -> bool {
    matches_permission_tool
        || matched_request_id.zip(current_request_id).is_some_and(
            |(matched_request_id, current_request_id)| matched_request_id == current_request_id,
        )
}
