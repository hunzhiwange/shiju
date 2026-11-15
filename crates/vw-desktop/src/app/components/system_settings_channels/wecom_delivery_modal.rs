//! 企业微信弹窗的桌面消息适配。
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_settings_channels::wecom_delivery_modal::overlay(super::view_state(app), base, super::map_message)
}
