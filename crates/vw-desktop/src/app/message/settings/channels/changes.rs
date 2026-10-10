//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::app::App;
use crate::app::message::settings::util::parse_comma_or_newline_list;

use super::helpers::{
    parse_qq_receive_mode, parse_receive_mode, set_group_reply_allowed, set_group_reply_mode,
    trim_to_option,
};

fn parse_instance_field<'a>(key: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(prefix)?.strip_prefix('.')?;
    let (index, field) = rest.split_once('.')?;
    Some((index.parse().ok()?, field))
}

fn apply_instance_text_change(app: &mut App, key: &str, value: &str) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.channels_settings.feishu_instances.get_mut(index) {
            match field {
                "id" => instance.id = value.trim().to_string(),
                "app_id" => instance.config.app_id = value.to_string(),
                "app_secret" => instance.config.app_secret = value.to_string(),
                "encrypt_key" => instance.config.encrypt_key = trim_to_option(value),
                "verification_token" => {
                    instance.config.verification_token = trim_to_option(value);
                }
                "allowed_users" => {
                    instance.config.allowed_users = parse_comma_or_newline_list(value);
                }
                "group_reply.allowed_sender_ids" => {
                    set_group_reply_allowed(&mut instance.config.group_reply, value);
                }
                _ => {}
            }
        }
        return true;
    }

    if let Some((index, field)) = parse_instance_field(key, "dingtalk_instances") {
        if let Some(instance) = app.channels_settings.dingtalk_instances.get_mut(index) {
            match field {
                "id" => instance.id = value.trim().to_string(),
                "client_id" => instance.config.client_id = value.to_string(),
                "client_secret" => instance.config.client_secret = value.to_string(),
                "allowed_users" => {
                    instance.config.allowed_users = parse_comma_or_newline_list(value);
                }
                _ => {}
            }
        }
        return true;
    }

    false
}

fn apply_instance_number_change(app: &mut App, key: &str, value: u32) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.channels_settings.feishu_instances.get_mut(index) {
            match field {
                "port" => instance.config.port = Some(value.clamp(1, u16::MAX as u32) as u16),
                "draft_update_interval_ms" => {
                    instance.config.draft_update_interval_ms = value.max(100) as u64;
                }
                "max_draft_edits" => instance.config.max_draft_edits = value.max(1),
                _ => {}
            }
        }
        return true;
    }

    false
}

fn apply_instance_receive_mode_change(app: &mut App, key: &str, value: &str) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.channels_settings.feishu_instances.get_mut(index) {
            match field {
                "group_reply.mode" => set_group_reply_mode(&mut instance.config.group_reply, value),
                "receive_mode" => instance.config.receive_mode = parse_receive_mode(value),
                _ => {}
            }
        }
        return true;
    }

    false
}

