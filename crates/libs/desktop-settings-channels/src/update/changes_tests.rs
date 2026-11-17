#![allow(unused_imports, dead_code)]

// 验证频道及动态实例字段变更对配置状态的映射。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use super::super::changes::{apply_bool_change, apply_number_change, apply_receive_mode_change};
use crate::state::ChannelsSettingsState as App;
use super::super::toggles;
use config_channel_types::GroupReplyMode;
use config_channel_services::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig,
    LarkReceiveMode, QQReceiveMode,
};

fn app() -> App {
    App::default()
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn channel_changes_apply_text_bool_number_and_modes() {
    let mut app = app();
    toggles::toggle_enabled(&mut app, "telegram", true);
    toggles::toggle_enabled(&mut app, "lark", true);
    toggles::toggle_enabled(&mut app, "qq", true);
    toggles::toggle_enabled(&mut app, "webhook", true);

    apply_text_change(&mut app, "telegram.bot_token", "token".to_string());
    apply_text_change(&mut app, "telegram.group_reply.allowed_sender_ids", "u1,u2".to_string());
    apply_bool_change(&mut app, "telegram.mention_only", true);
    apply_number_change(&mut app, "telegram.draft_update_interval_ms", 10);
    apply_number_change(&mut app, "webhook.port", 0);
    apply_receive_mode_change(&mut app, "lark.receive_mode", "webhook".to_string());
    apply_receive_mode_change(&mut app, "qq.receive_mode", "websocket".to_string());

    let telegram = app.telegram.as_ref().expect("telegram enabled");
    assert_eq!(telegram.bot_token, "token");
    assert_eq!(
        telegram.group_reply.as_ref().expect("group reply").allowed_sender_ids,
        vec!["u1".to_string(), "u2".to_string()]
    );
    assert!(telegram.mention_only);
    assert_eq!(telegram.draft_update_interval_ms, 100);
    assert_eq!(app.webhook.as_ref().expect("webhook").port, 1);
    assert_eq!(
        app.lark.as_ref().expect("lark").receive_mode,
        LarkReceiveMode::Webhook
    );
    assert_eq!(
        app.qq.as_ref().expect("qq").receive_mode,
        QQReceiveMode::Websocket
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn channel_text_changes_clear_optional_fields_and_ignore_disabled_or_unknown_keys() {
    let mut app = app();
    toggles::toggle_enabled(&mut app, "telegram", true);
    apply_text_change(&mut app, "telegram.base_url", " https://api.invalid ".to_string());
    apply_text_change(&mut app, "telegram.base_url", "   ".to_string());
    apply_text_change(&mut app, "discord.bot_token", "ignored".to_string());
    apply_text_change(&mut app, "unknown.field", "ignored".to_string());

    assert_eq!(app.telegram.as_ref().unwrap().base_url, None);
    assert!(app.discord.is_none());
}

#[test]
fn unknown_instance_fields_and_channel_keys_preserve_existing_values() {
    let mut app = app();
    toggles::add_channel_instance(&mut app, "feishu");
    app.feishu_instances[0].config.port = Some(8080);
    app.feishu_instances[0].config.receive_mode = LarkReceiveMode::Webhook;

    apply_number_change(&mut app, "feishu_instances.0.unknown", 1);
    apply_number_change(&mut app, "unknown.number", 1);
    apply_receive_mode_change(&mut app, "feishu_instances.0.unknown", "websocket".to_string());
    apply_receive_mode_change(&mut app, "unknown.mode", "websocket".to_string());
    apply_bool_change(&mut app, "unknown.flag", true);

    let feishu = &app.feishu_instances[0].config;
    assert_eq!(feishu.port, Some(8080));
    assert_eq!(feishu.receive_mode, LarkReceiveMode::Webhook);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn channel_number_changes_enforce_lower_and_upper_bounds() {
    let mut app = app();
    for channel in ["telegram", "webhook", "irc", "lark", "feishu"] {
        toggles::toggle_enabled(&mut app, channel, true);
    }

    apply_number_change(&mut app, "telegram.draft_update_interval_ms", 0);
    apply_number_change(&mut app, "webhook.port", u32::MAX);
    apply_number_change(&mut app, "irc.port", 0);
    apply_number_change(&mut app, "lark.port", u32::MAX);
    apply_number_change(&mut app, "lark.draft_update_interval_ms", 0);
    apply_number_change(&mut app, "lark.max_draft_edits", 0);
    apply_number_change(&mut app, "feishu.port", 0);
    apply_number_change(&mut app, "feishu.draft_update_interval_ms", 99);
    apply_number_change(&mut app, "feishu.max_draft_edits", 0);
    #[cfg(not(target_arch = "wasm32"))]
    {
        toggles::toggle_enabled(&mut app, "email", true);
        apply_number_change(&mut app, "email.imap_port", 0);
        apply_number_change(&mut app, "email.smtp_port", u32::MAX);
        apply_number_change(&mut app, "email.idle_timeout_secs", 1);
    }

    assert_eq!(app.telegram.as_ref().unwrap().draft_update_interval_ms, 100);
    assert_eq!(app.webhook.as_ref().unwrap().port, u16::MAX);
    assert_eq!(app.irc.as_ref().unwrap().port, 1);
    let lark = app.lark.as_ref().unwrap();
    assert_eq!(lark.port, Some(u16::MAX));
    assert_eq!(lark.draft_update_interval_ms, 100);
    assert_eq!(lark.max_draft_edits, 1);
    let feishu = app.feishu.as_ref().unwrap();
    assert_eq!(feishu.port, Some(1));
    assert_eq!(feishu.draft_update_interval_ms, 100);
    assert_eq!(feishu.max_draft_edits, 1);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let email = app.email.as_ref().unwrap();
        assert_eq!(email.imap_port, 1);
        assert_eq!(email.smtp_port, u16::MAX);
        assert_eq!(email.idle_timeout_secs, 60);
    }
}

#[test]
fn channel_receive_mode_changes_cover_group_and_transport_modes() {
    let mut app = app();
    for channel in ["telegram", "discord", "slack", "mattermost", "lark", "feishu", "qq"] {
        toggles::toggle_enabled(&mut app, channel, true);
    }

    for key in [
        "telegram.group_reply.mode",
        "discord.group_reply.mode",
        "slack.group_reply.mode",
        "mattermost.group_reply.mode",
        "lark.group_reply.mode",
        "feishu.group_reply.mode",
    ] {
        apply_receive_mode_change(&mut app, key, "mention_only".to_string());
    }
    apply_receive_mode_change(&mut app, "lark.receive_mode", " WEBHOOK ".to_string());
    apply_receive_mode_change(&mut app, "feishu.receive_mode", "unknown".to_string());
    apply_receive_mode_change(&mut app, "qq.receive_mode", "unknown".to_string());

    assert_eq!(
        app.telegram.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.discord.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.slack.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.mattermost.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(app.lark.as_ref().unwrap().receive_mode, LarkReceiveMode::Webhook);
    assert_eq!(
        app.feishu.as_ref().unwrap().receive_mode,
        LarkReceiveMode::Websocket
    );
    assert_eq!(app.qq.as_ref().unwrap().receive_mode, QQReceiveMode::Webhook);
}

#[test]
fn disabled_channel_changes_are_ignored_and_group_modes_fallback_to_all_messages() {
    let mut app = app();

    apply_bool_change(&mut app, "telegram.mention_only", true);
    apply_number_change(&mut app, "webhook.port", 443);
    apply_receive_mode_change(&mut app, "lark.receive_mode", "webhook".to_string());
    assert!(app.telegram.is_none());
    assert!(app.webhook.is_none());
    assert!(app.lark.is_none());

    for channel in ["telegram", "discord", "slack", "mattermost", "lark", "feishu"] {
        toggles::toggle_enabled(&mut app, channel, true);
    }
    for key in [
        "telegram.group_reply.mode",
        "discord.group_reply.mode",
        "slack.group_reply.mode",
        "mattermost.group_reply.mode",
        "lark.group_reply.mode",
        "feishu.group_reply.mode",
    ] {
        apply_receive_mode_change(&mut app, key, "all_messages".to_string());
    }

    assert_eq!(
        app.telegram.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.discord.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.slack.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.mattermost.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.lark.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.feishu.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn instance_number_changes_clamp_feishu_port_to_valid_lower_bound() {
    let mut app = app();
    toggles::add_channel_instance(&mut app, "feishu");

    apply_number_change(&mut app, "feishu_instances.0.port", 0);
    apply_number_change(&mut app, "feishu_instances.0.draft_update_interval_ms", 100);

    let feishu = &app.feishu_instances[0].config;
    assert_eq!(feishu.port, Some(1));
    assert_eq!(feishu.draft_update_interval_ms, 100);
}
