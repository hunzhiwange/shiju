//! 消息气泡与页脚的通用布局；宿主注入跨域控件与动作。
use super::contracts::message_footer_padding;
use crate::presentation::is_dark_theme;
use iced::widget::{Space, button, column, container, row, text as iced_text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

pub fn approval_badge_view<'a, M: Clone + 'static>(
    label: String,
    on_press: Option<M>,
    font: iced::Font,
) -> Element<'a, M> {
    let badge = container(iced_text(label).size(11).font(font).style(|theme: &Theme| {
        iced::widget::text::Style {
            color: Some(if is_dark_theme(theme) {
                Color::from_rgba8(255, 235, 177, 0.96)
            } else {
                Color::from_rgba8(142, 103, 16, 1.0)
            }),
        }
    }))
    .padding([2, 8])
    .style(|theme: &Theme| {
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
            ..Default::default()
        }
    });

    if let Some(on_press) = on_press {
        button(badge)
            .padding(0)
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: None,
                border: Border::default(),
                shadow: iced::Shadow::default(),
                ..Default::default()
            })
            .on_press(on_press)
            .into()
    } else {
        badge.into()
    }
}

pub fn message_footer<'a, M: 'a>(
    meta: Option<Element<'a, M>>,
    badge: Option<Element<'a, M>>,
    actions: Vec<Element<'a, M>>,
    is_user: bool,
) -> Element<'a, M> {
    let mut line = row![].spacing(8).align_y(Alignment::Center);
    if is_user {
        line = line.push(container(Space::new()).width(Length::Fill));
    }
    if let Some(meta) = meta {
        line = line.push(meta);
    }
    if let Some(badge) = badge {
        line = line.push(badge);
    }
    for action in actions {
        line = line.push(action);
    }
    column![line].spacing(6).into()
}

pub fn message_layout<'a, M: 'a>(
    bubble: Element<'a, M>,
    footer: Element<'a, M>,
    is_user: bool,
) -> Element<'a, M> {
    let alignment = if is_user {
        iced::alignment::Horizontal::Right
    } else {
        iced::alignment::Horizontal::Left
    };
    let footer = container(footer)
        .padding(message_footer_padding(is_user))
        .width(Length::Fill)
        .align_x(alignment);
    column![
        container(bubble).width(Length::Fill).align_x(alignment),
        row![footer].align_y(Alignment::Center)
    ]
    .spacing(4)
    .into()
}

use crate::presentation::{chat_secondary_text_color, icon_svg};
use desktop_assets::assets::Icon;
use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
const MESSAGE_META_TEXT_SIZE: f32 = 13.0;
pub const COMPACT_ACTION_BUTTON_RADIUS: f32 = 5.0;

pub fn icon_action_button<'a, M: Clone + 'static>(
    icon: Icon,
    tooltip: Element<'a, M>,
    on_press: M,
    highlighted: bool,
    compact_square: bool,
    is_user: bool,
) -> Element<'a, M> {
    // 紧凑动作图标（复制、分叉、重置）与相邻元信息文字同高，避免图标显得突兀偏大。
    let icon_size = if compact_square { MESSAGE_META_TEXT_SIZE } else { 12.0 };
    let compact_button_size = MESSAGE_META_TEXT_SIZE;
    let btn = button(
        icon_svg(icon).width(Length::Fixed(icon_size)).height(Length::Fixed(icon_size)).style(
            move |theme: &Theme, status| svg::Style {
                color: Some(message_action_icon_color(
                    theme,
                    status,
                    highlighted,
                    compact_square,
                    is_user,
                )),
            },
        ),
    )
    .width(if compact_square { Length::Fixed(compact_button_size) } else { Length::Fixed(24.0) })
    .height(if compact_square { Length::Fixed(compact_button_size) } else { Length::Fixed(22.0) })
    .padding(0)
    .style(move |theme: &Theme, status| message_action_button_style(theme, status, compact_square))
    .on_press(on_press);

    Tooltip::new(btn, tooltip, TooltipPosition::Top).gap(6).into()
}
pub fn message_action_icon_color(
    theme: &Theme,
    status: svg::Status,
    highlighted: bool,
    compact_square: bool,
    is_user: bool,
) -> Color {
    if highlighted {
        theme.extended_palette().success.base.color
    } else if compact_square {
        match status {
            svg::Status::Idle => message_meta_text_color(theme, is_user),
            svg::Status::Hovered => chat_secondary_text_color(theme),
        }
    } else {
        chat_secondary_text_color(theme)
    }
}
pub fn message_action_button_style(
    theme: &Theme,
    status: button::Status,
    compact_square: bool,
) -> button::Style {
    let (background, show_background) = match status {
        button::Status::Hovered => {
            if is_dark_theme(theme) {
                (Color::from_rgba8(31, 33, 38, 0.94), true)
            } else {
                (Color::from_rgb8(0xF1, 0xF3, 0xF6), true)
            }
        }
        button::Status::Pressed => {
            if is_dark_theme(theme) {
                (Color::from_rgba8(36, 38, 44, 0.98), true)
            } else {
                (Color::from_rgb8(0xE8, 0xEC, 0xF1), true)
            }
        }
        _ => (Color::TRANSPARENT, false),
    };
    button::Style {
        background: show_background.then_some(Background::Color(background)),
        border: Border {
            width: if show_background { 1.0 } else { 0.0 },
            color: if is_dark_theme(theme) {
                Color::from_rgba8(52, 56, 63, 0.9)
            } else {
                Color::from_rgba8(218, 223, 230, 1.0)
            },
            radius: if compact_square { COMPACT_ACTION_BUTTON_RADIUS.into() } else { 8.0.into() },
        },
        text_color: desktop_chat_tools::card_style::chat_secondary_muted_text_color(theme),
        ..Default::default()
    }
}
pub fn message_meta_text_color(theme: &Theme, is_user: bool) -> Color {
    if is_user {
        if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.50)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.72)
        }
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.46 } else { 0.48 })
    }
}
