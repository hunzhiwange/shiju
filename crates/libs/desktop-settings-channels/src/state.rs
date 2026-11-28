//! 维护系统设置状态及其按领域拆分的派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};
use iced::widget::text_editor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WeComDeliveryMode {
    #[default]
    Qr,
    Manual,
}

#[derive(Clone, Default)]
pub struct WeComDeliveryDraft {
    pub mode: WeComDeliveryMode,
    pub instance_id: Option<String>,
    pub bot_id: String,
    pub secret: String,
    pub clear_secret: bool,
    pub saving: bool,
    pub error: Option<String>,
}

impl std::fmt::Debug for WeComDeliveryDraft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeComDeliveryDraft").field("mode", &self.mode).finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WeComCronNameDraft {
    pub instance: String,
    pub value: String,
    pub saving: bool,
}

#[derive(Clone, Default, serde::Deserialize)]
pub struct WeComCronBinding {
    pub bound: Option<bool>,
    pub connected: Option<bool>,
    pub pairing_code: Option<String>,
    pub pairing_pending: Option<bool>,
}

impl std::fmt::Debug for WeComCronBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeComCronBinding").field("bound", &self.bound).finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelQrFlowState {
    pub channel: Option<String>,
    pub qrcode_image: Option<iced::widget::image::Handle>,
    pub handle: Option<String>,
    pub expires_at: Option<String>,
    pub status: Option<gateway_channel_qr::ChannelQrStatusDto>,
    pub poll_in_flight: bool,
    pub scan_observed: bool,
    pub generation: u64,
    pub credentials_ready: bool,
    pub(crate) instance_id: Option<String>,
    pub(crate) instance_credentials: Option<NamedQrCredentials>,
}

#[derive(Clone)]
pub(crate) enum NamedQrCredentials {
    Qq { app_id: String, app_secret: String },
    WeCom(config_channel_services::WeComConfig),
    WeChat(config_channel_services::WeChatConfig),
}

impl std::fmt::Debug for NamedQrCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NamedQrCredentials([REDACTED])")
    }
}

impl ChannelQrFlowState {
    pub fn close(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.channel = None;
        self.qrcode_image = None;
        self.handle = None;
        self.expires_at = None;
        self.status = None;
        self.poll_in_flight = false;
        self.scan_observed = false;
        self.credentials_ready = false;
        self.instance_id = None;
        self.instance_credentials = None;
    }
}

fn named_instance_field<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = key.strip_prefix(prefix)?.strip_prefix('.')?;
    let (_, field) = rest.split_once('.')?;
    Some(field)
}

fn is_static_multiline_channel_key(key: &str) -> bool {
    matches!(
        key,
        "telegram.allowed_users"
            | "telegram.group_reply.allowed_sender_ids"
            | "discord.allowed_users"
            | "discord.group_reply.allowed_sender_ids"
            | "slack.allowed_users"
            | "slack.group_reply.allowed_sender_ids"
            | "mattermost.allowed_users"
            | "mattermost.group_reply.allowed_sender_ids"
            | "imessage.allowed_contacts"
            | "signal.allowed_from"
            | "whatsapp.allowed_numbers"
            | "linq.allowed_senders"
            | "wati.allowed_numbers"
            | "nextcloud_talk.allowed_users"
            | "email.allowed_senders"
            | "irc.channels"
            | "irc.allowed_users"
            | "lark.allowed_users"
            | "lark.group_reply.allowed_sender_ids"
            | "feishu.allowed_users"
            | "feishu.group_reply.allowed_sender_ids"
            | "dingtalk.allowed_users"
            | "qq.allowed_users"
            | "nostr.relays"
            | "nostr.allowed_pubkeys"
            | "clawdtalk.allowed_destinations"
    )
}

fn is_named_instance_multiline_channel_key(key: &str) -> bool {
    matches!(
        named_instance_field(key, "feishu_instances"),
        Some("allowed_users" | "group_reply.allowed_sender_ids")
    ) || matches!(named_instance_field(key, "dingtalk_instances"), Some("allowed_users"))
        || matches!(named_instance_field(key, "qq_instances"), Some("allowed_users"))
}

