use super::*;
use crate::app::agent::config::schema::{
    DingTalkConfig, DingTalkInstanceConfig, DiscordConfig, EmailConfig, FeishuConfig,
    FeishuInstanceConfig, GroupReplyConfig, GroupReplyMode, IMessageConfig, IrcConfig, LarkConfig,
    LarkReceiveMode, LinqConfig, MattermostConfig,
    NextcloudTalkConfig, NostrConfig, QQConfig, QQReceiveMode, SignalConfig, SlackConfig,
    StreamMode, TelegramConfig, WatiConfig, WhatsAppConfig,
};
use vw_config_types::channels::clawdtalk::ClawdTalkConfig;

fn channel_names(channels: &[ConfiguredChannel]) -> Vec<&str> {
    channels.iter().map(|channel| channel.display_name.as_ref()).collect()
}

fn whatsapp_config(
    access_token: Option<&str>,
    phone_number_id: Option<&str>,
    verify_token: Option<&str>,
    session_path: Option<&str>,
) -> WhatsAppConfig {
    WhatsAppConfig {
        access_token: access_token.map(str::to_string),
        phone_number_id: phone_number_id.map(str::to_string),
        verify_token: verify_token.map(str::to_string),
        app_secret: None,
        session_path: session_path.map(str::to_string),
        pair_phone: None,
        pair_code: None,
        allowed_numbers: vec!["+15551112222".to_string()],
    }
}


fn lark_config(use_feishu: bool) -> LarkConfig {
    LarkConfig {
        app_id: "app-id".to_string(),
        app_secret: "app-secret".to_string(),
        encrypt_key: None,
        verification_token: Some("verify".to_string()),
        allowed_users: vec!["user".to_string()],
        mention_only: false,
        group_reply: None,
        use_feishu,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

fn feishu_config() -> FeishuConfig {
    FeishuConfig {
        app_id: "feishu-app-id".to_string(),
        app_secret: "feishu-app-secret".to_string(),
        encrypt_key: None,
        verification_token: Some("verify".to_string()),
        allowed_users: vec!["user".to_string()],
        group_reply: None,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

fn dingtalk_config() -> DingTalkConfig {
    DingTalkConfig {
        client_id: "ding-client".to_string(),
        client_secret: "ding-secret".to_string(),
        allowed_users: vec!["dingtalk-user".to_string()],
    }
}

#[test]
fn collect_configured_channels_returns_empty_for_default_config() {
    let config = Config::default();

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[test]
fn collect_configured_channels_builds_common_native_channels() {
    let mut config = Config::default();
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "tg-token".to_string(),
        allowed_users: vec!["alice".to_string()],
        stream_mode: StreamMode::Partial,
        draft_update_interval_ms: 500,
        interrupt_on_new_message: true,
        mention_only: true,
        group_reply: Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::MentionOnly),
            allowed_sender_ids: vec!["admin".to_string()],
        }),
        base_url: Some("https://api.telegram.test".to_string()),
    });
    config.channels_config.discord = Some(DiscordConfig {
        bot_token: "discord-token".to_string(),
        guild_id: Some("guild".to_string()),
        allowed_users: vec!["bob".to_string()],
        listen_to_bots: true,
        mention_only: true,
        group_reply: Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::MentionOnly),
            allowed_sender_ids: vec!["mod".to_string()],
        }),
    });
    config.channels_config.imessage =
        Some(IMessageConfig { allowed_contacts: vec!["+15551234567".to_string()] });
    config.channels_config.linq = Some(LinqConfig {
        api_token: "linq-token".to_string(),
        from_phone: "+15550000000".to_string(),
        signing_secret: None,
        allowed_senders: vec!["+15551111111".to_string()],
    });
    config.channels_config.irc = Some(IrcConfig {
        server: "irc.example.test".to_string(),
        port: 6697,
        nickname: "vibe".to_string(),
        username: None,
        channels: vec!["#general".to_string()],
        allowed_users: vec!["alice".to_string()],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: Some(true),
    });

    let channels = collect_configured_channels(&config, "test");
    let names = channel_names(&channels);

    assert!(names.contains(&"Telegram"));
    assert!(names.contains(&"Discord"));
    assert!(names.contains(&"iMessage"));
    assert!(names.contains(&"Linq"));
    assert!(names.contains(&"IRC"));
}

