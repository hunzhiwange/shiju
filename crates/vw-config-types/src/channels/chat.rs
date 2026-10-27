use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use super::clawdtalk::ClawdTalkConfig;
#[cfg(not(target_arch = "wasm32"))]
use super::email::EmailConfig;
use super::lark::{FeishuConfig, FeishuInstanceConfig, LarkConfig};
use super::wechat::WeChatConfig;
use super::wecom::WeComConfig;
use super::{
    other::{
        DingTalkConfig, DingTalkInstanceConfig, IrcConfig, LinqConfig, NextcloudTalkConfig,
        NostrConfig, QQConfig, SignalConfig, WatiConfig, WhatsAppConfig,
    },
    types::{
        DiscordConfig, IMessageConfig, MattermostConfig, SlackConfig, TelegramConfig, WebhookConfig,
    },
};

/// 默认的单条通道消息处理超时时间，单位为秒。
pub fn default_channel_message_timeout_secs() -> u64 {
    300
}

pub const DEFAULT_CHANNEL_INSTANCE_ID: &str = "default";

/// 已解析的通道实例 profile ref。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelInstanceProfile {
    pub channel_type: &'static str,
    pub instance_id: String,
    pub profile_ref: String,
}

/// 通道实例 profile ref 解析错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelInstanceProfileError {
    channel_type: &'static str,
    instance_id: String,
    reason: ChannelInstanceProfileErrorReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelInstanceProfileErrorReason {
    Empty,
    Unsafe,
    Duplicate,
}

impl ChannelInstanceProfileError {
    fn empty_id(channel_type: &'static str) -> Self {
        Self {
            channel_type,
            instance_id: String::new(),
            reason: ChannelInstanceProfileErrorReason::Empty,
        }
    }

    fn unsafe_id(channel_type: &'static str, instance_id: &str) -> Self {
        Self {
            channel_type,
            instance_id: instance_id.to_string(),
            reason: ChannelInstanceProfileErrorReason::Unsafe,
        }
    }

    fn duplicate_id(channel_type: &'static str, instance_id: &str) -> Self {
        Self {
            channel_type,
            instance_id: instance_id.to_string(),
            reason: ChannelInstanceProfileErrorReason::Duplicate,
        }
    }
}

impl fmt::Display for ChannelInstanceProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reason {
            ChannelInstanceProfileErrorReason::Empty => {
                write!(f, "{} channel instance id must not be empty", self.channel_type)
            }
            ChannelInstanceProfileErrorReason::Unsafe => write!(
                f,
                "{} channel instance id '{}' must not contain ':', '/' or '\\'",
                self.channel_type, self.instance_id
            ),
            ChannelInstanceProfileErrorReason::Duplicate => write!(
                f,
                "{} channel instance id '{}' is duplicated",
                self.channel_type, self.instance_id
            ),
        }
    }
}

impl std::error::Error for ChannelInstanceProfileError {}

