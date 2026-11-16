//! Complete channel page selection owned by the channel domain.
use crate::{
    ViewState, basic_channels, enterprise_channels, extended_channels, messages::Message, overview,
    state::ChannelsSettingsTab,
};
use iced::Element;
pub fn view(app: ViewState<'_>) -> Element<'_, Message> {
    let selected_panel: Element<'_, Message> = match app.channels_settings.active_tab {
        ChannelsSettingsTab::Global => overview::global_settings_content(app),
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
        ChannelsSettingsTab::Email => overview::global_settings_content(app),
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

    overview::view(app, selected_panel, std::convert::identity)
}