fn is_multiline_channel_key(key: &str) -> bool {
    is_static_multiline_channel_key(key) || is_named_instance_multiline_channel_key(key)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelsSettingsTab {
    Global,
    Telegram,
    Discord,
    Slack,
    Mattermost,
    Webhook,
    IMessage,
    Signal,
    WhatsApp,
    Linq,
    Wati,
    NextcloudTalk,
    Email,
    Irc,
    Lark,
    Feishu,
    DingTalk,
    Qq,
    WeCom,
    WeComCron,
    WeChat,
    Nostr,
    ClawdTalk,
}

impl ChannelsSettingsTab {
    pub fn from_channel_id(channel: &str) -> Option<Self> {
        match channel {
            "telegram" => Some(Self::Telegram),
            "discord" => Some(Self::Discord),
            "slack" => Some(Self::Slack),
            "mattermost" => Some(Self::Mattermost),
            "webhook" => Some(Self::Webhook),
            "imessage" => Some(Self::IMessage),
            "signal" => Some(Self::Signal),
            "whatsapp" => Some(Self::WhatsApp),
            "linq" => Some(Self::Linq),
            "wati" => Some(Self::Wati),
            "nextcloud_talk" => Some(Self::NextcloudTalk),
            "email" => Some(Self::Email),
            "irc" => Some(Self::Irc),
            "lark" => Some(Self::Lark),
            "feishu" => Some(Self::Feishu),
            "dingtalk" => Some(Self::DingTalk),
            "qq" => Some(Self::Qq),
            "wecom" => Some(Self::WeCom),
            "wecom_cron" => Some(Self::WeComCron),
            "wechat" => Some(Self::WeChat),
            "nostr" => Some(Self::Nostr),
            "clawdtalk" => Some(Self::ClawdTalk),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "全局",
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
            Self::Mattermost => "Mattermost",
            Self::Webhook => "Webhook",
            Self::IMessage => "iMessage",
            Self::Signal => "Signal",
            Self::WhatsApp => "WhatsApp",
            Self::Linq => "Linq",
            Self::Wati => "WATI",
            Self::NextcloudTalk => "Nextcloud Talk",
            Self::Email => "Email",
            Self::Irc => "IRC",
            Self::Lark => "Lark",
            Self::Feishu => "飞书",
            Self::DingTalk => "钉钉",
            Self::Qq => "QQ",
            Self::WeCom => "企业微信",
            Self::WeComCron => "定时任务企业微信通知",
            Self::WeChat => "微信",
            Self::Nostr => "Nostr",
            Self::ClawdTalk => "ClawdTalk",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelExecutionChoices {
    pub agents: Vec<(String, String)>,
    pub models: Vec<config_agent_profile::AgentModelReference>,
}

#[derive(Debug, Clone)]
/// 表示 ChannelsSettingsState 相关的应用状态或派生数据。
pub struct ChannelsSettingsState {
    pub execution_provider: std::collections::BTreeMap<String, String>,
    pub execution_choices: std::collections::BTreeMap<String, ChannelExecutionChoices>,
    pub execution_request: std::collections::BTreeMap<String, (String, u64)>,
    pub execution_generation: u64,
    pub execution_error: std::collections::BTreeMap<String, String>,
    pub instance_execution:
        std::collections::BTreeMap<String, config_channels::ChannelExecutionConfig>,
    pub instance_default_projects: std::collections::BTreeMap<String, String>,
    pub project_choices: Vec<(String, String)>,
    pub project_choices_error: Option<String>,
    pub project_choices_request: Option<(String, String, u64)>,
    pub project_choices_generation: u64,
    pub project_choices_loading: bool,
    pub project_dir_input: String,
    pub text_inputs: HashMap<String, String>,
    pub text_editors: HashMap<String, text_editor::Content>,
    pub cli: bool,
    pub telegram: Option<config_channel_types::TelegramConfig>,
    pub discord: Option<config_channel_types::DiscordConfig>,
    pub slack: Option<config_channel_types::SlackConfig>,
    pub mattermost: Option<config_channel_types::MattermostConfig>,
    pub webhook: Option<config_channel_types::WebhookConfig>,
    pub imessage: Option<config_channel_types::IMessageConfig>,
    pub signal: Option<config_channel_services::SignalConfig>,
    pub whatsapp: Option<config_channel_services::WhatsAppConfig>,
    pub linq: Option<config_channel_services::LinqConfig>,
    pub wati: Option<config_channel_services::WatiConfig>,
    pub nextcloud_talk: Option<config_channel_services::NextcloudTalkConfig>,
    #[cfg(not(target_arch = "wasm32"))]
    pub email: Option<config_channel_services::EmailConfig>,
    pub irc: Option<config_channel_services::IrcConfig>,
    pub lark: Option<config_channel_services::LarkConfig>,
    pub feishu: Option<config_channel_services::FeishuConfig>,
    pub feishu_instances: Vec<config_channel_services::FeishuInstanceConfig>,
    pub dingtalk: Option<config_channel_services::DingTalkConfig>,
    pub dingtalk_instances: Vec<config_channel_services::DingTalkInstanceConfig>,
    pub qq: Option<config_channel_services::QQConfig>,
    pub qq_instances: Vec<config_channel_services::QQInstanceConfig>,
    pub wecom: Option<config_channel_services::WeComConfig>,
    pub wecom_instances: Vec<config_channel_services::WeComInstanceConfig>,
    pub wecom_webhook: Option<config_channel_services::WeComWebhookConfig>,
    pub wechat: Option<config_channel_services::WeChatConfig>,
    pub wechat_instances: Vec<config_channel_services::WeChatInstanceConfig>,
    pub nostr: Option<config_channel_services::NostrConfig>,
    pub clawdtalk: Option<config_channel_services::ClawdTalkConfig>,
    pub message_timeout_secs: u32,
    pub active_tab: ChannelsSettingsTab,
    /// None identifies the default card; Some identifies a named instance by its row index.
    pub(crate) expanded_instances: HashMap<String, BTreeSet<Option<usize>>>,
    pub save_error: Option<String>,
    pub qr_flow: ChannelQrFlowState,
    pub wecom_delivery: Option<WeComDeliveryDraft>,
    pub(crate) wecom_cron_display_names: std::collections::BTreeMap<String, String>,
    pub(crate) wecom_cron_name: Option<WeComCronNameDraft>,
    pub wecom_cron_bindings: HashMap<String, WeComCronBinding>,
    pub wecom_cron_binding_request: u64,
    pub wecom_cron_binding_requests: HashMap<String, u64>,
}

impl Default for ChannelsSettingsState {
    fn default() -> Self {
        Self {
            instance_default_projects: Default::default(),
            instance_execution: Default::default(),
            execution_provider: Default::default(),
            execution_choices: Default::default(),
            execution_request: Default::default(),
            execution_generation: 0,
            execution_error: Default::default(),
            project_choices: Vec::new(),
            project_choices_error: None,
            project_choices_request: None,
            project_choices_generation: 0,
            project_choices_loading: false,
            project_dir_input: String::new(),
            text_inputs: HashMap::new(),
            text_editors: HashMap::new(),
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
            qq_instances: Vec::new(),
            wecom: None,
            wecom_instances: Vec::new(),
            wecom_webhook: None,
            wechat: None,
            wechat_instances: Vec::new(),
            nostr: None,
            clawdtalk: None,
            message_timeout_secs: 300,
            active_tab: ChannelsSettingsTab::Global,
            expanded_instances: HashMap::new(),
            save_error: None,
            qr_flow: ChannelQrFlowState::default(),
            wecom_delivery: None,
            wecom_cron_display_names: Default::default(),
            wecom_cron_name: None,
            wecom_cron_bindings: HashMap::new(),
            wecom_cron_binding_request: 0,
            wecom_cron_binding_requests: Default::default(),
        }
    }
}

impl ChannelsSettingsState {
    pub(crate) fn named_instance_count(&self, channel: &str) -> usize {
        match channel {
            "feishu" => self.feishu_instances.len(),
            "dingtalk" => self.dingtalk_instances.len(),
            "qq" => self.qq_instances.len(),
            "wecom" => self.wecom_instances.len(),
            "wechat" => self.wechat_instances.len(),
            _ => 0,
        }
    }

    pub(crate) fn instance_cards(&self, channel: &str) -> impl Iterator<Item = Option<usize>> + '_ {
        let wecom = channel == "wecom";
        std::iter::once(None).chain(
            (0..self.named_instance_count(channel))
                .filter(move |&index| {
                    !wecom || !config_channel_services::WeComConfig::is_cron_instance(&self.wecom_instances[index].id)
                })
                .map(Some),
        )
    }

    pub(crate) fn instance_collapsed(&self, channel: &str, index: Option<usize>) -> bool {
        !self.expanded_instances.get(channel).is_some_and(|cards| cards.contains(&index))
    }

    pub(crate) fn remove_instance_card(&mut self, channel: &str, index: usize) {
        if let Some(cards) = self.expanded_instances.get_mut(channel) {
            *cards = cards.iter().filter_map(|card| match *card {
                Some(row) if row == index => None,
                Some(row) if row > index => Some(Some(row - 1)),
                card => Some(card),
            }).collect();
        }
    }

    pub fn wecom_cron_label<'a>(
        &'a self,
        instance: &'a config_channel_services::WeComInstanceConfig,
    ) -> &'a str {
        self.wecom_cron_display_names
            .get(&instance.id)
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(&instance.config.bot_id)
    }

    pub(crate) fn wecom_connection(
        &self,
        instance_id: Option<&str>,
    ) -> Option<&config_channel_services::WeComConfig> {
        match instance_id {
            Some(id) => self
                .wecom_instances
                .iter()
                .find(|instance| instance.id == id)
                .map(|instance| &instance.config),
            None => self.wecom.as_ref(),
        }
    }

    /// 执行 project_dir 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn project_dir(&self) -> Option<PathBuf> {
        let value = self.project_dir_input.trim();
        if value.is_empty() { None } else { Some(PathBuf::from(value)) }
    }

    /// 执行 refresh_text_inputs 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn refresh_text_inputs(&mut self) {
        self.text_inputs.retain(|key, _| {
            !is_multiline_channel_key(key)
                && !["qq_instances.", "wecom_instances.", "wechat_instances."]
                    .iter()
                    .any(|prefix| key.starts_with(*prefix))
        });
        self.text_editors.retain(|key, _| is_static_multiline_channel_key(key));

        let insert_list = |map: &mut HashMap<String, String>, key: &str, values: &[String]| {
            map.insert(key.to_string(), values.join(", "));
        };
        let insert_editor =
            |map: &mut HashMap<String, text_editor::Content>, key: &str, values: &[String]| {
                map.insert(key.to_string(), text_editor::Content::with_text(&values.join("\n")));
            };

        if let Some(cfg) = self.telegram.as_ref() {
            insert_list(&mut self.text_inputs, "telegram.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "telegram.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "telegram.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "telegram.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        if let Some(cfg) = self.discord.as_ref() {
            insert_list(&mut self.text_inputs, "discord.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "discord.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "discord.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "discord.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        if let Some(cfg) = self.slack.as_ref() {
            insert_list(&mut self.text_inputs, "slack.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "slack.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "slack.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "slack.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        if let Some(cfg) = self.mattermost.as_ref() {
            insert_list(&mut self.text_inputs, "mattermost.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "mattermost.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "mattermost.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "mattermost.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        if let Some(cfg) = self.imessage.as_ref() {
            insert_list(&mut self.text_inputs, "imessage.allowed_contacts", &cfg.allowed_contacts);
            insert_editor(
                &mut self.text_editors,
                "imessage.allowed_contacts",
                &cfg.allowed_contacts,
            );
        }
        if let Some(cfg) = self.signal.as_ref() {
            insert_list(&mut self.text_inputs, "signal.allowed_from", &cfg.allowed_from);
            insert_editor(&mut self.text_editors, "signal.allowed_from", &cfg.allowed_from);
        }
        if let Some(cfg) = self.whatsapp.as_ref() {
            insert_list(&mut self.text_inputs, "whatsapp.allowed_numbers", &cfg.allowed_numbers);
            insert_editor(&mut self.text_editors, "whatsapp.allowed_numbers", &cfg.allowed_numbers);
        }
        if let Some(cfg) = self.linq.as_ref() {
            insert_list(&mut self.text_inputs, "linq.allowed_senders", &cfg.allowed_senders);
            insert_editor(&mut self.text_editors, "linq.allowed_senders", &cfg.allowed_senders);
        }
        if let Some(cfg) = self.wati.as_ref() {
            insert_list(&mut self.text_inputs, "wati.allowed_numbers", &cfg.allowed_numbers);
            insert_editor(&mut self.text_editors, "wati.allowed_numbers", &cfg.allowed_numbers);
        }
        if let Some(cfg) = self.nextcloud_talk.as_ref() {
            insert_list(&mut self.text_inputs, "nextcloud_talk.allowed_users", &cfg.allowed_users);
            insert_editor(
                &mut self.text_editors,
                "nextcloud_talk.allowed_users",
                &cfg.allowed_users,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(cfg) = self.email.as_ref() {
            insert_list(&mut self.text_inputs, "email.allowed_senders", &cfg.allowed_senders);
            insert_editor(&mut self.text_editors, "email.allowed_senders", &cfg.allowed_senders);
        }
        if let Some(cfg) = self.irc.as_ref() {
            insert_list(&mut self.text_inputs, "irc.channels", &cfg.channels);
            insert_list(&mut self.text_inputs, "irc.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "irc.channels", &cfg.channels);
            insert_editor(&mut self.text_editors, "irc.allowed_users", &cfg.allowed_users);
        }
        if let Some(cfg) = self.lark.as_ref() {
            insert_list(&mut self.text_inputs, "lark.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "lark.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "lark.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "lark.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        if let Some(cfg) = self.feishu.as_ref() {
            insert_list(&mut self.text_inputs, "feishu.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "feishu.allowed_users", &cfg.allowed_users);
            insert_list(
                &mut self.text_inputs,
                "feishu.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
            insert_editor(
                &mut self.text_editors,
                "feishu.group_reply.allowed_sender_ids",
                cfg.group_reply
                    .as_ref()
                    .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                    .unwrap_or(&[]),
            );
        }
        for (index, instance) in self.feishu_instances.iter().enumerate() {
            let allowed_users_key = format!("feishu_instances.{index}.allowed_users");
            insert_list(&mut self.text_inputs, &allowed_users_key, &instance.config.allowed_users);
            insert_editor(
                &mut self.text_editors,
                &allowed_users_key,
                &instance.config.allowed_users,
            );
            let group_reply_key =
                format!("feishu_instances.{index}.group_reply.allowed_sender_ids");
            let allowed_sender_ids = instance
                .config
                .group_reply
                .as_ref()
                .map(|group_reply| group_reply.allowed_sender_ids.as_slice())
                .unwrap_or(&[]);
            insert_list(&mut self.text_inputs, &group_reply_key, allowed_sender_ids);
            insert_editor(&mut self.text_editors, &group_reply_key, allowed_sender_ids);
        }
        if let Some(cfg) = self.dingtalk.as_ref() {
            insert_list(&mut self.text_inputs, "dingtalk.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "dingtalk.allowed_users", &cfg.allowed_users);
        }
        for (index, instance) in self.dingtalk_instances.iter().enumerate() {
            let key = format!("dingtalk_instances.{index}.allowed_users");
            insert_list(&mut self.text_inputs, &key, &instance.config.allowed_users);
            insert_editor(&mut self.text_editors, &key, &instance.config.allowed_users);
        }
        if let Some(cfg) = self.qq.as_ref() {
            insert_list(&mut self.text_inputs, "qq.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "qq.allowed_users", &cfg.allowed_users);
        }
        for (index, instance) in self.qq_instances.iter().enumerate() {
            let key = format!("qq_instances.{index}.allowed_users");
            insert_list(&mut self.text_inputs, &key, &instance.config.allowed_users);
            insert_editor(&mut self.text_editors, &key, &instance.config.allowed_users);
        }
        if let Some(cfg) = self.nostr.as_ref() {
            insert_list(&mut self.text_inputs, "nostr.relays", &cfg.relays);
            insert_list(&mut self.text_inputs, "nostr.allowed_pubkeys", &cfg.allowed_pubkeys);
            insert_editor(&mut self.text_editors, "nostr.relays", &cfg.relays);
            insert_editor(&mut self.text_editors, "nostr.allowed_pubkeys", &cfg.allowed_pubkeys);
        }
        if let Some(cfg) = self.clawdtalk.as_ref() {
            insert_list(
                &mut self.text_inputs,
                "clawdtalk.allowed_destinations",
                &cfg.allowed_destinations,
            );
            insert_editor(
                &mut self.text_editors,
                "clawdtalk.allowed_destinations",
                &cfg.allowed_destinations,
            );
        }
    }
}


#[path = "channel_projects.rs"]
mod channel_projects;
#[cfg(test)]
#[path = "channels_tests.rs"]
mod channels_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelQrFlowSnapshot {
    pub channel: Option<String>,
    pub generation: u64,
    pub poll_in_flight: bool,
    pub credentials_ready: bool,
    pub configured: bool,
}

impl ChannelQrFlowSnapshot {
    pub fn accepts_result(&self, channel: &str, generation: u64) -> bool {
        self.channel.as_deref() == Some(channel) && self.generation == generation
    }
}

impl ChannelsSettingsState {
    pub fn qr_flow_snapshot(&self) -> ChannelQrFlowSnapshot {
        let flow = &self.qr_flow;
        let configured = match flow.channel.as_deref() {
            Some("dingtalk") => self.dingtalk.is_some(),
            Some("feishu") => self.feishu.is_some(),
            Some("qq") => self.qq.is_some(),
            Some("wecom") => self.wecom.is_some(),
            Some("wechat") => self.wechat.is_some(),
            _ => false,
        };
        ChannelQrFlowSnapshot {
            channel: flow.channel.clone(),
            generation: flow.generation,
            poll_in_flight: flow.poll_in_flight,
            credentials_ready: flow.credentials_ready,
            configured,
        }
    }
}