/// 通道配置总表。
///
/// 该结构把所有可选消息通道集中到一个位置，调用方可以按需启用其中任意子通道。
/// 某个字段为 `Some` 通常表示对应通道已配置并可初始化。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChannelsConfig {
    /// 解密失败的运行期渠道实例；不持久化，不改变原始密文。
    #[serde(skip)]
    pub unavailable_channels: BTreeSet<String>,
    /// 是否启用 CLI 通道。
    pub cli: bool,
    /// Telegram 通道配置。
    pub telegram: Option<TelegramConfig>,
    /// Discord 通道配置。
    pub discord: Option<DiscordConfig>,
    /// Slack 通道配置。
    pub slack: Option<SlackConfig>,
    /// Mattermost 通道配置。
    pub mattermost: Option<MattermostConfig>,
    /// Webhook 通道配置。
    pub webhook: Option<WebhookConfig>,
    /// iMessage 通道配置。
    pub imessage: Option<IMessageConfig>,
    /// Signal 通道配置。
    pub signal: Option<SignalConfig>,
    /// WhatsApp 通道配置。
    pub whatsapp: Option<WhatsAppConfig>,
    /// Linq 通道配置。
    pub linq: Option<LinqConfig>,
    /// Wati 通道配置。
    pub wati: Option<WatiConfig>,
    /// Nextcloud Talk 通道配置。
    pub nextcloud_talk: Option<NextcloudTalkConfig>,
    /// 邮件通道配置，仅在非 WASM 平台可用。
    #[cfg(not(target_arch = "wasm32"))]
    pub email: Option<EmailConfig>,
    /// IRC 通道配置。
    pub irc: Option<IrcConfig>,
    /// Lark 通道配置。
    pub lark: Option<LarkConfig>,
    /// Feishu 通道配置。
    pub feishu: Option<FeishuConfig>,
    /// 命名 Feishu 通道实例配置。
    #[serde(default)]
    pub feishu_instances: Vec<FeishuInstanceConfig>,
    /// DingTalk 通道配置。
    pub dingtalk: Option<DingTalkConfig>,
    /// 命名 DingTalk 通道实例配置。
    #[serde(default)]
    pub dingtalk_instances: Vec<DingTalkInstanceConfig>,
    /// QQ 通道配置。
    pub qq: Option<QQConfig>,
    /// 企业微信 AI 机器人通道配置。
    pub wecom: Option<WeComConfig>,
    /// 微信 iLink 机器人通道配置。
    pub wechat: Option<WeChatConfig>,
    /// Nostr 通道配置。
    pub nostr: Option<NostrConfig>,
    /// ClawdTalk 通道配置。
    pub clawdtalk: Option<ClawdTalkConfig>,
    /// 单条消息允许的处理超时，单位为秒。
    #[serde(default = "default_channel_message_timeout_secs")]
    pub message_timeout_secs: u64,
    /// 实例稳定标识（channel_type:instance_id）到注册项目 ID 的默认绑定。
    #[serde(default)]
    pub instance_default_projects: BTreeMap<String, String>,
    /// 通道运行时可选的项目目录覆盖值。
    pub project_dir: Option<PathBuf>,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            unavailable_channels: BTreeSet::new(),
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            mattermost: None,
            webhook: None,
            imessage: None,
            signal: None,
            whatsapp: None,
            linq: None,
            wati: None,
            nextcloud_talk: None,
            #[cfg(not(target_arch = "wasm32"))]
            email: None,
            irc: None,
            lark: None,
            feishu: None,
            feishu_instances: Vec::new(),
            dingtalk: None,
            dingtalk_instances: Vec::new(),
            qq: None,
            wecom: None,
            wechat: None,
            nostr: None,
            clawdtalk: None,
            message_timeout_secs: default_channel_message_timeout_secs(),
            project_dir: None,
            instance_default_projects: BTreeMap::new(),
        }
    }
}

impl ChannelsConfig {
    /// 创建可实例化的配置快照，保留源配置中的故障凭据供用户修复。
    pub fn available_channels(&self) -> Self {
        let mut next = self.clone();
        for profile in &self.unavailable_channels {
            match profile.as_str() {
                "telegram:default" => next.telegram = None,
                "discord:default" => next.discord = None,
                "slack:default" => next.slack = None,
                "mattermost:default" => next.mattermost = None,
                "webhook:default" => next.webhook = None,
                "whatsapp:default" => next.whatsapp = None,
                "linq:default" => next.linq = None,
                "nextcloud_talk:default" => next.nextcloud_talk = None,
                "irc:default" => next.irc = None,
                "lark:default" => next.lark = None,
                "dingtalk:default" => next.dingtalk = None,
                "qq:default" => next.qq = None,
                "wecom:default" => next.wecom = None,
                "wechat:default" => next.wechat = None,
                "nostr:default" => next.nostr = None,
                "clawdtalk:default" => next.clawdtalk = None,
                _ => {}
            }
        }
        next.instance_default_projects
            .retain(|profile, _| !self.unavailable_channels.contains(profile));
        next.feishu_instances.retain(|instance| {
            !self.unavailable_channels.contains(&format!("feishu:{}", instance.id))
        });
        next.dingtalk_instances.retain(|instance| {
            !self.unavailable_channels.contains(&format!("dingtalk:{}", instance.id))
        });
        next
    }

    /// 校验后更新绑定；错误不会改变原配置。项目存在性与租户权限由运行时验证。
    pub fn set_instance_default_project(
        &mut self,
        key: &str,
        project_id: &str,
    ) -> Result<(), String> {
        self.validate_instance_default_project(key, project_id)?;
        self.instance_default_projects.insert(key.to_string(), project_id.to_string());
        Ok(())
    }

    /// 拒绝持久化配置中的未知实例、非法键与路径形式的项目值。
    pub fn validate_instance_default_projects(&self) -> Result<(), String> {
        for (key, project_id) in &self.instance_default_projects {
            self.validate_instance_default_project(key, project_id)?;
        }
        Ok(())
    }

