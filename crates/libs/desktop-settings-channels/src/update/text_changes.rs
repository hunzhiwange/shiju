use crate::state::ChannelsSettingsState;
use super::changes::apply_instance_text_change;
use super::parse_comma_or_newline_list;
use super::helpers::{trim_to_option, set_group_reply_allowed};

/// 处理 `apply_text_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_text_change(app: &mut ChannelsSettingsState, key: &str, value: String) {
    if apply_instance_text_change(app, key, &value) {
        return;
    }

    match key {
        "telegram.bot_token" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.bot_token = value;
            }
        }
        "telegram.allowed_users" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "telegram.base_url" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.base_url = trim_to_option(&value);
            }
        }
        "telegram.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.telegram.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "discord.bot_token" => {
            if let Some(cfg) = app.discord.as_mut() {
                cfg.bot_token = value;
            }
        }
        "discord.guild_id" => {
            if let Some(cfg) = app.discord.as_mut() {
                cfg.guild_id = trim_to_option(&value);
            }
        }
        "discord.allowed_users" => {
            if let Some(cfg) = app.discord.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "discord.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.discord.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "slack.bot_token" => {
            if let Some(cfg) = app.slack.as_mut() {
                cfg.bot_token = value;
            }
        }
        "slack.app_token" => {
            if let Some(cfg) = app.slack.as_mut() {
                cfg.app_token = trim_to_option(&value);
            }
        }
        "slack.channel_id" => {
            if let Some(cfg) = app.slack.as_mut() {
                cfg.channel_id = trim_to_option(&value);
            }
        }
        "slack.allowed_users" => {
            if let Some(cfg) = app.slack.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "slack.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.slack.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "mattermost.url" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.url = value;
            }
        }
        "mattermost.bot_token" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.bot_token = value;
            }
        }
        "mattermost.channel_id" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.channel_id = trim_to_option(&value);
            }
        }
        "mattermost.allowed_users" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "mattermost.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "webhook.secret" => {
            if let Some(cfg) = app.webhook.as_mut() {
                cfg.secret = trim_to_option(&value);
            }
        }
        "imessage.allowed_contacts" => {
            if let Some(cfg) = app.imessage.as_mut() {
                cfg.allowed_contacts = parse_comma_or_newline_list(&value);
            }
        }
        "signal.http_url" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.http_url = value;
            }
        }
        "signal.account" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.account = value;
            }
        }
        "signal.group_id" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.group_id = trim_to_option(&value);
            }
        }
        "signal.allowed_from" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.allowed_from = parse_comma_or_newline_list(&value);
            }
        }
        "whatsapp.access_token" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.access_token = trim_to_option(&value);
            }
        }
        "whatsapp.phone_number_id" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.phone_number_id = trim_to_option(&value);
            }
        }
        "whatsapp.verify_token" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.verify_token = trim_to_option(&value);
            }
        }
        "whatsapp.app_secret" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.app_secret = trim_to_option(&value);
            }
        }
        "whatsapp.session_path" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.session_path = trim_to_option(&value);
            }
        }
        "whatsapp.pair_phone" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.pair_phone = trim_to_option(&value);
            }
        }
        "whatsapp.pair_code" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.pair_code = trim_to_option(&value);
            }
        }
        "whatsapp.allowed_numbers" => {
            if let Some(cfg) = app.whatsapp.as_mut() {
                cfg.allowed_numbers = parse_comma_or_newline_list(&value);
            }
        }
        "linq.api_token" => {
            if let Some(cfg) = app.linq.as_mut() {
                cfg.api_token = value;
            }
        }
        "linq.from_phone" => {
            if let Some(cfg) = app.linq.as_mut() {
                cfg.from_phone = value;
            }
        }
        "linq.signing_secret" => {
            if let Some(cfg) = app.linq.as_mut() {
                cfg.signing_secret = trim_to_option(&value);
            }
        }
        "linq.allowed_senders" => {
            if let Some(cfg) = app.linq.as_mut() {
                cfg.allowed_senders = parse_comma_or_newline_list(&value);
            }
        }
        "wati.api_token" => {
            if let Some(cfg) = app.wati.as_mut() {
                cfg.api_token = value;
            }
        }
        "wati.api_url" => {
            if let Some(cfg) = app.wati.as_mut() {
                cfg.api_url = value;
            }
        }
        "wati.tenant_id" => {
            if let Some(cfg) = app.wati.as_mut() {
                cfg.tenant_id = trim_to_option(&value);
            }
        }
        "wati.allowed_numbers" => {
            if let Some(cfg) = app.wati.as_mut() {
                cfg.allowed_numbers = parse_comma_or_newline_list(&value);
            }
        }
        "nextcloud_talk.base_url" => {
            if let Some(cfg) = app.nextcloud_talk.as_mut() {
                cfg.base_url = value;
            }
        }
        "nextcloud_talk.app_token" => {
            if let Some(cfg) = app.nextcloud_talk.as_mut() {
                cfg.app_token = value;
            }
        }
        "nextcloud_talk.webhook_secret" => {
            if let Some(cfg) = app.nextcloud_talk.as_mut() {
                cfg.webhook_secret = trim_to_option(&value);
            }
        }
        "nextcloud_talk.allowed_users" => {
            if let Some(cfg) = app.nextcloud_talk.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_host" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.imap_host = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_folder" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.imap_folder = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_host" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.smtp_host = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.username" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.username = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.password" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.password = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.from_address" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.from_address = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.allowed_senders" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.allowed_senders = parse_comma_or_newline_list(&value);
            }
        }
        "irc.server" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.server = value;
            }
        }
        "irc.nickname" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.nickname = value;
            }
        }
        "irc.username" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.username = trim_to_option(&value);
            }
        }
        "irc.channels" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.channels = parse_comma_or_newline_list(&value);
            }
        }
        "irc.allowed_users" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "irc.server_password" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.server_password = trim_to_option(&value);
            }
        }
        "irc.nickserv_password" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.nickserv_password = trim_to_option(&value);
            }
        }
        "irc.sasl_password" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.sasl_password = trim_to_option(&value);
            }
        }
        "lark.app_id" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.app_id = value;
            }
        }
        "lark.app_secret" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.app_secret = value;
            }
        }
        "lark.encrypt_key" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.encrypt_key = trim_to_option(&value);
            }
        }
        "lark.verification_token" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.verification_token = trim_to_option(&value);
            }
        }
        "lark.allowed_users" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "lark.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.lark.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "feishu.app_id" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.app_id = value;
            }
        }
        "feishu.app_secret" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.app_secret = value;
            }
        }
        "feishu.encrypt_key" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.encrypt_key = trim_to_option(&value);
            }
        }
        "feishu.verification_token" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.verification_token = trim_to_option(&value);
            }
        }
        "feishu.allowed_users" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "feishu.group_reply.allowed_sender_ids" => {
            if let Some(cfg) = app.feishu.as_mut() {
                set_group_reply_allowed(&mut cfg.group_reply, &value);
            }
        }
        "dingtalk.client_id" => {
            if let Some(cfg) = app.dingtalk.as_mut() {
                cfg.client_id = value;
            }
        }
        "dingtalk.client_secret" => {
            if let Some(cfg) = app.dingtalk.as_mut() {
                cfg.client_secret = value;
            }
        }
        "dingtalk.allowed_users" => {
            if let Some(cfg) = app.dingtalk.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "qq.app_id" => {
            if let Some(cfg) = app.qq.as_mut() {
                cfg.app_id = value;
            }
        }
        "qq.app_secret" => {
            if let Some(cfg) = app.qq.as_mut() {
                cfg.app_secret = value;
            }
        }
        "qq.allowed_users" => {
            if let Some(cfg) = app.qq.as_mut() {
                cfg.allowed_users = parse_comma_or_newline_list(&value);
            }
        }
        "wecom.bot_id" => {
            if let Some(cfg) = app.wecom.as_mut() {
                cfg.bot_id = value;
            }
        }
        "wecom.secret" => {
            if let Some(cfg) = app.wecom.as_mut() {
                cfg.secret = value;
            }
        }
        "wechat.bot_token" => {
            if let Some(cfg) = app.wechat.as_mut() {
                cfg.bot_token = value;
            }
        }
        "wechat.base_url" => {
            if let Some(cfg) = app.wechat.as_mut() {
                cfg.base_url = value;
            }
        }
        "nostr.private_key" => {
            if let Some(cfg) = app.nostr.as_mut() {
                cfg.private_key = value;
            }
        }
        "nostr.relays" => {
            if let Some(cfg) = app.nostr.as_mut() {
                cfg.relays = parse_comma_or_newline_list(&value);
            }
        }
        "nostr.allowed_pubkeys" => {
            if let Some(cfg) = app.nostr.as_mut() {
                cfg.allowed_pubkeys = parse_comma_or_newline_list(&value);
            }
        }
        "clawdtalk.api_key" => {
            if let Some(cfg) = app.clawdtalk.as_mut() {
                cfg.api_key = value;
            }
        }
        "clawdtalk.connection_id" => {
            if let Some(cfg) = app.clawdtalk.as_mut() {
                cfg.connection_id = value;
            }
        }
        "clawdtalk.from_number" => {
            if let Some(cfg) = app.clawdtalk.as_mut() {
                cfg.from_number = value;
            }
        }
        "clawdtalk.allowed_destinations" => {
            if let Some(cfg) = app.clawdtalk.as_mut() {
                cfg.allowed_destinations = parse_comma_or_newline_list(&value);
            }
        }
        "clawdtalk.webhook_secret" => {
            if let Some(cfg) = app.clawdtalk.as_mut() {
                cfg.webhook_secret = trim_to_option(&value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod changes_tests;
