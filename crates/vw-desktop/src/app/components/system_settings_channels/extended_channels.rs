//! 通道领域视图的桌面消息适配。
#[cfg(test)]
pub(super) fn whatsapp_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::whatsapp_panel(super::view_state(app)).map(super::map_message) }
#[cfg(test)]
pub(super) fn linq_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::linq_panel(super::view_state(app)).map(super::map_message) }
#[cfg(test)]
pub(super) fn wati_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::wati_panel(super::view_state(app)).map(super::map_message) }
#[cfg(test)]
pub(super) fn nextcloud_talk_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::nextcloud_talk_panel(super::view_state(app)).map(super::map_message) }
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn email_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::email_panel(super::view_state(app)).map(super::map_message) }
#[cfg(test)]
pub(super) fn irc_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::extended_channels::irc_panel(super::view_state(app)).map(super::map_message) }

#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use iced::Element;
