//! 其他聊天与消息渠道配置的 agent 侧适配。
//!
//! 本模块集中为 Signal、WhatsApp、IRC 等非核心渠道补齐 `ChannelConfig`
//! 元数据。实际字段定义仍来自 `vw_config_types`，这里仅负责稳定的用户可见名称与描述。

pub use vw_config_types::channels::other::*;

use crate::app::agent::config::traits::ChannelConfig;

/// 为 Signal 渠道配置提供展示名称和说明。
impl ChannelConfig for SignalConfig {
    fn name() -> &'static str {
        "Signal"
    }
    fn desc() -> &'static str {
        "An open-source, encrypted messaging service"
    }
}

/// 为 WhatsApp Business Cloud API 配置提供展示名称和说明。
impl ChannelConfig for WhatsAppConfig {
    fn name() -> &'static str {
        "WhatsApp"
    }
    fn desc() -> &'static str {
        "Business Cloud API"
    }
}

/// 为 Linq API 渠道配置提供展示名称和说明。
impl ChannelConfig for LinqConfig {
    fn name() -> &'static str {
        "Linq"
    }
    fn desc() -> &'static str {
        "iMessage/RCS/SMS via Linq API"
    }
}

/// 为 WATI Business API 渠道配置提供展示名称和说明。
impl ChannelConfig for WatiConfig {
    fn name() -> &'static str {
        "WATI"
    }
    fn desc() -> &'static str {
        "WhatsApp via WATI Business API"
    }
}

/// 为 Nextcloud Talk 渠道配置提供展示名称和说明。
impl ChannelConfig for NextcloudTalkConfig {
    fn name() -> &'static str {
        "NextCloud Talk"
    }
    fn desc() -> &'static str {
        "NextCloud Talk platform"
    }
}

/// 为 IRC over TLS 渠道配置提供展示名称和说明。
impl ChannelConfig for IrcConfig {
    fn name() -> &'static str {
        "IRC"
    }
    fn desc() -> &'static str {
        "IRC over TLS"
    }
}

/// 为钉钉 Stream Mode 渠道配置提供展示名称和说明。
impl ChannelConfig for DingTalkConfig {
    fn name() -> &'static str {
        "DingTalk"
    }
    fn desc() -> &'static str {
        "DingTalk Stream Mode"
    }
}

/// 为 QQ 官方机器人渠道配置提供展示名称和说明。
impl ChannelConfig for QQConfig {
    fn name() -> &'static str {
        "QQ Official"
    }
    fn desc() -> &'static str {
        "Tencent QQ Bot"
    }
}

/// 为 Nostr 私信渠道配置提供展示名称和说明。
impl ChannelConfig for NostrConfig {
    fn name() -> &'static str {
        "Nostr"
    }
    fn desc() -> &'static str {
        "Nostr DMs"
    }
}
