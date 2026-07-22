//! 维护系统设置状态及其按领域拆分的派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

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
            | "matrix.allowed_users"
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
    matches!(named_instance_field(key, "matrix_instances"), Some("allowed_users"))
        || matches!(
            named_instance_field(key, "feishu_instances"),
            Some("allowed_users" | "group_reply.allowed_sender_ids")
        )
        || matches!(named_instance_field(key, "dingtalk_instances"), Some("allowed_users"))
}

fn is_multiline_channel_key(key: &str) -> bool {
    is_static_multiline_channel_key(key) || is_named_instance_multiline_channel_key(key)
}

#[derive(Debug, Clone)]
/// 表示 ChannelsSettingsState 相关的应用状态或派生数据。
pub(crate) struct ChannelsSettingsState {
    pub(crate) project_dir_input: String,
    pub(crate) text_inputs: HashMap<String, String>,
    pub(crate) text_editors: HashMap<String, text_editor::Content>,
    pub(crate) cli: bool,
    pub(crate) telegram: Option<vw_config_types::channels::TelegramConfig>,
    pub(crate) discord: Option<vw_config_types::channels::DiscordConfig>,
    pub(crate) slack: Option<vw_config_types::channels::SlackConfig>,
    pub(crate) mattermost: Option<vw_config_types::channels::MattermostConfig>,
    pub(crate) webhook: Option<vw_config_types::channels::WebhookConfig>,
    pub(crate) imessage: Option<vw_config_types::channels::IMessageConfig>,
    pub(crate) matrix: Option<vw_config_types::channels::MatrixConfig>,
    pub(crate) matrix_instances: Vec<vw_config_types::channels::MatrixInstanceConfig>,
    pub(crate) signal: Option<vw_config_types::channels::SignalConfig>,
    pub(crate) whatsapp: Option<vw_config_types::channels::WhatsAppConfig>,
    pub(crate) linq: Option<vw_config_types::channels::LinqConfig>,
    pub(crate) wati: Option<vw_config_types::channels::WatiConfig>,
    pub(crate) nextcloud_talk: Option<vw_config_types::channels::NextcloudTalkConfig>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) email: Option<vw_config_types::channels::EmailConfig>,
    pub(crate) irc: Option<vw_config_types::channels::IrcConfig>,
    pub(crate) lark: Option<vw_config_types::channels::LarkConfig>,
    pub(crate) feishu: Option<vw_config_types::channels::FeishuConfig>,
    pub(crate) feishu_instances: Vec<vw_config_types::channels::FeishuInstanceConfig>,
    pub(crate) dingtalk: Option<vw_config_types::channels::DingTalkConfig>,
    pub(crate) dingtalk_instances: Vec<vw_config_types::channels::DingTalkInstanceConfig>,
    pub(crate) qq: Option<vw_config_types::channels::QQConfig>,
    pub(crate) nostr: Option<vw_config_types::channels::NostrConfig>,
    pub(crate) clawdtalk: Option<vw_config_types::channels::ClawdTalkConfig>,
    pub(crate) message_timeout_secs: u32,
    pub(crate) expanded_panels: HashSet<String>,
    pub(crate) save_error: Option<String>,
}

impl Default for ChannelsSettingsState {
    fn default() -> Self {
        Self {
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
            matrix: None,
            matrix_instances: Vec::new(),
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
            nostr: None,
            clawdtalk: None,
            message_timeout_secs: 300,
            expanded_panels: ["feishu".to_string()].into_iter().collect(),
            save_error: None,
        }
    }
}

impl ChannelsSettingsState {
    /// 执行 project_dir 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn project_dir(&self) -> Option<PathBuf> {
        let value = self.project_dir_input.trim();
        if value.is_empty() { None } else { Some(PathBuf::from(value)) }
    }

    /// 执行 refresh_text_inputs 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn refresh_text_inputs(&mut self) {
        self.text_inputs.retain(|key, _| !is_multiline_channel_key(key));
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
        if let Some(cfg) = self.matrix.as_ref() {
            insert_list(&mut self.text_inputs, "matrix.allowed_users", &cfg.allowed_users);
            insert_editor(&mut self.text_editors, "matrix.allowed_users", &cfg.allowed_users);
        }
        for (index, instance) in self.matrix_instances.iter().enumerate() {
            let key = format!("matrix_instances.{index}.allowed_users");
            insert_list(&mut self.text_inputs, &key, &instance.config.allowed_users);
            insert_editor(&mut self.text_editors, &key, &instance.config.allowed_users);
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

#[cfg(test)]
#[path = "channels_tests.rs"]
mod channels_tests;
