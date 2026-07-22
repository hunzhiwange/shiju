use super::chat::ChannelsConfigExt;
use super::other::{QQConfig, QQReceiveMode};
use super::{ChannelsConfig, TelegramConfig, WebhookConfig};

#[test]
fn channels_list_adds_webhook_after_regular_channels() {
    let config = ChannelsConfig::default();
    let regular_len = config.channels_except_webhook().len();
    let all = config.channels();

    assert_eq!(all.len(), regular_len + 1);
    assert_eq!(all.last().unwrap().0.name(), "Webhook");
    assert!(!all.last().unwrap().1);
}

#[test]
fn channels_except_webhook_reports_regular_channels_in_display_order() {
    let config = ChannelsConfig::default();
    let channels = config.channels_except_webhook();

    #[cfg(not(target_arch = "wasm32"))]
    let expected = vec![
        ("Telegram", "connect your bot", false),
        ("Discord", "connect your bot", false),
        ("Slack", "connect your bot", false),
        ("Mattermost", "connect to your bot", false),
        ("iMessage", "macOS only", false),
        ("Matrix", "self-hosted chat", false),
        ("Signal", "An open-source, encrypted messaging service", false),
        ("WhatsApp", "Business Cloud API", false),
        ("Linq", "iMessage/RCS/SMS via Linq API", false),
        ("WATI", "WhatsApp via WATI Business API", false),
        ("NextCloud Talk", "NextCloud Talk platform", false),
        ("Email", "Email over IMAP/SMTP", false),
        ("IRC", "IRC over TLS", false),
        ("Lark", "Lark Bot", false),
        ("Feishu", "Feishu Bot", false),
        ("DingTalk", "DingTalk Stream Mode", false),
        ("QQ Official", "Tencent QQ Bot", false),
        ("Nostr", "Nostr DMs", false),
        ("ClawdTalk", "ClawdTalk Channel", false),
    ];
    #[cfg(target_arch = "wasm32")]
    let expected = vec![
        ("Telegram", "connect your bot", false),
        ("Discord", "connect your bot", false),
        ("Slack", "connect your bot", false),
        ("Mattermost", "connect to your bot", false),
        ("iMessage", "macOS only", false),
        ("Matrix", "self-hosted chat", false),
        ("Signal", "An open-source, encrypted messaging service", false),
        ("WhatsApp", "Business Cloud API", false),
        ("Linq", "iMessage/RCS/SMS via Linq API", false),
        ("WATI", "WhatsApp via WATI Business API", false),
        ("NextCloud Talk", "NextCloud Talk platform", false),
        ("IRC", "IRC over TLS", false),
        ("Lark", "Lark Bot", false),
        ("Feishu", "Feishu Bot", false),
        ("DingTalk", "DingTalk Stream Mode", false),
        ("QQ Official", "Tencent QQ Bot", false),
        ("Nostr", "Nostr DMs", false),
        ("ClawdTalk", "ClawdTalk Channel", false),
    ];

    let actual: Vec<_> =
        channels.iter().map(|(handle, enabled)| (handle.name(), handle.desc(), *enabled)).collect();
    assert_eq!(actual, expected);
}

#[test]
fn channels_except_webhook_excludes_configured_webhook() {
    let config = ChannelsConfig {
        webhook: Some(WebhookConfig { port: 8787, secret: Some("secret".to_string()) }),
        ..Default::default()
    };

    assert!(config.channels_except_webhook().iter().all(|(handle, _)| handle.name() != "Webhook"));
}

#[test]
fn channels_list_reports_enabled_state_for_configured_channels() {
    let mut config = ChannelsConfig {
        telegram: Some(TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: Vec::new(),
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        webhook: Some(WebhookConfig { port: 8787, secret: None }),
        ..Default::default()
    };

    let channels = config.channels();
    assert!(channels.iter().any(|(handle, enabled)| handle.name() == "Telegram" && *enabled));
    assert!(channels.iter().any(|(handle, enabled)| handle.name() == "Webhook" && *enabled));

    config.qq = Some(QQConfig {
        app_id: "app".to_string(),
        app_secret: "secret".to_string(),
        allowed_users: Vec::new(),
        receive_mode: QQReceiveMode::Webhook,
    });
    assert!(
        config
            .channels_except_webhook()
            .iter()
            .any(|(handle, enabled)| handle.name() == "QQ Official" && !*enabled)
    );

    config.qq.as_mut().unwrap().receive_mode = QQReceiveMode::Websocket;
    assert!(
        config
            .channels_except_webhook()
            .iter()
            .any(|(handle, enabled)| handle.name() == "QQ Official" && *enabled)
    );
}
