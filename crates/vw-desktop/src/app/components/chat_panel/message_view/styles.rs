//! 消息视图样式与基础展示工具。
//!
//! 该模块集中定义消息视图内部复用的尺寸常量、主题色计算和
//! 通用文本展示逻辑，避免主入口文件混入过多样式细节。

use iced::widget::{row, text};
use iced::{Alignment, Color, Element, Theme};

use crate::app::Message;
use crate::app::components::animated_text::neutral_sweep_text_color;
use crate::app::components::status_animation::spinner_frame;

use super::super::utils::chat_secondary_text_color;

pub(super) const MESSAGE_TEXT_SIZE: f32 = 15.0;
pub(super) const MESSAGE_META_TEXT_SIZE: f32 = 13.0;
pub(super) const THINK_STATUS_TEXT_SIZE: f32 = 13.0;
pub(super) const THINK_META_TEXT_SIZE: f32 = 12.0;
pub(super) const COMPACT_ACTION_BUTTON_RADIUS: f32 = 5.0;

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
    let mut content = row![text(spinner_frame(animation_frame)).size(THINK_META_TEXT_SIZE).style(
        |theme: &Theme| iced::widget::text::Style { color: Some(think_block_text_color(theme)) }
    )]
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
    iced::widget::text::LineHeight::Relative(1.62)
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