/// 处理 `apply_text_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_text_change(app: &mut App, key: &str, value: String) {
    if apply_instance_text_change(app, key, &value) {
        return;
    }

    match key {
        "telegram.bot_token" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.bot_token = value;
            }
        }
        "telegram.allowed_users" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "telegram.base_url" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.base_url = trim_to_option(&value);
            }
        }
        "telegram.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "discord.bot_token" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                cfg.bot_token = value;
            }
        }
        "discord.guild_id" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                cfg.guild_id = trim_to_option(&value);
            }
        }
        "discord.allowed_users" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "discord.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "slack.bot_token" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                cfg.bot_token = value;
            }
        }
        "slack.app_token" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                cfg.app_token = trim_to_option(&value);
            }
        }
        "slack.channel_id" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                cfg.channel_id = trim_to_option(&value);
            }
        }
        "slack.allowed_users" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "slack.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "mattermost.url" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.url = value;
            }
        }
        "mattermost.bot_token" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.bot_token = value;
            }
        }
        "mattermost.channel_id" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.channel_id = trim_to_option(&value);
            }
        }
        "mattermost.allowed_users" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "mattermost.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "webhook.secret" => {
            if let Some(cfg) = app.channels_settings.webhook.as_mut() {
                cfg.secret = trim_to_option(&value);
            }
        }
        "imessage.allowed_contacts" => {
            if let Some(cfg) = app.channels_settings.imessage.as_mut() {
                cfg.allowed_contacts = parse_comma_or_newline_list(&value);
            }
        }
        "signal.http_url" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.http_url = value;
            }
        }
        "signal.account" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.account = value;
            }
        }
        "signal.group_id" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.group_id = trim_to_option(&value);
            }
        }
        "signal.allowed_from" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.allowed_from = parse_comma_or_newline_list(&value);
            }
        }
        "whatsapp.access_token" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.access_token = trim_to_option(&value);
            }
        }
        "whatsapp.phone_number_id" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.phone_number_id = trim_to_option(&value);
            }
        }
        "whatsapp.verify_token" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.verify_token = trim_to_option(&value);
            }
        }
        "whatsapp.app_secret" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.app_secret = trim_to_option(&value);
            }
        }
        "whatsapp.session_path" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.session_path = trim_to_option(&value);
            }
        }
        "whatsapp.pair_phone" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.pair_phone = trim_to_option(&value);
            }
        }
        "whatsapp.pair_code" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.pair_code = trim_to_option(&value);
            }
        }
        "whatsapp.allowed_numbers" => {
            if let Some(cfg) = app.channels_settings.whatsapp.as_mut() {
                cfg.allowed_numbers = parse_comma_or_newline_list(&value);
            }
        }
        "linq.api_token" => {
            if let Some(cfg) = app.channels_settings.linq.as_mut() {
                cfg.api_token = value;
            }
        }
        "linq.from_phone" => {
            if let Some(cfg) = app.channels_settings.linq.as_mut() {
                cfg.from_phone = value;
            }
        }
        "linq.signing_secret" => {
            if let Some(cfg) = app.channels_settings.linq.as_mut() {
                cfg.signing_secret = trim_to_option(&value);
            }
        }
        "linq.allowed_senders" => {
            if let Some(cfg) = app.channels_settings.linq.as_mut() {
                cfg.allowed_senders = parse_comma_or_newline_list(&value);
            }
        }
        "wati.api_token" => {
            if let Some(cfg) = app.channels_settings.wati.as_mut() {
                cfg.api_token = value;
            }
        }
        "wati.api_url" => {
            if let Some(cfg) = app.channels_settings.wati.as_mut() {
                cfg.api_url = value;
            }
        }
        "wati.tenant_id" => {
            if let Some(cfg) = app.channels_settings.wati.as_mut() {
                cfg.tenant_id = trim_to_option(&value);
            }
        }
        "wati.allowed_numbers" => {
            if let Some(cfg) = app.channels_settings.wati.as_mut() {
                cfg.allowed_numbers = parse_comma_or_newline_list(&value);
            }
        }
        "nextcloud_talk.base_url" => {
            if let Some(cfg) = app.channels_settings.nextcloud_talk.as_mut() {
                cfg.base_url = value;
            }
        }
        "nextcloud_talk.app_token" => {
            if let Some(cfg) = app.channels_settings.nextcloud_talk.as_mut() {
                cfg.app_token = value;
            }
        }
        "nextcloud_talk.webhook_secret" => {
            if let Some(cfg) = app.channels_settings.nextcloud_talk.as_mut() {
                cfg.webhook_secret = trim_to_option(&value);
            }
        }
        "nextcloud_talk.allowed_users" => {
            if let Some(cfg) = app.channels_settings.nextcloud_talk.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_host" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.imap_host = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_folder" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.imap_folder = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_host" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.smtp_host = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.username" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.username = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.password" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.password = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.from_address" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.from_address = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.allowed_senders" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.allowed_senders = parse_comma_or_newline_list(&value);
            }
        }
        "irc.server" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.server = value;
            }
        }
        "irc.nickname" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.nickname = value;
            }
        }
        "irc.username" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.username = trim_to_option(&value);
            }
        }
        "irc.channels" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.channels = parse_comma_or_newline_list(&value);
            }
        }
        "irc.allowed_users" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "irc.server_password" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.server_password = trim_to_option(&value);
            }
        }
        "irc.nickserv_password" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.nickserv_password = trim_to_option(&value);
            }
        }
        "irc.sasl_password" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.sasl_password = trim_to_option(&value);
            }
        }
        "lark.app_id" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.app_id = value;
            }
        }
        "lark.app_secret" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.app_secret = value;
            }
        }
        "lark.encrypt_key" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.encrypt_key = trim_to_option(&value);
            }
        }
        "lark.verification_token" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.verification_token = trim_to_option(&value);
            }
        }
        "lark.allowed_users" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "lark.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "feishu.app_id" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.app_id = value;
            }
        }
        "feishu.app_secret" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.app_secret = value;
            }
        }
        "feishu.encrypt_key" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.encrypt_key = trim_to_option(&value);
            }
        }
        "feishu.verification_token" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.verification_token = trim_to_option(&value);
            }
        }
        "feishu.allowed_users" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "feishu.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "dingtalk.client_id" => {
            if let Some(cfg) = app.channels_settings.dingtalk.as_mut() {
                cfg.client_id = value;
            }
        }
        "dingtalk.client_secret" => {
            if let Some(cfg) = app.channels_settings.dingtalk.as_mut() {
                cfg.client_secret = value;
            }
        }
        "dingtalk.allowed_users" => {
            if let Some(cfg) = app.channels_settings.dingtalk.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "qq.app_id" => {
            if let Some(cfg) = app.channels_settings.qq.as_mut() {
                cfg.app_id = value;
            }
        }
        "qq.app_secret" => {
            if let Some(cfg) = app.channels_settings.qq.as_mut() {
                cfg.app_secret = value;
            }
        }
        "qq.allowed_users" => {
            if let Some(cfg) = app.channels_settings.qq.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "nostr.private_key" => {
            if let Some(cfg) = app.channels_settings.nostr.as_mut() {
                cfg.private_key = value;
            }
        }
        "nostr.relays" => {
            if let Some(cfg) = app.channels_settings.nostr.as_mut() {
                cfg.relays = parse_comma_or_newline_list(&value);
            }
        }
        "nostr.allowed_pubkeys" => {
            if let Some(cfg) = app.channels_settings.nostr.as_mut() {
                cfg.allowed_pubkeys = parse_comma_or_newline_list(&value);
            }
        }
        "clawdtalk.api_key" => {
            if let Some(cfg) = app.channels_settings.clawdtalk.as_mut() {
                cfg.api_key = value;
            }
        }
        "clawdtalk.connection_id" => {
            if let Some(cfg) = app.channels_settings.clawdtalk.as_mut() {
                cfg.connection_id = value;
            }
        }
        "clawdtalk.from_number" => {
            if let Some(cfg) = app.channels_settings.clawdtalk.as_mut() {
                cfg.from_number = value;
            }
        }
        "clawdtalk.allowed_destinations" => {
            if let Some(cfg) = app.channels_settings.clawdtalk.as_mut() {
                cfg.allowed_destinations = parse_comma_or_newline_list(&value);
            }
        }
        "clawdtalk.webhook_secret" => {
            if let Some(cfg) = app.channels_settings.clawdtalk.as_mut() {
                cfg.webhook_secret = trim_to_option(&value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod changes_tests;

/// 处理 `apply_bool_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_bool_change(app: &mut App, key: &str, value: bool) {

    match key {
        "telegram.interrupt_on_new_message" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.interrupt_on_new_message = value;
            }
        }
        "telegram.mention_only" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.mention_only = value;
            }
        }
        "discord.listen_to_bots" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                cfg.listen_to_bots = value;
            }
        }
        "discord.mention_only" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                cfg.mention_only = value;
            }
        }
        "mattermost.thread_replies" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.thread_replies = Some(value);
            }
        }
        "mattermost.mention_only" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                cfg.mention_only = Some(value);
            }
        }
        "signal.ignore_attachments" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.ignore_attachments = value;
            }
        }
        "signal.ignore_stories" => {
            if let Some(cfg) = app.channels_settings.signal.as_mut() {
                cfg.ignore_stories = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_tls" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.smtp_tls = value;
            }
        }
        "lark.mention_only" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.mention_only = value;
            }
        }
        "lark.use_feishu" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.use_feishu = value;
            }
        }
        "irc.verify_tls" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.verify_tls = Some(value);
            }
        }
        _ => {}
    }
}

