//! 通道领域视图的桌面消息适配。
use crate::app::{App, Message};
use iced::Element;
pub(super) fn lark_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::lark_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn feishu_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::feishu_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn dingtalk_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::dingtalk_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn qq_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::qq_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn wecom_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::wecom_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn wechat_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::wechat_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn nostr_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::nostr_panel(super::view_state(app)).map(super::map_message) }
pub(super) fn clawdtalk_panel(app: &App) -> Element<'_, Message> { desktop_settings_channels::enterprise_channels::clawdtalk_panel(super::view_state(app)).map(super::map_message) }
