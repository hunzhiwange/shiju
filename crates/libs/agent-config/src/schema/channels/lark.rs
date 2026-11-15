//! Lark 与飞书渠道配置的适配层。
//!
//! 该模块复用 `config_channel_services` 中的渠道结构体，并在 agent 侧为它们补齐
//! `ChannelConfig` 元数据，使配置 UI 和渠道注册逻辑可以用统一接口展示名称与说明。

pub use config_channel_services::lark::*;

use crate::traits::ChannelConfig;

/// 为 Lark Bot 配置提供渠道展示元数据。
impl ChannelConfig for LarkConfig {
    fn name() -> &'static str {
        "Lark"
    }
    fn desc() -> &'static str {
        "Lark Bot"
    }
}

/// 为飞书 Bot 配置提供渠道展示元数据。
impl ChannelConfig for FeishuConfig {
    fn name() -> &'static str {
        "Feishu"
    }
    fn desc() -> &'static str {
        "Feishu Bot"
    }
}
