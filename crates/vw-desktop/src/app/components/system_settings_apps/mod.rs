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
    i18n::I18n,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::AppsCatalogState,
};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Theme};

fn message(value: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(value))
}

pub(crate) fn view(state: &AppsCatalogState) -> Element<'_, Message> {
    list::view(state)
}

pub(crate) fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    i18n: I18n,
    theme: &Theme,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !state.connection_open {
        return launch::overlay(
            state,
            i18n,
            theme,
            preferences::overlay(
                state,
                i18n,
                theme,
                lifecycle::overlay(&state.lifecycle, &state.host, base),
            ),
        );
    }
    let close = message(AppsCatalogMessage::ConnectionClose);
    let host = settings_text_input("管理员登记的主机 ID", &state.host)
        .on_input(|value| message(AppsCatalogMessage::HostInput(value)));
    let token = settings_text_input("应用管理令牌", &state.token)
        .secure(true)
        .on_input(|value| message(AppsCatalogMessage::TokenInput(value)));
    let connect = button(text("加载应用").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(
            (!state.host.trim().is_empty() && !state.token.is_empty())
                .then_some(message(AppsCatalogMessage::Refresh)),
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

#[cfg(test)]
mod uninstall_tests;

pub(crate) mod preferences;
#[cfg(test)]
pub(crate) mod preferences_tests;

pub(crate) mod launch;
#[cfg(test)]
mod launch_tests;