#[test]
fn collect_configured_channels_builds_remaining_native_channels() {
    let mut config = Config::default();
    config.channels_config.slack = Some(SlackConfig {
        bot_token: "slack-token".to_string(),
        app_token: None,
        channel_id: Some("channel".to_string()),
        allowed_users: vec!["alice".to_string()],
        group_reply: Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::AllMessages),
            allowed_sender_ids: vec!["sender".to_string()],
        }),
    });
    config.channels_config.mattermost = Some(MattermostConfig {
        url: "https://mattermost.example.test".to_string(),
        bot_token: "mattermost-token".to_string(),
        channel_id: Some("town-square".to_string()),
        allowed_users: vec!["bob".to_string()],
        thread_replies: None,
        mention_only: Some(true),
        group_reply: Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::MentionOnly),
            allowed_sender_ids: vec!["lead".to_string()],
        }),
    });
    config.channels_config.signal = Some(SignalConfig {
        http_url: "http://127.0.0.1:8080".to_string(),
        account: "+15550000001".to_string(),
        group_id: Some("group".to_string()),
        allowed_from: vec!["+15550000002".to_string()],
        ignore_attachments: true,
        ignore_stories: true,
    });
    config.channels_config.whatsapp =
        Some(whatsapp_config(Some("token"), Some("phone"), Some("verify"), Some("/tmp/session")));
    config.channels_config.wati = Some(WatiConfig {
        api_token: "wati-token".to_string(),
        api_url: "https://wati.example.test".to_string(),
        tenant_id: Some("tenant".to_string()),
        allowed_numbers: vec!["+15550000003".to_string()],
    });
    config.channels_config.nextcloud_talk = Some(NextcloudTalkConfig {
        base_url: "https://nextcloud.example.test".to_string(),
        app_token: "nextcloud-token".to_string(),
        webhook_secret: Some("secret".to_string()),
        allowed_users: vec!["carol".to_string()],
    });
    config.channels_config.email = Some(EmailConfig {
        imap_host: "imap.example.test".to_string(),
        imap_port: 993,
        imap_folder: "INBOX".to_string(),
        smtp_host: "smtp.example.test".to_string(),
        smtp_port: 465,
        smtp_tls: true,
        username: "bot@example.test".to_string(),
        password: "password".to_string(),
        from_address: "bot@example.test".to_string(),
        idle_timeout_secs: 30,
        allowed_senders: vec!["sender@example.test".to_string()],
    });
    config.channels_config.dingtalk = Some(DingTalkConfig {
        client_id: "ding-client".to_string(),
        client_secret: "ding-secret".to_string(),
        allowed_users: vec!["dingtalk-user".to_string()],
    });
    config.channels_config.clawdtalk = Some(ClawdTalkConfig {
        api_key: "clawdtalk-key".to_string(),
        connection_id: "connection".to_string(),
        from_number: "+15550000004".to_string(),
        allowed_destinations: vec!["+1555".to_string()],
        webhook_secret: Some("webhook-secret".to_string()),
    });

    assert_eq!(
        channel_names(&collect_configured_channels(&config, "test")),
        vec![
            "Slack",
            "Mattermost",
            "Signal",
            "WhatsApp",
            "WATI",
            "Nextcloud Talk",
            "Email",
            "DingTalk",
            "ClawdTalk"
        ]
    );
}

#[test]
fn collect_configured_channels_skips_invalid_whatsapp_cloud_config() {
    let mut config = Config::default();
    config.channels_config.whatsapp = Some(whatsapp_config(None, Some("phone"), None, None));

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[test]
fn collect_configured_channels_skips_whatsapp_without_backend_selector() {
    let mut config = Config::default();
    config.channels_config.whatsapp = Some(whatsapp_config(None, None, None, None));

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[test]
fn collect_configured_channels_skips_incomplete_whatsapp_cloud_config() {
    // phone_number_id 已设置，但缺少 access_token / verify_token，不构成完整 Cloud 配置。
    let mut config = Config::default();
    config.channels_config.whatsapp = Some(whatsapp_config(None, Some("phone"), None, None));

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[test]
fn collect_configured_channels_prefers_cloud_for_ambiguous_whatsapp_config() {
    // 同时设置 phone_number_id 与 session_path 时记录歧义警告并优先 Cloud 模式。
    let mut config = Config::default();
    config.channels_config.whatsapp =
        Some(whatsapp_config(Some("token"), Some("phone"), Some("verify"), Some("/tmp/session")));

    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["WhatsApp"]);
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn collect_configured_channels_builds_whatsapp_web_when_feature_enabled() {
    let mut config = Config::default();
    config.channels_config.whatsapp = Some(whatsapp_config(None, None, None, Some("/tmp/session")));

    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["WhatsApp"]);
}

#[cfg(not(feature = "whatsapp-web"))]
#[test]
fn collect_configured_channels_skips_whatsapp_web_without_feature() {
    let mut config = Config::default();
    config.channels_config.whatsapp = Some(whatsapp_config(None, None, None, Some("/tmp/session")));

    assert!(collect_configured_channels(&config, "test").is_empty());
}







#[cfg(feature = "channel-lark")]
#[test]
fn collect_configured_channels_builds_lark_and_feishu_variants() {
    let mut config = Config::default();
    config.channels_config.lark = Some(lark_config(false));
    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["Lark"]);

    config.channels_config.lark = Some(lark_config(true));
    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["Feishu"]);

    config.channels_config.feishu = Some(feishu_config());
    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["Feishu"]);
}