    fn validate_instance_default_project(&self, key: &str, project_id: &str) -> Result<(), String> {
        if project_id.trim().is_empty()
            || project_id != project_id.trim()
            || project_id.contains([':', '/', '\\'])
            || project_id == "."
            || project_id == ".."
        {
            return Err(
                "channel default project must be a nonempty registered project ID, not a path"
                    .into(),
            );
        }
        let profiles = self.resolve_channel_instance_profiles().map_err(|err| err.to_string())?;
        if !profiles.iter().any(|profile| profile.profile_ref == key) {
            return Err(format!("unknown channel instance binding: {key}"));
        }
        Ok(())
    }

    /// 解析已配置的通道实例，并返回稳定 profile ref。
    pub fn resolve_channel_instance_profiles(
        &self,
    ) -> Result<Vec<ChannelInstanceProfile>, ChannelInstanceProfileError> {
        let mut profiles = Vec::new();

        push_instance_profiles(
            &mut profiles,
            "feishu",
            self.feishu.is_some() || self.lark.as_ref().is_some_and(|config| config.use_feishu),
            self.feishu_instances.iter().map(|instance| instance.id.as_str()),
        )?;
        push_instance_profiles(
            &mut profiles,
            "dingtalk",
            self.dingtalk.is_some(),
            self.dingtalk_instances.iter().map(|instance| instance.id.as_str()),
        )?;

        for (channel_type, configured) in [
            ("telegram", self.telegram.is_some()),
            ("discord", self.discord.is_some()),
            ("slack", self.slack.is_some()),
            ("mattermost", self.mattermost.is_some()),
            ("webhook", self.webhook.is_some()),
            ("imessage", self.imessage.is_some()),
            ("signal", self.signal.is_some()),
            ("whatsapp", self.whatsapp.is_some()),
            ("linq", self.linq.is_some()),
            ("wati", self.wati.is_some()),
            ("nextcloud_talk", self.nextcloud_talk.is_some()),
            ("irc", self.irc.is_some()),
            ("qq", self.qq.is_some()),
            ("wecom", self.wecom.is_some()),
            ("wechat", self.wechat.is_some()),
            ("nostr", self.nostr.is_some()),
            ("clawdtalk", self.clawdtalk.is_some()),
            ("lark", self.lark.as_ref().is_some_and(|config| !config.use_feishu)),
            #[cfg(not(target_arch = "wasm32"))]
            ("email", self.email.is_some()),
        ] {
            if configured {
                push_instance_profiles(&mut profiles, channel_type, true, std::iter::empty())?;
            }
        }

        Ok(profiles)
    }
}

fn push_instance_profiles<'a>(
    profiles: &mut Vec<ChannelInstanceProfile>,
    channel_type: &'static str,
    has_legacy_instance: bool,
    instance_ids: impl Iterator<Item = &'a str>,
) -> Result<(), ChannelInstanceProfileError> {
    let mut seen = BTreeSet::new();

    if has_legacy_instance {
        push_instance_profile(profiles, &mut seen, channel_type, DEFAULT_CHANNEL_INSTANCE_ID)?;
    }

    for instance_id in instance_ids {
        push_instance_profile(profiles, &mut seen, channel_type, instance_id)?;
    }

    Ok(())
}

fn push_instance_profile(
    profiles: &mut Vec<ChannelInstanceProfile>,
    seen: &mut BTreeSet<String>,
    channel_type: &'static str,
    instance_id: &str,
) -> Result<(), ChannelInstanceProfileError> {
    validate_instance_id(channel_type, instance_id)?;

    if !seen.insert(instance_id.to_string()) {
        return Err(ChannelInstanceProfileError::duplicate_id(channel_type, instance_id));
    }

    profiles.push(ChannelInstanceProfile {
        channel_type,
        instance_id: instance_id.to_string(),
        profile_ref: format!("{channel_type}:{instance_id}"),
    });
    Ok(())
}

fn validate_instance_id(
    channel_type: &'static str,
    instance_id: &str,
) -> Result<(), ChannelInstanceProfileError> {
    if instance_id.trim().is_empty() {
        return Err(ChannelInstanceProfileError::empty_id(channel_type));
    }

    if instance_id.contains([':', '/', '\\']) {
        return Err(ChannelInstanceProfileError::unsafe_id(channel_type, instance_id));
    }

    Ok(())
}
