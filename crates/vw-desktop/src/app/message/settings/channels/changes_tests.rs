// 验证频道及动态实例字段变更对配置状态的映射。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::message::settings::channels::toggles;
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig, GroupReplyMode,
    LarkReceiveMode, MatrixConfig, MatrixInstanceConfig, QQReceiveMode,
};

fn app() -> App {
    App::new().0
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

    let telegram = app.channels_settings.telegram.as_ref().expect("telegram enabled");
    assert_eq!(telegram.bot_token, "token");
    assert_eq!(
        telegram.group_reply.as_ref().expect("group reply").allowed_sender_ids,
        vec!["u1".to_string(), "u2".to_string()]
    );
    assert!(telegram.mention_only);
    assert_eq!(telegram.draft_update_interval_ms, 100);
    assert_eq!(app.channels_settings.webhook.as_ref().expect("webhook").port, 1);
    assert_eq!(
        app.channels_settings.lark.as_ref().expect("lark").receive_mode,
        LarkReceiveMode::Webhook
    );
    assert_eq!(
        app.channels_settings.qq.as_ref().expect("qq").receive_mode,
        QQReceiveMode::Websocket
    );
}

#[test]
fn channel_instance_settings_apply_dynamic_instance_fields() {
    let mut app = app();
    app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
        id: "planner".to_string(),
        config: MatrixConfig {
            homeserver: String::new(),
            access_token: String::new(),
            user_id: None,
            device_id: None,
            room_id: String::new(),
            allowed_users: Vec::new(),
            mention_only: false,
        },
    });
    app.channels_settings.feishu_instances.push(FeishuInstanceConfig {
        id: "cn".to_string(),
        config: FeishuConfig {
            app_id: String::new(),
            app_secret: String::new(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            group_reply: None,
            receive_mode: LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        },
    });
    app.channels_settings.dingtalk_instances.push(DingTalkInstanceConfig {
        id: "ops".to_string(),
        config: DingTalkConfig {
            client_id: String::new(),
            client_secret: String::new(),
            allowed_users: Vec::new(),
        },
    });

    apply_text_change(&mut app, "matrix_instances.0.id", "ops".to_string());
    apply_text_change(&mut app, "matrix_instances.0.allowed_users", "@alice,@bob".to_string());
    apply_bool_change(&mut app, "matrix_instances.0.mention_only", true);
    apply_text_change(&mut app, "feishu_instances.0.app_secret", "secret".to_string());
    apply_text_change(
        &mut app,
        "feishu_instances.0.group_reply.allowed_sender_ids",
        "u1,u2".to_string(),
    );
    apply_number_change(&mut app, "feishu_instances.0.max_draft_edits", 0);
    apply_receive_mode_change(&mut app, "feishu_instances.0.receive_mode", "webhook".to_string());
    apply_text_change(
        &mut app,
        "dingtalk_instances.0.client_secret",
        "dingtalk-secret".to_string(),
    );

    let matrix = &app.channels_settings.matrix_instances[0];
    assert_eq!(matrix.id, "ops");
    assert_eq!(matrix.config.allowed_users, vec!["@alice".to_string(), "@bob".to_string()]);
    assert!(matrix.config.mention_only);
    let feishu = &app.channels_settings.feishu_instances[0];
    assert_eq!(feishu.config.app_secret, "secret");
    assert_eq!(
        feishu.config.group_reply.as_ref().unwrap().allowed_sender_ids,
        vec!["u1".to_string(), "u2".to_string()]
    );
    assert_eq!(feishu.config.max_draft_edits, 1);
    assert_eq!(feishu.config.receive_mode, LarkReceiveMode::Webhook);
    assert_eq!(app.channels_settings.dingtalk_instances[0].config.client_secret, "dingtalk-secret");
}

#[test]
fn parse_instance_field_accepts_indexed_fields_and_rejects_malformed_keys() {
    assert_eq!(
        parse_instance_field("matrix_instances.12.room_id", "matrix_instances"),
        Some((12, "room_id"))
    );
    assert_eq!(parse_instance_field("matrix_instances.room_id", "matrix_instances"), None);
    assert_eq!(parse_instance_field("matrix_instances.x.room_id", "matrix_instances"), None);
    assert_eq!(parse_instance_field("matrix_instances.0", "matrix_instances"), None);
    assert_eq!(parse_instance_field("matrix.0.room_id", "matrix_instances"), None);
}

