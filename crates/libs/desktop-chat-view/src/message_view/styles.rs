//! 消息视图样式与基础展示工具。
//!
//! 该模块集中定义消息视图内部复用的尺寸常量、主题色计算和
//! 通用文本展示逻辑，避免主入口文件混入过多样式细节。

use iced::widget::{container, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use super::text::Message;
use desktop_ui_controls::animated_text::neutral_sweep_text_color;
use desktop_chat_tools::tool_text_support::chat_text_line_height;
use desktop_ui_controls::status_animation::activity_spinner;

use super::super::global_status::COMPACT_ACTIVITY_TEXT_SIZE;
use crate::presentation::chat_secondary_text_color;

pub const MESSAGE_TEXT_SIZE: f32 = 15.0;
pub const MESSAGE_META_TEXT_SIZE: f32 = 13.0;
pub const PROCESSING_DURATION_TEXT_SIZE: f32 = 14.0;
pub const THINK_STATUS_TEXT_SIZE: f32 = COMPACT_ACTIVITY_TEXT_SIZE;
pub const THINK_META_TEXT_SIZE: f32 = 12.0;
#[cfg(test)]
pub use crate::message_view::layout::COMPACT_ACTION_BUTTON_RADIUS;
pub const ASSISTANT_BLOCK_SPACING: f32 = 12.0;
pub const ASSISTANT_HEADER_BODY_SPACING: f32 = 10.0;
pub const MARKDOWN_BLOCK_SPACING: f32 = 8.0;
pub const THINK_BLOCK_CONTENT_SPACING: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MessageSurfaceContract {
    pub padding: Padding,
    pub width: Length,
    pub max_width: Option<f32>,
    pub corner_radius: f32,
    pub has_background: bool,
}

pub fn message_surface_contract(is_user: bool) -> MessageSurfaceContract {
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

pub fn message_surface_style(
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

pub use crate::message_view::layout::message_action_icon_color;

pub use crate::message_view::layout::message_action_button_style;

pub fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn compaction_status_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgb8(0xDC, 0xE3, 0xEC)
    } else {
        Color::from_rgb8(0x3D, 0x49, 0x57)
    }
}

pub fn compaction_status_card_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            Color::from_rgb8(0x25, 0x2A, 0x31)
        } else {
            Color::from_rgb8(0xEE, 0xF1, 0xF5)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark_theme(theme) {
                Color::from_rgb8(0x3D, 0x45, 0x50)
            } else {
                Color::from_rgb8(0xD7, 0xDC, 0xE3)
            },
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub fn think_block_text_color(theme: &Theme) -> Color {
    chat_secondary_text_color(theme)
}

pub use crate::message_view::error_style::subtle_card_shadow;

pub fn thinking_status_text<'a>(
    label: &str,
    now_ms: u64,
    animation_frame: usize,
) -> Element<'a, Message> {
    let char_count = label.chars().count().max(1);
    let spinner = activity_spinner(animation_frame, THINK_STATUS_TEXT_SIZE)
        .style(|theme: &Theme, _status| svg::Style { color: Some(think_block_text_color(theme)) });
    let mut content = row![spinner].spacing(4).align_y(Alignment::Center);

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

pub fn message_text_line_height() -> iced::widget::text::LineHeight {
    chat_text_line_height()
}

pub fn message_body_text_color(theme: &Theme, is_user: bool) -> Color {
    if is_user {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.96 } else { 0.90 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.91 } else { 0.88 })
    }
}

pub use crate::message_view::layout::message_meta_text_color;

pub fn processing_duration_text_color(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.52 })
}
