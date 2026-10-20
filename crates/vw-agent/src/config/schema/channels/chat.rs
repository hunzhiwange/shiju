//! 聊天通道配置聚合与展示适配。
//!
//! 本模块复用 `vw_config_types` 中的聊天通道 schema，并补充代理侧设置界面需要的
//! 枚举能力：把每个可配置通道包装为统一的 `ConfigHandle`，供 UI 或配置扫描逻辑
//! 展示名称、描述和启用状态。

pub use vw_config_types::channels::chat::{ChannelsConfig, default_channel_message_timeout_secs};

use crate::app::agent::config::traits::ChannelConfig;
use crate::app::agent::config::traits::ConfigHandle;
use vw_config_types::channels::other::QQReceiveMode;
use vw_config_types::channels::{WeChatConfig, WeComConfig};

impl ChannelConfig for WeComConfig {
    fn name() -> &'static str {
        "WeCom"
    }

    fn desc() -> &'static str {
        "WeCom Bot"
    }
}

impl ChannelConfig for WeChatConfig {
    fn name() -> &'static str {
        "WeChat"
    }

    fn desc() -> &'static str {
        "WeChat Bot"
    }
}

/// 聊天通道配置的代理侧扩展方法。
///
/// 该 trait 不改变共享配置 schema，只在代理 crate 内提供“列出可展示通道”的能力。
/// 返回值中的布尔值表示对应通道是否在当前配置中启用。
pub trait ChannelsConfigExt {
    /// 列出除 webhook 外的所有聊天通道配置句柄。
    ///
    /// # 返回值
    ///
    /// 返回 `(ConfigHandle, enabled)` 列表。`ConfigHandle` 提供通道名称和描述，
    /// `enabled` 表示当前 `ChannelsConfig` 是否包含该通道配置。
    fn channels_except_webhook(&self) -> Vec<(Box<dyn ConfigHandle>, bool)>;

    /// 列出所有聊天通道配置句柄，包括 webhook。
    ///
    /// # 返回值
    ///
    /// 返回 `(ConfigHandle, enabled)` 列表，顺序与设置界面的展示顺序保持一致。
    fn channels(&self) -> Vec<(Box<dyn ConfigHandle>, bool)>;
}

impl ChannelsConfigExt for ChannelsConfig {
    #[rustfmt::skip]
    fn channels_except_webhook(&self) -> Vec<(Box<dyn ConfigHandle>, bool)> {
        // 这里显式枚举各通道，保持 UI 展示顺序可审计；QQ 只有 websocket 模式
        // 才作为常规接收通道展示，避免把其他接收模式误标为已启用。
        let channels: Vec<(Box<dyn ConfigHandle>, bool)> = vec![
            (
                Box::new(ConfigWrapper::new(self.telegram.as_ref())),
                self.telegram.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.discord.as_ref())),
                self.discord.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.slack.as_ref())),
                self.slack.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.mattermost.as_ref())),
                self.mattermost.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.imessage.as_ref())),
                self.imessage.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.signal.as_ref())),
                self.signal.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.whatsapp.as_ref())),
                self.whatsapp.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.linq.as_ref())),
                self.linq.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.wati.as_ref())),
                self.wati.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.nextcloud_talk.as_ref())),
                self.nextcloud_talk.is_some(),
            ),
            #[cfg(not(target_arch = "wasm32"))]
            (
                Box::new(ConfigWrapper::new(self.email.as_ref())),
                self.email.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.irc.as_ref())),
                self.irc.is_some()
            ),
            (
                Box::new(ConfigWrapper::new(self.lark.as_ref())),
                self.lark.as_ref().is_some_and(|lark| !lark.use_feishu),
            ),
            (
                Box::new(ConfigWrapper::new(self.feishu.as_ref())),
                self.feishu
                    .as_ref()
                    .is_some_and(|feishu| feishu.has_required_credentials())
                    || self
                        .feishu_instances
                        .iter()
                        .any(|instance| instance.config.has_required_credentials())
                    || (self.feishu.is_none()
                        && self.lark.as_ref().is_some_and(|lark| {
                            lark.use_feishu && lark.has_required_credentials()
                        })),
            ),
            (
                Box::new(ConfigWrapper::new(self.dingtalk.as_ref())),
                self.dingtalk.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.wecom.as_ref())),
                self.wecom
                    .as_ref()
                    .is_some_and(|wecom| wecom.has_required_credentials()),
            ),
            (
                Box::new(ConfigWrapper::new(self.wechat.as_ref())),
                self.wechat
                    .as_ref()
                    .is_some_and(|wechat| wechat.has_required_credentials()),
            ),
            (
                Box::new(ConfigWrapper::new(self.qq.as_ref())),
                self.qq
                    .as_ref()
                    .is_some_and(|qq| qq.receive_mode == QQReceiveMode::Websocket)
            ),
            (
                Box::new(ConfigWrapper::new(self.nostr.as_ref())),
                self.nostr.is_some(),
            ),
            (
                Box::new(ConfigWrapper::new(self.clawdtalk.as_ref())),
                self.clawdtalk.is_some(),
            ),
        ];
        channels
    }

    fn channels(&self) -> Vec<(Box<dyn ConfigHandle>, bool)> {
        let mut ret = self.channels_except_webhook();
        // webhook 作为特殊入口追加到末尾，便于调用方在需要时单独处理它。
        ret.push((Box::new(ConfigWrapper::new(self.webhook.as_ref())), self.webhook.is_some()));
        ret
    }
}

/// 将具体通道配置类型适配为统一的配置展示句柄。
///
/// 包装器不持有配置值，只通过类型参数读取 `ChannelConfig` 的静态元数据。
/// 这样即使通道未启用，也能在设置界面展示其名称和描述。
pub(crate) struct ConfigWrapper<T: ChannelConfig>(std::marker::PhantomData<T>);

impl<T: ChannelConfig> ConfigWrapper<T> {
    /// 创建指定通道类型的配置句柄包装器。
    ///
    /// # 参数
    ///
    /// - `_`: 当前通道的可选配置，仅用于类型推导；函数不会读取或保存该值。
    ///
    /// # 返回值
    ///
    /// 返回只携带类型信息的 `ConfigWrapper`。
    pub(crate) fn new(_: Option<&T>) -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: ChannelConfig> ConfigHandle for ConfigWrapper<T> {
    fn name(&self) -> &'static str {
        T::name()
    }

    fn desc(&self) -> &'static str {
        T::desc()
    }
}
