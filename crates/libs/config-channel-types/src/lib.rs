//! 常用聊天通道配置类型。
//!
//! 本模块承载 Telegram、Discord、Slack、Mattermost 等常用聊天平台的配置结构，
//! 以及这些平台共用的群聊回复策略类型。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 流式输出模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    /// 关闭流式输出。
    #[default]
    Off,
    /// 以部分增量形式输出。
    Partial,
}

fn default_draft_update_interval_ms() -> u64 {
    1000
}

/// 群聊回复模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupReplyMode {
    /// 仅在被提及时回复。
    MentionOnly,
    /// 对所有消息都进行回复判定。
    AllMessages,
}

impl GroupReplyMode {
    #[must_use]
    pub fn requires_mention(self) -> bool {
        matches!(self, Self::MentionOnly)
    }
}

/// 群聊回复细粒度配置。
///
/// 用于统一表达“是否需要提及”和“允许哪些发送者触发”这两个条件。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct GroupReplyConfig {
    /// 显式配置的群聊回复模式。
    #[serde(default)]
    pub mode: Option<GroupReplyMode>,

    /// 允许触发群聊回复的发送者 ID 列表。
    #[serde(default)]
    pub allowed_sender_ids: Vec<String>,
}

/// 解析最终生效的群聊回复模式。
///
/// 优先使用 `group_reply.mode`，若缺失则回退到旧字段 `mention_only`，最后使用
/// 传入的默认值。
pub fn resolve_group_reply_mode(
    group_reply: Option<&GroupReplyConfig>,
    legacy_mention_only: Option<bool>,
    default_mode: GroupReplyMode,
) -> GroupReplyMode {
    if let Some(mode) = group_reply.and_then(|cfg| cfg.mode) {
        return mode;
    }
    if let Some(mention_only) = legacy_mention_only {
        return if mention_only {
            GroupReplyMode::MentionOnly
        } else {
            GroupReplyMode::AllMessages
        };
    }
    default_mode
}

/// 克隆群聊回复允许的发送者列表。
pub fn clone_group_reply_allowed_sender_ids(group_reply: Option<&GroupReplyConfig>) -> Vec<String> {
    group_reply.map(|cfg| cfg.allowed_sender_ids.clone()).unwrap_or_default()
}

/// Telegram 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TelegramConfig {
    /// 机器人 token。
    pub bot_token: String,
    /// 允许交互的用户列表。
    pub allowed_users: Vec<String>,
    /// 流式输出模式。
    #[serde(default)]
    pub stream_mode: StreamMode,
    /// 草稿更新间隔，单位为毫秒。
    #[serde(default = "default_draft_update_interval_ms")]
    pub draft_update_interval_ms: u64,
    /// 收到新消息时是否中断当前响应。
    #[serde(default)]
    pub interrupt_on_new_message: bool,
    /// 兼容旧配置：是否仅在被提及时回复。
    #[serde(default)]
    pub mention_only: bool,
    /// 新版群聊回复配置。
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
    /// 可选的 Telegram API 基础地址。
    #[serde(default)]
    pub base_url: Option<String>,
}

impl TelegramConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(
            self.group_reply.as_ref(),
            Some(self.mention_only),
            GroupReplyMode::AllMessages,
        )
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}

/// Discord 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordConfig {
    /// 机器人 token。
    pub bot_token: String,
    /// 可选的服务器 ID。
    pub guild_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub listen_to_bots: bool,
    #[serde(default)]
    pub mention_only: bool,
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
}

impl DiscordConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(
            self.group_reply.as_ref(),
            Some(self.mention_only),
            GroupReplyMode::AllMessages,
        )
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}

/// Slack 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SlackConfig {
    /// Bot token。
    pub bot_token: String,
    /// 可选的 App-level token。
    pub app_token: Option<String>,
    /// 可选的频道 ID。
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
}

impl SlackConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(self.group_reply.as_ref(), None, GroupReplyMode::AllMessages)
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}

/// Mattermost 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MattermostConfig {
    /// Mattermost 服务地址。
    pub url: String,
    /// 机器人 token。
    pub bot_token: String,
    /// 可选的频道 ID。
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub thread_replies: Option<bool>,
    #[serde(default)]
    pub mention_only: Option<bool>,
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
}

impl MattermostConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(
            self.group_reply.as_ref(),
            Some(self.mention_only.unwrap_or(false)),
            GroupReplyMode::AllMessages,
        )
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}

/// Webhook 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebhookConfig {
    /// Webhook 监听端口。
    pub port: u16,
    /// 用于签名校验的可选密钥。
    pub secret: Option<String>,
}

/// iMessage 通道配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IMessageConfig {
    /// 允许接入的联系人列表。
    pub allowed_contacts: Vec<String>,
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
