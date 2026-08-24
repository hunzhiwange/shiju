//! 消息视图样式与基础展示工具。
//!
//! 该模块集中定义消息视图内部复用的尺寸常量、主题色计算和
//! 通用文本展示逻辑，避免主入口文件混入过多样式细节。

use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::Message;
use crate::app::components::animated_text::neutral_sweep_text_color;
use crate::app::components::chat_panel::tool_text_support::chat_text_line_height;

use super::super::global_status::{
    COMPACT_ACTIVITY_TEXT_SIZE, compact_running_activity_indicator,
};
use super::super::utils::chat_secondary_text_color;

pub(super) const MESSAGE_TEXT_SIZE: f32 = 15.0;
pub(super) const MESSAGE_META_TEXT_SIZE: f32 = 13.0;
pub(super) const PROCESSING_DURATION_TEXT_SIZE: f32 = 14.0;
pub(super) const THINK_STATUS_TEXT_SIZE: f32 = COMPACT_ACTIVITY_TEXT_SIZE;
pub(super) const THINK_META_TEXT_SIZE: f32 = 12.0;
pub(super) const COMPACT_ACTION_BUTTON_RADIUS: f32 = 5.0;
pub(super) const ASSISTANT_BLOCK_SPACING: f32 = 12.0;
pub(super) const ASSISTANT_HEADER_BODY_SPACING: f32 = 10.0;
pub(super) const MARKDOWN_BLOCK_SPACING: f32 = 8.0;
pub(super) const THINK_BLOCK_CONTENT_SPACING: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MessageSurfaceContract {
    pub(crate) padding: Padding,
    pub(crate) width: Length,
    pub(crate) max_width: Option<f32>,
    pub(crate) corner_radius: f32,
    pub(crate) has_background: bool,
}

pub(crate) fn message_surface_contract(is_user: bool) -> MessageSurfaceContract {
    if is_user {
        MessageSurfaceContract {
            padding: Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 },
            width: Length::Fill,
            max_width: None,
            corner_radius: 12.0,
            has_background: true,
        }
    } else {
        MessageSurfaceContract {
            padding: Padding { top: 2.0, right: 2.0, bottom: 2.0, left: 0.0 },
            width: Length::Fill,
            max_width: None,
            corner_radius: 0.0,
            has_background: false,
        }
    }
}

pub(crate) fn message_surface_style(
    theme: &Theme,
    is_user: bool,
    is_permission_target: bool,
) -> container::Style {
    let contract = message_surface_contract(is_user);
    if !is_user {
        if is_permission_target {
            let is_dark = is_dark_theme(theme);
            return container::Style {
                background: Some(Background::Color(if is_dark {
                    Color::from_rgba8(61, 49, 19, 0.34)
                } else {
                    Color::from_rgba8(255, 248, 229, 0.96)
                })),
                border: Border {
                    width: 1.0,
                    color: if is_dark {
                        Color::from_rgba8(168, 132, 45, 0.82)
                    } else {
                        Color::from_rgba8(214, 169, 68, 0.96)
                    },
                    radius: 14.0.into(),
                },
                ..Default::default()
            };
        }
        return container::Style {
            background: None,
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: contract.corner_radius.into(),
            },
            shadow: iced::Shadow::default(),
            ..Default::default()
        };
    }

    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.055)
    } else {
        Color::from_rgba8(246, 247, 249, 1.0)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.07)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            radius: contract.corner_radius.into(),
        },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

pub(super) fn message_action_icon_color(
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

pub(super) fn message_action_button_style(
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
            radius: if compact_square {
                COMPACT_ACTION_BUTTON_RADIUS.into()
            } else {
                8.0.into()
            },
        },
        text_color: super::super::utils::chat_secondary_muted_text_color(theme),
        ..Default::default()
    }
}

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub(super) fn think_block_text_color(theme: &Theme) -> Color {
    chat_secondary_text_color(theme)
}

pub(super) fn subtle_card_shadow(theme: &Theme) -> iced::Shadow {
    iced::Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.10 } else { 0.03 }),
        offset: iced::Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    }
}

pub(super) fn thinking_status_text<'a>(
    label: &str,
    now_ms: u64,
    animation_frame: usize,
) -> Element<'a, Message> {
    let char_count = label.chars().count().max(1);
    let mut content = row![compact_running_activity_indicator(animation_frame)]
        .spacing(4)
        .align_y(Alignment::Center);

    for (char_idx, character) in label.chars().enumerate() {
        content = content.push(text(character.to_string()).size(THINK_STATUS_TEXT_SIZE).style(
            move |theme: &Theme| iced::widget::text::Style {
                color: Some(neutral_sweep_text_color(
                    theme,
                    think_block_text_color(theme),
                    now_ms,
                    char_idx,
                    char_count,
                    true,
                )),
            },
        ));
    }

    content.into()
}

pub(super) fn message_text_line_height() -> iced::widget::text::LineHeight {
    chat_text_line_height()
}

pub(super) fn message_body_text_color(theme: &Theme, is_user: bool) -> Color {
    if is_user {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.96 } else { 0.90 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.91 } else { 0.88 })
    }
}

pub(super) fn message_meta_text_color(theme: &Theme, is_user: bool) -> Color {
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

pub(super) fn processing_duration_text_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.52 })
}
