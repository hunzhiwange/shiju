//! 通道领域库的桌面消息适配。
use crate::app::{App, Message};
use iced::Element;
pub(super) fn telegram_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::telegram_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn discord_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::discord_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn slack_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::slack_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn mattermost_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::mattermost_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn webhook_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::webhook_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn imessage_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::imessage_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn signal_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::basic_channels::signal_panel(super::view_state(app)).map(super::map_message) }
