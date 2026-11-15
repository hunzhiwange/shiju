//! 常用消息渠道配置的 agent 侧适配。
//!
//! 这里重新导出共享配置类型，并为 Telegram、Discord、Slack 等常用渠道实现
//! `ChannelConfig`。该层不保存渠道密钥，也不修改字段语义，只提供注册和展示所需的元数据。

pub use config_channel_types::*;

use crate::traits::ChannelConfig;

/// 为 Telegram Bot 配置提供渠道展示元数据。
impl ChannelConfig for TelegramConfig {
    fn name() -> &'static str {
        "Telegram"
    }
    fn desc() -> &'static str {
        "connect your bot"
    }
}

/// 为 Discord Bot 配置提供渠道展示元数据。
impl ChannelConfig for DiscordConfig {
    fn name() -> &'static str {
        "Discord"
    }
    fn desc() -> &'static str {
        "connect your bot"
    }
}

/// 为 Slack Bot 配置提供渠道展示元数据。
impl ChannelConfig for SlackConfig {
    fn name() -> &'static str {
        "Slack"
    }
    fn desc() -> &'static str {
        "connect your bot"
    }
}

/// 为 Mattermost Bot 配置提供渠道展示元数据。
impl ChannelConfig for MattermostConfig {
    fn name() -> &'static str {
        "Mattermost"
    }
    fn desc() -> &'static str {
        "connect to your bot"
    }
}

/// 为通用 Webhook 渠道配置提供展示元数据。
impl ChannelConfig for WebhookConfig {
    fn name() -> &'static str {
        "Webhook"
    }
    fn desc() -> &'static str {
        "HTTP endpoint"
    }
}

/// 为 macOS iMessage 渠道配置提供展示元数据。
impl ChannelConfig for IMessageConfig {
    fn name() -> &'static str {
        "iMessage"
    }
    fn desc() -> &'static str {
        "macOS only"
    }
}