/// 处理 `apply_number_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_number_change(app: &mut App, key: &str, value: u32) {
    if apply_instance_number_change(app, key, value) {
        return;
    }

    match key {
        "telegram.draft_update_interval_ms" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "webhook.port" => {
            if let Some(cfg) = app.channels_settings.webhook.as_mut() {
                cfg.port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_port" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.imap_port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_port" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.smtp_port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.idle_timeout_secs" => {
            if let Some(cfg) = app.channels_settings.email.as_mut() {
                cfg.idle_timeout_secs = value.max(60) as u64;
            }
        }
        "irc.port" => {
            if let Some(cfg) = app.channels_settings.irc.as_mut() {
                cfg.port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        "lark.port" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.port = Some(value.clamp(1, u16::MAX as u32) as u16);
            }
        }
        "lark.draft_update_interval_ms" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "lark.max_draft_edits" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.max_draft_edits = value.max(1);
            }
        }
        "feishu.port" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.port = Some(value.clamp(1, u16::MAX as u32) as u16);
            }
        }
        "feishu.draft_update_interval_ms" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "feishu.max_draft_edits" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.max_draft_edits = value.max(1);
            }
        }
        _ => {}
    }
}

/// 处理 `apply_receive_mode_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_receive_mode_change(app: &mut App, key: &str, value: String) {
    if apply_instance_receive_mode_change(app, key, &value) {
        return;
    }

    match key {
        "telegram.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.telegram.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "discord.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.discord.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "slack.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.slack.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "mattermost.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.mattermost.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "lark.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "lark.receive_mode" => {
            if let Some(cfg) = app.channels_settings.lark.as_mut() {
                cfg.receive_mode = parse_receive_mode(&value);
            }
        }
        "feishu.group_reply.mode" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "feishu.receive_mode" => {
            if let Some(cfg) = app.channels_settings.feishu.as_mut() {
                cfg.receive_mode = parse_receive_mode(&value);
            }
        }
        "qq.receive_mode" => {
            if let Some(cfg) = app.channels_settings.qq.as_mut() {
                cfg.receive_mode = parse_qq_receive_mode(&value);
            }
        }
        _ => {}
    }
}
