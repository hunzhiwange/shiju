//! 渲染聊天消息分叉目标选择对话框。
//!
//! 本模块只负责把分叉目标选择转成明确的聊天消息，不直接执行会话或 worktree 操作。

use super::message::chat::ForkSessionTarget;
use super::{App, Message, message};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style,
};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

pub(crate) fn with_chat_fork_dialog<'a>(
    app: &App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(msg_idx) = app.chat_fork_dialog_idx else {
        return root_content;
    };
    if app.chat.get(msg_idx).is_none() {
        return root_content;
    }

    let close_message = Message::Chat(message::ChatMessage::CloseForkSessionDialog);
    let local_message = Message::Chat(message::ChatMessage::ForkSessionAt {
        msg_idx,
        target: ForkSessionTarget::Local,
    });
    let worktree_message = Message::Chat(message::ChatMessage::ForkSessionAt {
        msg_idx,
        target: ForkSessionTarget::NewWorktree,
    });

    let card = settings_modal_card(
        column![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("分叉到新会话")).size(18),
            text(
                crate::app::i18n::I18n::new(app.desktop_locale).__("会复制当前消息及之前的聊天历史。选择新工作树时，会先创建隔离 worktree，再把会话复制过去。")
            )
            .size(13)
            .style(settings_muted_text_style),
            button(column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("分叉到新会话")).size(14),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("打开一个新会话，并复制到当前消息为止")).size(12).style(settings_muted_text_style),
            ])
            .width(Length::Fill)
            .padding([12, 14])
            .style(rounded_action_btn_style)
            .on_press(local_message),
            button(column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("分叉到新工作树")).size(14),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("先创建 git worktree，再在其中复制会话")).size(12).style(settings_muted_text_style),
            ])
            .width(Length::Fill)
            .padding([12, 14])
            .style(rounded_action_btn_style)
            .on_press(worktree_message),
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
    .width(Length::Fixed(460.0));

    settings_modal_overlay(Some(root_content), close_message, card)
}

#[cfg(test)]
#[path = "chat_fork_tests.rs"]
mod chat_fork_tests;