#[test]
fn channel_text_changes_apply_scalar_optional_and_list_fields() {
    let mut app = app();
    for channel in [
        "telegram",
        "discord",
        "slack",
        "mattermost",
        "webhook",
        "imessage",
        "matrix",
        "signal",
        "whatsapp",
        "linq",
        "wati",
        "nextcloud_talk",
        "irc",
        "lark",
        "feishu",
        "dingtalk",
        "qq",
        "nostr",
        "clawdtalk",
    ] {
        toggles::toggle_enabled(&mut app, channel, true);
    }

    for (key, value) in [
        ("telegram.bot_token", "telegram-value"),
        ("telegram.allowed_users", "t1\nt2"),
        ("telegram.base_url", " https://telegram.invalid "),
        ("telegram.group_reply.allowed_sender_ids", "t1,t2"),
        ("discord.bot_token", "discord-value"),
        ("discord.guild_id", " guild "),
        ("discord.allowed_users", "d1\nd2"),
        ("discord.group_reply.allowed_sender_ids", "d1,d2"),
        ("slack.bot_token", "slack-value"),
        ("slack.app_token", " app "),
        ("slack.channel_id", " channel "),
        ("slack.allowed_users", "s1,s2"),
        ("slack.group_reply.allowed_sender_ids", "s1\ns2"),
        ("mattermost.url", "https://mattermost.invalid"),
        ("mattermost.bot_token", "mattermost-value"),
        ("mattermost.channel_id", " room "),
        ("mattermost.allowed_users", "m1,m2"),
        ("mattermost.group_reply.allowed_sender_ids", "m1,m2"),
        ("webhook.secret", " secret "),
        ("imessage.allowed_contacts", "alice\nbob"),
        ("matrix.homeserver", "https://matrix.invalid"),
        ("matrix.access_token", "matrix-value"),
        ("matrix.user_id", " @user:invalid "),
        ("matrix.device_id", " device "),
        ("matrix.room_id", "!room:invalid"),
        ("matrix.allowed_users", "@a:invalid,@b:invalid"),
        ("signal.http_url", "http://signal.invalid"),
        ("signal.account", "+10000000000"),
        ("signal.group_id", " group "),
        ("signal.allowed_from", "+1\n+2"),
        ("whatsapp.access_token", " access "),
        ("whatsapp.phone_number_id", " phone "),
        ("whatsapp.verify_token", " verify "),
        ("whatsapp.app_secret", " app-secret "),
        ("whatsapp.session_path", " session "),
        ("whatsapp.pair_phone", " pair-phone "),
        ("whatsapp.pair_code", " pair-code "),
        ("whatsapp.allowed_numbers", "+1,+2"),
        ("linq.api_token", "linq-value"),
        ("linq.from_phone", "+3"),
        ("linq.signing_secret", " signing "),
        ("linq.allowed_senders", "+3,+4"),
        ("wati.api_token", "wati-value"),
        ("wati.api_url", "https://wati.invalid"),
        ("wati.tenant_id", " tenant "),
        ("wati.allowed_numbers", "+5,+6"),
        ("nextcloud_talk.base_url", "https://nextcloud.invalid"),
        ("nextcloud_talk.app_token", "nextcloud-value"),
        ("nextcloud_talk.webhook_secret", " webhook "),
        ("nextcloud_talk.allowed_users", "n1,n2"),
        ("irc.server", "irc.invalid"),
        ("irc.nickname", "vibe"),
        ("irc.username", " user "),
        ("irc.channels", "#one\n#two"),
        ("irc.allowed_users", "i1,i2"),
        ("irc.server_password", " server-password "),
        ("irc.nickserv_password", " nick-password "),
        ("irc.sasl_password", " sasl-password "),
        ("lark.app_id", "lark-id"),
        ("lark.app_secret", "lark-value"),
        ("lark.encrypt_key", " encrypt "),
        ("lark.verification_token", " verify "),
        ("lark.allowed_users", "l1,l2"),
        ("lark.group_reply.allowed_sender_ids", "l1,l2"),
        ("feishu.app_id", "feishu-id"),
        ("feishu.app_secret", "feishu-value"),
        ("feishu.encrypt_key", " encrypt "),
        ("feishu.verification_token", " verify "),
        ("feishu.allowed_users", "f1,f2"),
        ("feishu.group_reply.allowed_sender_ids", "f1,f2"),
        ("dingtalk.client_id", "ding-id"),
        ("dingtalk.client_secret", "ding-value"),
        ("dingtalk.allowed_users", "dt1,dt2"),
        ("qq.app_id", "qq-id"),
        ("qq.app_secret", "qq-value"),
        ("qq.allowed_users", "q1,q2"),
        ("nostr.private_key", "nostr-value"),
        ("nostr.relays", "wss://one.invalid\nwss://two.invalid"),
        ("nostr.allowed_pubkeys", "p1,p2"),
        ("clawdtalk.api_key", "clawd-value"),
        ("clawdtalk.connection_id", "connection"),
        ("clawdtalk.from_number", "+7"),
        ("clawdtalk.allowed_destinations", "+8,+9"),
        ("clawdtalk.webhook_secret", " clawd-webhook "),
    ] {
        apply_text_change(&mut app, key, value.to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        toggles::toggle_enabled(&mut app, "email", true);
        for (key, value) in [
            ("email.imap_host", "imap.invalid"),
            ("email.imap_folder", "Archive"),
            ("email.smtp_host", "smtp.invalid"),
            ("email.username", "mail-user"),
            ("email.password", "mail-value"),
            ("email.from_address", "agent@example.invalid"),
            ("email.allowed_senders", "one@example.invalid,two@example.invalid"),
        ] {
            apply_text_change(&mut app, key, value.to_string());
        }
    }

    let telegram = app.channels_settings.telegram.as_ref().unwrap();
    assert_eq!(telegram.allowed_users, ["t1", "t2"]);
    assert_eq!(telegram.base_url.as_deref(), Some("https://telegram.invalid"));
    let discord = app.channels_settings.discord.as_ref().unwrap();
    assert_eq!(discord.guild_id.as_deref(), Some("guild"));
    assert_eq!(discord.allowed_users, ["d1", "d2"]);
    assert_eq!(discord.group_reply_allowed_sender_ids(), ["d1", "d2"]);
    let slack = app.channels_settings.slack.as_ref().unwrap();
    assert_eq!(slack.app_token.as_deref(), Some("app"));
    assert_eq!(slack.channel_id.as_deref(), Some("channel"));
    assert_eq!(slack.allowed_users, ["s1", "s2"]);
    let matrix = app.channels_settings.matrix.as_ref().unwrap();
    assert_eq!(matrix.user_id.as_deref(), Some("@user:invalid"));
    assert_eq!(matrix.device_id.as_deref(), Some("device"));
    assert_eq!(matrix.allowed_users, ["@a:invalid", "@b:invalid"]);
    let whatsapp = app.channels_settings.whatsapp.as_ref().unwrap();
    assert_eq!(whatsapp.access_token.as_deref(), Some("access"));
    assert_eq!(whatsapp.session_path.as_deref(), Some("session"));
    assert_eq!(whatsapp.allowed_numbers, ["+1", "+2"]);
    let irc = app.channels_settings.irc.as_ref().unwrap();
    assert_eq!(irc.username.as_deref(), Some("user"));
    assert_eq!(irc.channels, ["#one", "#two"]);
    assert_eq!(irc.sasl_password.as_deref(), Some("sasl-password"));
    assert_eq!(app.channels_settings.lark.as_ref().unwrap().allowed_users, ["l1", "l2"]);
    assert_eq!(app.channels_settings.feishu.as_ref().unwrap().allowed_users, ["f1", "f2"]);
    assert_eq!(app.channels_settings.nostr.as_ref().unwrap().relays.len(), 2);
    assert_eq!(
        app.channels_settings.clawdtalk.as_ref().unwrap().webhook_secret.as_deref(),
        Some("clawd-webhook")
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let email = app.channels_settings.email.as_ref().unwrap();
        assert_eq!(email.imap_host, "imap.invalid");
        assert_eq!(email.imap_folder, "Archive");
        assert_eq!(email.allowed_senders, ["one@example.invalid", "two@example.invalid"]);
    }
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

    assert_eq!(app.channels_settings.telegram.as_ref().unwrap().base_url, None);
    assert!(app.channels_settings.discord.is_none());
}

#[test]
fn instance_changes_cover_all_fields_and_safely_ignore_invalid_targets() {
    let mut app = app();
    toggles::add_channel_instance(&mut app, "matrix");
    toggles::add_channel_instance(&mut app, "feishu");
    toggles::add_channel_instance(&mut app, "dingtalk");

    for (key, value) in [
        ("matrix_instances.0.id", " matrix-id "),
        ("matrix_instances.0.homeserver", "https://matrix.invalid"),
        ("matrix_instances.0.access_token", "matrix-value"),
        ("matrix_instances.0.user_id", " @user:invalid "),
        ("matrix_instances.0.device_id", " device "),
        ("matrix_instances.0.room_id", "!room:invalid"),
        ("matrix_instances.0.allowed_users", "@one:invalid,@two:invalid"),
        ("feishu_instances.0.id", " feishu-id "),
        ("feishu_instances.0.app_id", "app-id"),
        ("feishu_instances.0.app_secret", "app-value"),
        ("feishu_instances.0.encrypt_key", " encrypt "),
        ("feishu_instances.0.verification_token", " verify "),
        ("feishu_instances.0.allowed_users", "one,two"),
        ("feishu_instances.0.group_reply.allowed_sender_ids", "one,two"),
        ("dingtalk_instances.0.id", " ding-id "),
        ("dingtalk_instances.0.client_id", "client-id"),
        ("dingtalk_instances.0.client_secret", "client-value"),
        ("dingtalk_instances.0.allowed_users", "one,two"),
    ] {
        apply_text_change(&mut app, key, value.to_string());
    }
    apply_bool_change(&mut app, "matrix_instances.0.mention_only", true);
    apply_number_change(&mut app, "feishu_instances.0.port", u32::MAX);
    apply_number_change(&mut app, "feishu_instances.0.draft_update_interval_ms", 1);
    apply_number_change(&mut app, "feishu_instances.0.max_draft_edits", 0);
    apply_receive_mode_change(
        &mut app,
        "feishu_instances.0.group_reply.mode",
        "mention_only".to_string(),
    );
    apply_receive_mode_change(&mut app, "feishu_instances.0.receive_mode", "webhook".to_string());

    let matrix = &app.channels_settings.matrix_instances[0];
    assert_eq!(matrix.id, "matrix-id");
    assert_eq!(matrix.config.user_id.as_deref(), Some("@user:invalid"));
    assert!(matrix.config.mention_only);
    let feishu = &app.channels_settings.feishu_instances[0];
    assert_eq!(feishu.id, "feishu-id");
    assert_eq!(feishu.config.port, Some(u16::MAX));
    assert_eq!(feishu.config.draft_update_interval_ms, 100);
    assert_eq!(feishu.config.max_draft_edits, 1);
    assert_eq!(feishu.config.receive_mode, LarkReceiveMode::Webhook);
    assert_eq!(feishu.config.effective_group_reply_mode(), GroupReplyMode::MentionOnly);
    assert_eq!(app.channels_settings.dingtalk_instances[0].id, "ding-id");

    apply_text_change(&mut app, "matrix_instances.9.room_id", "ignored".to_string());
    apply_text_change(&mut app, "matrix_instances.0.unknown", "ignored".to_string());
    apply_bool_change(&mut app, "matrix_instances.9.mention_only", false);
    apply_number_change(&mut app, "feishu_instances.9.port", 1);
    apply_receive_mode_change(&mut app, "feishu_instances.9.receive_mode", "websocket".to_string());

    assert_eq!(app.channels_settings.matrix_instances[0].config.room_id, "!room:invalid");
    assert_eq!(app.channels_settings.feishu_instances[0].config.port, Some(u16::MAX));
}

#[test]
fn unknown_instance_fields_and_channel_keys_preserve_existing_values() {
    let mut app = app();
    toggles::add_channel_instance(&mut app, "feishu");
    app.channels_settings.feishu_instances[0].config.port = Some(8080);
    app.channels_settings.feishu_instances[0].config.receive_mode = LarkReceiveMode::Webhook;

    apply_number_change(&mut app, "feishu_instances.0.unknown", 1);
    apply_number_change(&mut app, "unknown.number", 1);
    apply_receive_mode_change(&mut app, "feishu_instances.0.unknown", "websocket".to_string());
    apply_receive_mode_change(&mut app, "unknown.mode", "websocket".to_string());
    apply_bool_change(&mut app, "unknown.flag", true);

    let feishu = &app.channels_settings.feishu_instances[0].config;
    assert_eq!(feishu.port, Some(8080));
    assert_eq!(feishu.receive_mode, LarkReceiveMode::Webhook);
}

#[test]
fn channel_bool_changes_update_every_supported_flag() {
    let mut app = app();
    for channel in ["telegram", "discord", "mattermost", "matrix", "signal", "lark", "irc"] {
        toggles::toggle_enabled(&mut app, channel, true);
    }

    for key in [
        "telegram.interrupt_on_new_message",
        "telegram.mention_only",
        "discord.listen_to_bots",
        "discord.mention_only",
        "mattermost.thread_replies",
        "mattermost.mention_only",
        "matrix.mention_only",
        "signal.ignore_attachments",
        "signal.ignore_stories",
        "lark.mention_only",
        "lark.use_feishu",
        "irc.verify_tls",
    ] {
        apply_bool_change(&mut app, key, true);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        toggles::toggle_enabled(&mut app, "email", true);
        apply_bool_change(&mut app, "email.smtp_tls", false);
    }

    let telegram = app.channels_settings.telegram.as_ref().unwrap();
    assert!(telegram.interrupt_on_new_message && telegram.mention_only);
    let discord = app.channels_settings.discord.as_ref().unwrap();
    assert!(discord.listen_to_bots && discord.mention_only);
    let mattermost = app.channels_settings.mattermost.as_ref().unwrap();
    assert_eq!(mattermost.thread_replies, Some(true));
    assert_eq!(mattermost.mention_only, Some(true));
    let signal = app.channels_settings.signal.as_ref().unwrap();
    assert!(signal.ignore_attachments && signal.ignore_stories);
    let lark = app.channels_settings.lark.as_ref().unwrap();
    assert!(lark.mention_only && lark.use_feishu);
    assert_eq!(app.channels_settings.irc.as_ref().unwrap().verify_tls, Some(true));
    #[cfg(not(target_arch = "wasm32"))]
    assert!(!app.channels_settings.email.as_ref().unwrap().smtp_tls);
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

    assert_eq!(app.channels_settings.telegram.as_ref().unwrap().draft_update_interval_ms, 100);
    assert_eq!(app.channels_settings.webhook.as_ref().unwrap().port, u16::MAX);
    assert_eq!(app.channels_settings.irc.as_ref().unwrap().port, 1);
    let lark = app.channels_settings.lark.as_ref().unwrap();
    assert_eq!(lark.port, Some(u16::MAX));
    assert_eq!(lark.draft_update_interval_ms, 100);
    assert_eq!(lark.max_draft_edits, 1);
    let feishu = app.channels_settings.feishu.as_ref().unwrap();
    assert_eq!(feishu.port, Some(1));
    assert_eq!(feishu.draft_update_interval_ms, 100);
    assert_eq!(feishu.max_draft_edits, 1);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let email = app.channels_settings.email.as_ref().unwrap();
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
        app.channels_settings.telegram.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.channels_settings.discord.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.channels_settings.slack.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        app.channels_settings.mattermost.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(app.channels_settings.lark.as_ref().unwrap().receive_mode, LarkReceiveMode::Webhook);
    assert_eq!(
        app.channels_settings.feishu.as_ref().unwrap().receive_mode,
        LarkReceiveMode::Websocket
    );
    assert_eq!(app.channels_settings.qq.as_ref().unwrap().receive_mode, QQReceiveMode::Webhook);
}

#[test]
fn disabled_channel_changes_are_ignored_and_group_modes_fallback_to_all_messages() {
    let mut app = app();

    apply_bool_change(&mut app, "telegram.mention_only", true);
    apply_number_change(&mut app, "webhook.port", 443);
    apply_receive_mode_change(&mut app, "lark.receive_mode", "webhook".to_string());
    assert!(app.channels_settings.telegram.is_none());
    assert!(app.channels_settings.webhook.is_none());
    assert!(app.channels_settings.lark.is_none());

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
        app.channels_settings.telegram.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.channels_settings.discord.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.channels_settings.slack.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.channels_settings.mattermost.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.channels_settings.lark.as_ref().unwrap().effective_group_reply_mode(),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        app.channels_settings.feishu.as_ref().unwrap().effective_group_reply_mode(),
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

    let feishu = &app.channels_settings.feishu_instances[0].config;
    assert_eq!(feishu.port, Some(1));
    assert_eq!(feishu.draft_update_interval_ms, 100);
}
