//! Authorized host connection form.
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
#[derive(Clone)]
pub enum ConnectionMessage { Close, HostInput(String), TokenInput(String), Refresh }
pub fn overlay<'a, M: Clone + 'static>(host: &'a str, token: &'a str, base: Element<'a, M>, message: impl Fn(ConnectionMessage) -> M + Copy + 'static) -> Element<'a, M> {
    let ready = !host.trim().is_empty() && !token.is_empty();
    let close = message(ConnectionMessage::Close);
    let host = settings_text_input("管理员登记的主机 ID", host)
        .on_input(move |value| message(ConnectionMessage::HostInput(value)));
    let token = settings_text_input("应用管理令牌", token)
        .secure(true)
        .on_input(move |value| message(ConnectionMessage::TokenInput(value)));
    let connect = button(text("加载应用").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(
            ready
                .then_some(message(ConnectionMessage::Refresh)),
        );
    let cancel = button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(close.clone());
    let header = settings_modal_header(
        settings_page_heading(
            "选择执行主机",
            "主机 ID 与令牌由网关管理员提供；令牌仅在本次会话内存中保留。",
        ),
        settings_close_button(close.clone()),
    );
    let fields = column![
        column![settings_field_label("执行主机"), host].spacing(6),
        column![settings_field_label("应用管理令牌"), token].spacing(6),
    ]
    .spacing(14);
    let footer = row![iced::widget::Space::new().width(Length::Fill), cancel, connect]
        .spacing(10)
        .align_y(iced::Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(footer.into()))
        .width(Length::Fill)
        .max_width(520.0)
        .max_height(342.0);
    let card_with_gutter = container(card).padding(12).center(Length::Fill);
    settings_modal_overlay(Some(base), close, card_with_gutter)
}
