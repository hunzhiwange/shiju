//! 渲染聊天消息重置确认对话框。
//!
//! 本模块只负责让重置路径显式可见，具体历史截断与网关同步由聊天消息处理层执行。

use super::{App, Message, message};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_header, settings_modal_overlay_with_escape, settings_muted_text_style,
};
use crate::app::i18n::I18n;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

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
    let confirm_message = Message::Chat(message::ChatMessage::ResetSessionToMessage { msg_idx });

    let i18n = I18n::new(app.desktop_locale);
    let card = settings_modal_card(
        column![
            settings_modal_header(
                text(i18n.__("重置到此点?")).size(20),
                settings_close_button(close_message.clone()),
            ),
            text(i18n.__("会先立即截断本地聊天历史并保存，然后同步网关历史。"))
                .size(13)
                .style(settings_muted_text_style),
            row![
                Space::new().width(Length::Fill),
                button(text(i18n.__("取消")).size(13))
                    .style(rounded_action_btn_style)
                    .padding([9, 18])
                    .on_press(close_message.clone()),
                button(text(i18n.__("确认")).size(13))
                    .style(primary_action_btn_style)
                    .padding([9, 18])
                    .on_press(confirm_message),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(14),
    )
    .width(Length::Fixed(480.0));

    settings_modal_overlay_with_escape(root_content, close_message, container(card).padding(12))
}

#[cfg(test)]
#[path = "chat_reset_tests.rs"]
mod chat_reset_tests;