#[cfg(feature = "channel-lark")]
#[test]
fn collect_configured_channels_skips_blank_and_partial_feishu_credentials() {
    let mut legacy_config = Config::default();
    let mut legacy_feishu = lark_config(true);
    legacy_feishu.app_secret = " ".to_string();
    legacy_config.channels_config.lark = Some(legacy_feishu);
    assert!(collect_configured_channels(&legacy_config, "test").is_empty());

    let mut config = Config::default();
    let mut blank_feishu = feishu_config();
    blank_feishu.app_id.clear();
    config.channels_config.feishu = Some(blank_feishu);
    config.channels_config.feishu_instances.push(FeishuInstanceConfig {
        id: "partial".to_string(),
        config: FeishuConfig { app_secret: "\t".to_string(), ..feishu_config() },
    });

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[cfg(not(feature = "channel-lark"))]
#[test]
fn collect_configured_channels_skips_lark_and_feishu_without_feature() {
    let mut config = Config::default();
    config.channels_config.lark = Some(lark_config(false));
    config.channels_config.feishu = Some(feishu_config());

    assert!(collect_configured_channels(&config, "test").is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn collect_configured_channels_skips_instance_channels_when_profiles_invalid() {
    // 不安全的实例 id（包含 ':'）使 resolve_channel_instance_profiles 失败，
    // 进而跳过受 profile 校验保护的 DingTalk（含其命名实例）。
    let mut config = Config::default();
    config.channels_config.dingtalk = Some(dingtalk_config());
    config
        .channels_config
        .dingtalk_instances
        .push(DingTalkInstanceConfig { id: "bad:id".to_string(), config: dingtalk_config() });
    // 同时配置一个不受 profile 校验保护的通道，确认其仍然被收集。
    config.channels_config.imessage =
        Some(IMessageConfig { allowed_contacts: vec!["+15551234567".to_string()] });

    assert_eq!(channel_names(&collect_configured_channels(&config, "test")), vec!["iMessage"]);
}

#[test]
fn collect_configured_channels_skips_qq_webhook_mode_and_keeps_websocket_mode() {
    let mut config = Config::default();
    config.channels_config.qq = Some(QQConfig {
        app_id: "app".to_string(),
        app_secret: "secret".to_string(),
        allowed_users: vec!["user".to_string()],
        receive_mode: QQReceiveMode::Webhook,
    });
    assert!(collect_configured_channels(&config, "test").is_empty());

    config.channels_config.qq = Some(QQConfig {
        app_id: "app".to_string(),
        app_secret: "secret".to_string(),
        allowed_users: vec!["user".to_string()],
        receive_mode: QQReceiveMode::Websocket,
    });
    let channels = collect_configured_channels(&config, "test");

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].display_name, "QQ");
}

#[tokio::test]
async fn append_nostr_channel_if_available_is_noop_when_not_configured() {
    let config = Config::default();
    let mut channels = Vec::new();

    assert_eq!(append_nostr_channel_if_available(&config, &mut channels, "test").await, None);
    assert!(channels.is_empty());
}

#[tokio::test]
async fn append_nostr_channel_if_available_adds_valid_configured_channel() {
    let mut config = Config::default();
    config.channels_config.nostr = Some(NostrConfig {
        private_key: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        relays: Vec::new(),
        allowed_pubkeys: vec!["*".to_string()],
    });
    let mut channels = Vec::new();

    assert_eq!(append_nostr_channel_if_available(&config, &mut channels, "test").await, None);
    assert_eq!(channel_names(&channels), vec!["Nostr"]);
}

#[tokio::test]
async fn append_nostr_channel_if_available_reports_invalid_private_key() {
    let mut config = Config::default();
    config.channels_config.nostr = Some(NostrConfig {
        private_key: "not-a-private-key".to_string(),
        relays: Vec::new(),
        allowed_pubkeys: Vec::new(),
    });
    let mut channels = Vec::new();

    let reason = append_nostr_channel_if_available(&config, &mut channels, "startup").await;

    assert!(channels.is_empty());
    assert!(reason.is_some_and(|message| message.contains("Nostr init failed during startup")));
}
