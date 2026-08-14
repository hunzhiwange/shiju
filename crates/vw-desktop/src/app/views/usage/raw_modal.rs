//! 用量步骤原始记录弹窗。
//!
//! 展示完整的原始 LLM 载荷，并提供一键复制。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay, settings_muted_text_style,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, scrollable, text, text::Wrapping};
use iced::{Alignment, Element, Length};
use std::hash::{Hash, Hasher};

fn hash_copy_text(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn vertical_scrollbar() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
    )
}

/// 构建使用量步骤原始记录弹窗。
///
/// 当 `app.usage_step_raw_modal` 为空时返回 `None`。
pub fn view<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    let modal = app.usage_step_raw_modal.as_ref()?;
    let close_message = Message::View(message::ViewMessage::UsageStepRawClose);
    let copy_label = if app.last_copied_code_hash == Some(hash_copy_text(&modal.content)) {
        "✓"
    } else {
        "复制全部"
    };
    let copy_button = button(text(copy_label).size(13))
        .on_press(Message::CopyCode(modal.content.clone()))
        .padding([10, 18])
        .width(Length::Fixed(112.0))
        .style(rounded_action_btn_style);

    let body = container(
        scrollable(text(&modal.content).size(12).width(Length::Fill).wrapping(Wrapping::None))
            .direction(vertical_scrollbar()),
    )
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|theme: &iced::Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(iced::Background::Color(palette.background.weak.color)),
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 10.0.into(),
            },
            ..Default::default()
        }
    });

    let card = settings_modal_card(
        column![
            row![
                column![
                    text(format!("原始记录 · 步骤 {}", modal.step_index)).size(18),
                    text("完整 LLM 原始载荷，可复制。").size(12).style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .align_y(Alignment::Start),
            body,
            container(
                row![
                    copy_button,
                    button(text("关闭").size(13))
                        .on_press(close_message.clone())
                        .padding([10, 18])
                        .style(primary_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(14)
        .height(Length::Fill),
    )
    .width(Length::Fixed(780.0))
    .height(Length::Fixed(560.0));

    Some(settings_modal_overlay(None, close_message, card))
}

#[cfg(test)]
#[path = "raw_modal_tests.rs"]
mod raw_modal_tests;
