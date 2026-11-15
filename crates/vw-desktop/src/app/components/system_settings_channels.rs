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

#[cfg(test)]
mod enterprise_channels_tests;
#[cfg(test)]
mod extended_channels_tests;

use crate::app::{App, Message};
use crate::app::state::ChannelsSettingsTab;
use iced::Element;
#[cfg(test)] use iced::widget::text;
#[cfg(test)] use desktop_i18n::settings_extensions::text as tr;
#[cfg(test)] use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
#[cfg(test)] use self::common::enabled_channels;
use config_ui::DesktopLocale;
use crate::app::components::system_settings_search::SettingsTabDescriptor;
#[cfg(test)] use desktop_settings_channels::overview::{CHANNEL_TAB_COLUMNS, CHINA_CHANNEL_CARD_COLUMNS, CHANNEL_TAB_GROUPS, ChannelTabGroup, channel_tab_group, china_channel_icon};
fn view_state(app: &App) -> desktop_settings_channels::ViewState<'_> { desktop_settings_channels::ViewState { channels_settings: &app.channels_settings, desktop_locale: app.desktop_locale, app_theme: &app.app_theme } }
fn map_message(message: desktop_settings_channels::messages::Message) -> Message { use desktop_settings_channels::messages as domain; match message { domain::Message::None => Message::None, domain::Message::Settings(domain::SettingsMessage::Channels(message)) => Message::Settings(crate::app::message::settings::SettingsMessage::Channels(message)), } }
pub(super) fn channel_tab_descriptors() -> Vec<SettingsTabDescriptor<ChannelsSettingsTab>> { desktop_settings_channels::overview::channel_tab_descriptors(super) }
fn global_settings_content(app: &App) -> Element<'_, Message> { desktop_settings_channels::overview::global_settings_content(view_state(app)).map(map_message) }
#[cfg(test)]
fn china_channel_card(
    locale: DesktopLocale,
    descriptor: &SettingsTabDescriptor<ChannelsSettingsTab>,
    active_tab: ChannelsSettingsTab,
) -> Element<'static, Message> { desktop_settings_channels::overview::china_channel_card(locale, descriptor, active_tab).map(map_message) }
#[cfg(test)]
fn china_channel_cards(
    locale: DesktopLocale,
    descriptors: &[&SettingsTabDescriptor<ChannelsSettingsTab>],
    active_tab: ChannelsSettingsTab,
) -> Element<'static, Message> { desktop_settings_channels::overview::china_channel_cards(locale, descriptors, active_tab).map(map_message) }
#[cfg(test)]
fn channel_tabs(
    locale: DesktopLocale,
    active_tab: ChannelsSettingsTab,
) -> Element<'static, Message> { desktop_settings_channels::overview::channel_tabs(locale, active_tab).map(map_message) }
pub fn view(app: &App) -> Element<'_, Message> {
    let selected_panel: Element<'_, Message> = match app.channels_settings.active_tab {
        ChannelsSettingsTab::Global => global_settings_content(app),
        ChannelsSettingsTab::Telegram => basic_channels::telegram_panel(app),
        ChannelsSettingsTab::Discord => basic_channels::discord_panel(app),
        ChannelsSettingsTab::Slack => basic_channels::slack_panel(app),
        ChannelsSettingsTab::Mattermost => basic_channels::mattermost_panel(app),
        ChannelsSettingsTab::Webhook => basic_channels::webhook_panel(app),
        ChannelsSettingsTab::IMessage => basic_channels::imessage_panel(app),
        ChannelsSettingsTab::Signal => basic_channels::signal_panel(app),
        ChannelsSettingsTab::WhatsApp => extended_channels::whatsapp_panel(app),
        ChannelsSettingsTab::Linq => extended_channels::linq_panel(app),
        ChannelsSettingsTab::Wati => extended_channels::wati_panel(app),
        ChannelsSettingsTab::NextcloudTalk => extended_channels::nextcloud_talk_panel(app),
        #[cfg(not(target_arch = "wasm32"))]
        ChannelsSettingsTab::Email => extended_channels::email_panel(app),
        #[cfg(target_arch = "wasm32")]
        ChannelsSettingsTab::Email => global_settings_content(app),
        ChannelsSettingsTab::Irc => extended_channels::irc_panel(app),
        ChannelsSettingsTab::Lark => enterprise_channels::lark_panel(app),
        ChannelsSettingsTab::Feishu => enterprise_channels::feishu_panel(app),
        ChannelsSettingsTab::DingTalk => enterprise_channels::dingtalk_panel(app),
        ChannelsSettingsTab::Qq => enterprise_channels::qq_panel(app),
        ChannelsSettingsTab::WeCom => enterprise_channels::wecom_panel(app),
        ChannelsSettingsTab::WeChat => enterprise_channels::wechat_panel(app),
        ChannelsSettingsTab::Nostr => enterprise_channels::nostr_panel(app),
        ChannelsSettingsTab::ClawdTalk => enterprise_channels::clawdtalk_panel(app),
    };

 desktop_settings_channels::overview::view(view_state(app), selected_panel, map_message)
}
#[cfg(test)]
#[path = "system_settings_channels_tests.rs"]
mod system_settings_channels_tests;

#[cfg(test)]
#[path = "system_settings_channels_ui_refresh_17_tests.rs"]
mod ui_refresh_17_tests;
