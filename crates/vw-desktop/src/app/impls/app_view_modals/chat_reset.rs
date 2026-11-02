//! 渲染聊天消息重置确认对话框。
//!
//! 本模块只负责让重置路径显式可见，具体历史截断与网关同步由聊天消息处理层执行。

use super::{App, Message, message};
use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style,
};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Color, Element, Length, Theme};

fn action_button_title_text_style(_theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Color::WHITE) }
}

fn action_button_detail_text_style(_theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.82)) }
}

pub(crate) fn with_chat_reset_dialog<'a>(
    app: &App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(msg_idx) = app.chat_reset_menu_idx else {
        return root_content;
    };
    if app.chat.get(msg_idx).is_none() {
        return root_content;
    }

    let close_message = Message::Chat(message::ChatMessage::CloseResetMenu);
    let keep_files_message =
        Message::Chat(message::ChatMessage::ResetSessionToMessage { msg_idx, revert_code: false });
    let revert_files_message =
        Message::Chat(message::ChatMessage::ResetSessionToMessage { msg_idx, revert_code: true });

    let card = settings_modal_card(
        column![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("重置到此点?")).size(18),
            text(
                crate::app::i18n::I18n::new(app.desktop_locale).__("会先立即截断本地聊天历史并保存，然后同步网关历史。选择回滚代码会额外还原此点之后的文件改动。")
            )
            .size(13)
            .style(settings_muted_text_style),
            button(column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("仅重置会话历史")).size(14).style(action_button_title_text_style),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("保留当前文件内容，只把聊天回到此点"))
                    .size(12)
                    .style(action_button_detail_text_style),
            ])
            .width(Length::Fill)
            .padding([12, 14])
            .style(primary_action_btn_style)
            .on_press(keep_files_message),
            button(column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("回滚代码并重置")).size(14).style(action_button_title_text_style),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("同时撤销此点之后产生的文件改动"))
                    .size(12)
                    .style(action_button_detail_text_style),
            ])
            .width(Length::Fill)
            .padding([12, 14])
            .style(danger_action_btn_style)
            .on_press(revert_files_message),
            row![
                Space::new().width(Length::Fill),
                button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("取消")).size(13))
                    .style(rounded_action_btn_style)
                    .padding([9, 18])
                    .on_press(close_message.clone()),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .width(Length::Fixed(480.0));

    settings_modal_overlay(Some(root_content), close_message, card)
}

#[cfg(test)]
#[path = "chat_reset_tests.rs"]
mod chat_reset_tests;
