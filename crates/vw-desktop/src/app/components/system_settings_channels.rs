//! 系统设置中 channels 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

mod basic_channels;
mod common;
mod enterprise_channels;
pub(crate) mod wecom_delivery_modal;
pub(crate) use wecom_delivery_modal::view_overlays;
mod extended_channels;
#[cfg(test)]
mod wecom_delivery_modal_tests;

use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::ChannelsSettingsTab;
use crate::app::{App, Message};
use iced::Element;
fn view_state(app: &App) -> desktop_settings_channels::ViewState<'_> {
    desktop_settings_channels::ViewState {
        channels_settings: &app.channels_settings,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
    }
}
fn map_message(message: desktop_settings_channels::messages::Message) -> Message {
    use desktop_settings_channels::messages as domain;
    match message {
        domain::Message::None => Message::None,
        domain::Message::CopyCode(value) => Message::CopyCode(value),
        domain::Message::Append(value) => {
            crate::app::components::text_editor_context_menu::text_context_message(
                desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(value),
            )
        }
        domain::Message::OpenUrlExternal(url) => {
            Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))
        }
        domain::Message::Settings(domain::SettingsMessage::Channels(message)) => {
            Message::Settings(crate::app::message::settings::SettingsMessage::Channels(message))
        }
    }
}
pub(super) fn channel_tab_descriptors() -> Vec<SettingsTabDescriptor<ChannelsSettingsTab>> {
    desktop_settings_channels::overview::channel_tab_descriptors()
}
pub fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_channels::view::view(view_state(app)).map(map_message)
}

#[cfg(test)]
#[path = "system_settings_channels_ui_refresh_17_tests.rs"]
mod ui_refresh_17_tests;
