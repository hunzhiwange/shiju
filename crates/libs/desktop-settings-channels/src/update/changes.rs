//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::state::ChannelsSettingsState;
use super::parse_comma_or_newline_list;

use super::helpers::{
    parse_qq_receive_mode, parse_receive_mode, set_group_reply_allowed, set_group_reply_mode,
    trim_to_option,
};

fn parse_instance_field<'a>(key: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(prefix)?.strip_prefix('.')?;
    let (index, field) = rest.split_once('.')?;
    Some((index.parse().ok()?, field))
}

pub(super) fn apply_instance_text_change(app: &mut ChannelsSettingsState, key: &str, value: &str) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.feishu_instances.get_mut(index) {
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
        if let Some(instance) = app.dingtalk_instances.get_mut(index) {
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

fn apply_instance_number_change(app: &mut ChannelsSettingsState, key: &str, value: u32) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.feishu_instances.get_mut(index) {
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

fn apply_instance_receive_mode_change(app: &mut ChannelsSettingsState, key: &str, value: &str) -> bool {
    if let Some((index, field)) = parse_instance_field(key, "feishu_instances") {
        if let Some(instance) = app.feishu_instances.get_mut(index) {
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

pub(super) use super::text_changes::apply_text_change;

/// 处理 `apply_bool_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_bool_change(app: &mut ChannelsSettingsState, key: &str, value: bool) {
    match key {
        "telegram.interrupt_on_new_message" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.interrupt_on_new_message = value;
            }
        }
        "telegram.mention_only" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.mention_only = value;
            }
        }
        "discord.listen_to_bots" => {
            if let Some(cfg) = app.discord.as_mut() {
                cfg.listen_to_bots = value;
            }
        }
        "discord.mention_only" => {
            if let Some(cfg) = app.discord.as_mut() {
                cfg.mention_only = value;
            }
        }
        "mattermost.thread_replies" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.thread_replies = Some(value);
            }
        }
        "mattermost.mention_only" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                cfg.mention_only = Some(value);
            }
        }
        "signal.ignore_attachments" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.ignore_attachments = value;
            }
        }
        "signal.ignore_stories" => {
            if let Some(cfg) = app.signal.as_mut() {
                cfg.ignore_stories = value;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_tls" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.smtp_tls = value;
            }
        }
        "lark.mention_only" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.mention_only = value;
            }
        }
        "lark.use_feishu" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.use_feishu = value;
            }
        }
        "irc.verify_tls" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.verify_tls = Some(value);
            }
        }
        _ => {}
    }
}

/// 处理 `apply_number_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_number_change(app: &mut ChannelsSettingsState, key: &str, value: u32) {
    if apply_instance_number_change(app, key, value) {
        return;
    }

    match key {
        "telegram.draft_update_interval_ms" => {
            if let Some(cfg) = app.telegram.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "webhook.port" => {
            if let Some(cfg) = app.webhook.as_mut() {
                cfg.port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.imap_port" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.imap_port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.smtp_port" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.smtp_port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        "email.idle_timeout_secs" => {
            if let Some(cfg) = app.email.as_mut() {
                cfg.idle_timeout_secs = value.max(60) as u64;
            }
        }
        "irc.port" => {
            if let Some(cfg) = app.irc.as_mut() {
                cfg.port = value.clamp(1, u16::MAX as u32) as u16;
            }
        }
        "lark.port" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.port = Some(value.clamp(1, u16::MAX as u32) as u16);
            }
        }
        "lark.draft_update_interval_ms" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "lark.max_draft_edits" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.max_draft_edits = value.max(1);
            }
        }
        "feishu.port" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.port = Some(value.clamp(1, u16::MAX as u32) as u16);
            }
        }
        "feishu.draft_update_interval_ms" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.draft_update_interval_ms = value.max(100) as u64;
            }
        }
        "feishu.max_draft_edits" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.max_draft_edits = value.max(1);
            }
        }
        _ => {}
    }
}

/// 处理 `apply_receive_mode_change` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn apply_receive_mode_change(app: &mut ChannelsSettingsState, key: &str, value: String) {
    if apply_instance_receive_mode_change(app, key, &value) {
        return;
    }

    match key {
        "telegram.group_reply.mode" => {
            if let Some(cfg) = app.telegram.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "discord.group_reply.mode" => {
            if let Some(cfg) = app.discord.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "slack.group_reply.mode" => {
            if let Some(cfg) = app.slack.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "mattermost.group_reply.mode" => {
            if let Some(cfg) = app.mattermost.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "lark.group_reply.mode" => {
            if let Some(cfg) = app.lark.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "lark.receive_mode" => {
            if let Some(cfg) = app.lark.as_mut() {
                cfg.receive_mode = parse_receive_mode(&value);
            }
        }
        "feishu.group_reply.mode" => {
            if let Some(cfg) = app.feishu.as_mut() {
                set_group_reply_mode(&mut cfg.group_reply, &value);
            }
        }
        "feishu.receive_mode" => {
            if let Some(cfg) = app.feishu.as_mut() {
                cfg.receive_mode = parse_receive_mode(&value);
            }
        }
        "qq.receive_mode" => {
            if let Some(cfg) = app.qq.as_mut() {
                cfg.receive_mode = parse_qq_receive_mode(&value);
            }
        }
        _ => {}
    }
}
