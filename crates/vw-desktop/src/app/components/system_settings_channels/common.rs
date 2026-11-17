//! 通道领域库的桌面消息适配。
#[cfg(test)]
pub(super) fn group_reply_mode_value(group_reply: &Option<GroupReplyConfig>) -> &'static str { desktop_settings_channels::common::group_reply_mode_value(group_reply) }
#[cfg(test)]
pub(super) fn lark_receive_mode_value(mode: LarkReceiveMode) -> &'static str { desktop_settings_channels::common::lark_receive_mode_value(mode) }
#[cfg(test)]
pub(super) fn qq_receive_mode_value(mode: QQReceiveMode) -> &'static str { desktop_settings_channels::common::qq_receive_mode_value(mode) }
#[cfg(test)]
pub(super) fn is_multiline_list_key(key: &str) -> bool { desktop_settings_channels::common::is_multiline_list_key(key) }
#[cfg(test)]
pub(super) fn localized_label(label: &'static str) -> &'static str { desktop_settings_channels::common::localized_label(label) }
#[cfg(test)]
pub(super) fn multiline_placeholder(placeholder: &'static str) -> &'static str { desktop_settings_channels::common::multiline_placeholder(placeholder) }
#[cfg(test)]
pub(super) fn text_row<'a>(
    app: &'a App,
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    key: &'static str,
    secure: bool,
) -> Element<'a, Message> { desktop_settings_channels::common::text_row(super::view_state(app), label, placeholder, value, key, secure).map(super::map_message) }
#[cfg(test)]
pub(super) fn text_row_with_key<'a>(
    app: &'a App,
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    key: String,
    secure: bool,
) -> Element<'a, Message> { desktop_settings_channels::common::text_row_with_key(super::view_state(app), label, placeholder, value, key, secure).map(super::map_message) }
#[cfg(test)]
pub(super) fn bool_row<'a>(
    locale: DesktopLocale,
    label: &'static str,
    checked: bool,
    description: &'static str,
    key: &'static str,
) -> Element<'a, Message> { desktop_settings_channels::common::bool_row(locale, label, checked, description, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn bool_row_with_key<'a>(
    locale: DesktopLocale,
    label: &'static str,
    checked: bool,
    description: &'static str,
    key: String,
) -> Element<'a, Message> { desktop_settings_channels::common::bool_row_with_key(locale, label, checked, description, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn number_row<'a>(
    locale: DesktopLocale,
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    key: &'static str,
) -> Element<'a, Message> { desktop_settings_channels::common::number_row(locale, label, value, min, max, suffix, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn number_row_with_hint<'a>(
    app: &'a App,
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    key: &'static str,
    (description, hint): (&'static str, &'static str),
) -> Element<'a, Message> { desktop_settings_channels::common::number_row_with_hint(super::view_state(app), label, value, min, max, key, (description, hint)).map(super::map_message) }
#[cfg(test)]
pub(super) fn number_row_with_key<'a>(
    locale: DesktopLocale,
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    key: String,
) -> Element<'a, Message> { desktop_settings_channels::common::number_row_with_key(locale, label, value, min, max, suffix, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn pick_row<'a>(
    locale: DesktopLocale,
    label: &'static str,
    selected: &'static str,
    options: &'static [LabeledOption],
    key: &'static str,
) -> Element<'a, Message> { desktop_settings_channels::common::pick_row(locale, label, selected, options, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn pick_row_with_key<'a>(
    locale: DesktopLocale,
    label: &'static str,
    selected: &'static str,
    options: &'static [LabeledOption],
    key: String,
) -> Element<'a, Message> { desktop_settings_channels::common::pick_row_with_key(locale, label, selected, options, key).map(super::map_message) }
#[cfg(test)]
pub(super) fn instance_title_row<'a>(
    locale: DesktopLocale,
    title: String,
    channel: &'static str,
    index: usize,
) -> Element<'a, Message> { desktop_settings_channels::common::instance_title_row(locale, title, channel, index).map(super::map_message) }
#[cfg(test)]
pub(super) fn add_instance_row<'a>(
    channel: &'static str,
    label: &'static str,
) -> Element<'a, Message> { desktop_settings_channels::common::add_instance_row(channel, label).map(super::map_message) }
#[cfg(test)]
pub(super) fn hint_row<'a>(message: &'a str) -> Element<'a, Message> { desktop_settings_channels::common::hint_row(message).map(super::map_message) }
#[cfg(test)]
pub(super) fn panel<'a>(
    app: &'a App,
    id: &'static str,
    title: &'static str,
    description: &'static str,
    enabled: bool,
    body: Element<'a, Message>,
) -> Element<'a, Message> { desktop_settings_channels::common::panel_mapped(super::view_state(app), id, title, description, enabled, body, super::map_message) }
#[cfg(test)]
pub(super) fn enabled_channels(app: &App) -> Vec<&'static str> { desktop_settings_channels::common::enabled_channels(super::view_state(app)) }
#[cfg(test)]
pub(super) fn default_project_selector(app: &App, key: String) -> Element<'_, Message> { desktop_settings_channels::common::default_project_selector(super::view_state(app), key).map(super::map_message) }

#[cfg(test)]
use config_channel_services::LarkReceiveMode;
#[cfg(test)]
use config_channel_services::QQReceiveMode;
#[cfg(test)]
use config_channel_types::GroupReplyConfig;
#[cfg(test)]
use config_ui::DesktopLocale;
#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use desktop_settings_channels::common::LabeledOption;
#[cfg(test)]
use iced::Element;
#[cfg(test)]
pub(super) use desktop_settings_channels::common::GROUP_REPLY_MODE_OPTIONS;
#[cfg(test)]
pub(super) use desktop_settings_channels::common::LARK_RECEIVE_MODE_OPTIONS;
#[cfg(test)]
pub(super) use desktop_settings_channels::common::QQ_RECEIVE_MODE_OPTIONS;
