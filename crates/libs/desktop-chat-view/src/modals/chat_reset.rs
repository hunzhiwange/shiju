//! 渲染聊天消息重置确认对话框。
//!
//! 本模块只负责让重置路径显式可见，具体历史截断与网关同步由聊天消息处理层执行。


#[derive(Debug, Clone)]
pub enum Event { CloseResetMenu, ResetSessionToMessage { msg_idx: usize } }

use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_header, settings_modal_overlay_with_escape, settings_muted_text_style,
};
use desktop_i18n::I18n;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn with_chat_reset_dialog<'a, Message: Clone + 'static>(
    msg_idx: Option<usize>, chat_len: usize, locale: config_ui::DesktopLocale,
    emit: impl Fn(Event) -> Message + Copy + 'a,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(msg_idx) = msg_idx else {
        return root_content;
    };
    if msg_idx >= chat_len {
        return root_content;
    }

    let close_message = emit(Event::CloseResetMenu);
    let confirm_message = emit(Event::ResetSessionToMessage { msg_idx });

    let i18n = I18n::new(locale);
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
