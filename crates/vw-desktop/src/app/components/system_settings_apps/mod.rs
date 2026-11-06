pub(crate) mod catalog;
#[cfg(test)]
mod catalog_tests;
mod details;
#[cfg(test)]
mod details_tests;
pub(crate) mod drag;
#[cfg(test)]
mod drag_tests;
pub(crate) mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
mod list;
pub(crate) mod local;
mod presentation;
#[cfg(test)]
mod presentation_tests;
mod styles;
pub(crate) mod uninstall;

use super::system_settings_common::*;
use crate::app::{
    Message,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::AppsCatalogState,
};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

fn message(value: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(value))
}

pub(crate) fn view(state: &AppsCatalogState) -> Element<'_, Message> {
    list::view(state)
}

pub(crate) fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !state.connection_open {
        return launch::overlay(
            state,
            preferences::overlay(state, lifecycle::overlay(&state.lifecycle, &state.host, base)),
        );
    }
    let close = message(AppsCatalogMessage::ConnectionClose);
    let host = text_input("管理员登记的主机 ID", &state.host)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|value| message(AppsCatalogMessage::HostInput(value)));
    let token = text_input("应用管理令牌", &state.token)
        .secure(true)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|value| message(AppsCatalogMessage::TokenInput(value)));
    let connect =
        button(text("加载应用")).padding([8, 16]).style(primary_action_btn_style).on_press_maybe(
            (!state.host.trim().is_empty() && !state.token.is_empty())
                .then_some(message(AppsCatalogMessage::Refresh)),
        );
    let cancel = button(text("取消"))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(close.clone());
    let card = settings_modal_card(
        column![
            row![
                container(text("选择执行主机").size(22)).width(Length::Fill),
                settings_close_button(close.clone())
            ],
            text("主机 ID 与令牌由网关管理员提供；令牌仅在本次会话内存中保留。")
                .size(12)
                .style(settings_muted_text_style),
            text("执行主机").size(13),
            host,
            text("应用管理令牌").size(13),
            token,
            row![container(text("")).width(Length::Fill), cancel, connect].spacing(10),
        ]
        .spacing(12),
    )
    .width(Length::Fixed(520.0));
    settings_modal_overlay(Some(base), close, card)
}

#[cfg(test)]
mod uninstall_tests;

pub(crate) mod preferences;
#[cfg(test)]
mod preferences_tests;

pub(crate) mod launch;
#[cfg(test)]
mod launch_tests;
