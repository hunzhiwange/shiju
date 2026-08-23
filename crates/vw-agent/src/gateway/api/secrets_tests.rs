// 验证配置中的敏感字段能够完整遮蔽，并仅从既有配置安全恢复。
use super::*;

use crate::app::agent::channels::email_channel::EmailConfig;
use crate::app::agent::config::schema::{
    CloudflareTunnelConfig, CustomTunnelConfig, DingTalkConfig, DiscordConfig,
    EmbeddingRouteConfig, FeishuConfig, IrcConfig, LarkConfig, LarkReceiveMode, LinqConfig,
    MatrixConfig, MattermostConfig, NextcloudTalkConfig, NgrokTunnelConfig, NostrConfig, QQConfig,
    QQReceiveMode, SlackConfig, StreamMode, TelegramConfig, WatiConfig, WebhookConfig,
    WhatsAppConfig,
};
use crate::app::agent::config::{Config, DelegateAgentConfig};
use vw_config_types::channels::ClawdTalkConfig;

fn secret(value: &str) -> Option<String> {
    Some(value.to_string())
}

fn full_config() -> Config {
    let mut config = Config::default();
    config.config_path = "/tmp/current/vibewindow.json".into();
    config.workspace_dir = "/tmp/current/workspace".into();
    config.api_key = secret("api-key");
    config.gateway.paired_tokens =
        vec!["paired-a".to_string(), String::new(), "paired-c".to_string()];
    config.proxy.http_proxy = secret("http://user:pass@proxy.local:8080");
    config.proxy.https_proxy = secret("https://user:pass@proxy.local:8443");
    config.proxy.all_proxy = secret("socks5://user:pass@proxy.local:1080");
    config.browser.computer_use.api_key = secret("computer-use-key");
    config.web_fetch.api_key = secret("web-fetch-key");
    config.web_search.api_key = secret("web-search-key");
    config.web_search.brave_api_key = secret("brave-key");
    config.storage.provider.config.db_url = secret("postgres://user:pass@localhost/db");
    config.tunnel.cloudflare = Some(CloudflareTunnelConfig { token: "cloudflare-token".into() });
    config.tunnel.ngrok =
        Some(NgrokTunnelConfig { auth_token: "ngrok-token".into(), domain: None });
    config.tunnel.custom = Some(CustomTunnelConfig {
        url: Some("https://tunnel.example".into()),
        auth_token: secret("custom-token"),
        start_command: "tunnel --port {port}".into(),
        health_url: None,
        url_pattern: None,
    });
    config.agents.insert(
        "coder".into(),
        DelegateAgentConfig { api_key: secret("agent-key"), ..DelegateAgentConfig::default() },
    );
    config.embedding_routes = vec![
        EmbeddingRouteConfig {
            hint: "semantic".into(),
            provider: "openai".into(),
            model: "text-embedding-3-small".into(),
            dimensions: Some(1536),
            api_key: secret("embedding-key"),
        },
        EmbeddingRouteConfig {
            hint: "public".into(),
            provider: "none".into(),
            model: "noop".into(),
            dimensions: None,
            api_key: None,
        },
    ];
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "telegram-token".into(),
        allowed_users: vec!["alice".into()],
        stream_mode: StreamMode::Off,
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    config.channels_config.discord = Some(DiscordConfig {
        bot_token: "discord-token".into(),
        guild_id: Some("guild".into()),
        allowed_users: vec!["alice".into()],
        listen_to_bots: false,
        mention_only: false,
        group_reply: None,
    });
    config.channels_config.slack = Some(SlackConfig {
        bot_token: "slack-token".into(),
        app_token: secret("slack-app-token"),
        channel_id: Some("channel".into()),
        allowed_users: vec!["alice".into()],
        group_reply: None,
    });
    config.channels_config.mattermost = Some(MattermostConfig {
        url: "https://mattermost.example".into(),
        bot_token: "mattermost-token".into(),
        channel_id: Some("channel".into()),
        allowed_users: vec!["alice".into()],
        thread_replies: None,
        mention_only: None,
        group_reply: None,
    });
    config.channels_config.webhook = Some(WebhookConfig { port: 42617, secret: secret("webhook") });
    config.channels_config.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.example".into(),
        access_token: "matrix-token".into(),
        user_id: None,
        device_id: None,
        room_id: "!room:example".into(),
        allowed_users: vec!["alice".into()],
        mention_only: false,
    });
    config.channels_config.whatsapp = Some(WhatsAppConfig {
        access_token: secret("whatsapp-token"),
        phone_number_id: Some("phone".into()),
        verify_token: secret("whatsapp-verify"),
        app_secret: secret("whatsapp-secret"),
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: vec!["*".into()],
    });
    config.channels_config.linq = Some(LinqConfig {
        api_token: "linq-token".into(),
        from_phone: "+15550101".into(),
        signing_secret: secret("linq-signing"),
        allowed_senders: vec!["*".into()],
    });
    config.channels_config.wati = Some(WatiConfig {
        api_token: "wati-token".into(),
        api_url: "https://live-mt-server.wati.io".into(),
        tenant_id: Some("tenant".into()),
        allowed_numbers: vec!["*".into()],
    });
    config.channels_config.nextcloud_talk = Some(NextcloudTalkConfig {
        base_url: "https://nextcloud.example".into(),
        app_token: "nextcloud-token".into(),
        webhook_secret: secret("nextcloud-webhook"),
        allowed_users: vec!["alice".into()],
    });
    config.channels_config.email = Some(EmailConfig {
        imap_host: "imap.example".into(),
        smtp_host: "smtp.example".into(),
        username: "bot@example".into(),
        password: "email-password".into(),
        from_address: "bot@example".into(),
        ..EmailConfig::default()
    });
    config.channels_config.irc = Some(IrcConfig {
        server: "irc.example".into(),
        port: 6697,
        nickname: "bot".into(),
        username: Some("bot".into()),
        channels: vec!["#ops".into()],
        allowed_users: vec!["alice".into()],
        server_password: secret("irc-server"),
        nickserv_password: secret("irc-nickserv"),
        sasl_password: secret("irc-sasl"),
        verify_tls: Some(true),
    });
    config.channels_config.lark = Some(LarkConfig {
        app_id: "lark-app".into(),
        app_secret: "lark-secret".into(),
        encrypt_key: secret("lark-encrypt"),
        verification_token: secret("lark-verify"),
        allowed_users: vec!["alice".into()],
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(42617),
        draft_update_interval_ms:
            crate::app::agent::config::schema::default_lark_draft_update_interval_ms(),
        max_draft_edits: crate::app::agent::config::schema::default_lark_max_draft_edits(),
    });
    config.channels_config.feishu = Some(FeishuConfig {
        app_id: "feishu-app".into(),
        app_secret: "feishu-secret".into(),
        encrypt_key: secret("feishu-encrypt"),
        verification_token: secret("feishu-verify"),
        allowed_users: vec!["alice".into()],
        group_reply: None,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(42618),
        draft_update_interval_ms:
            crate::app::agent::config::schema::default_lark_draft_update_interval_ms(),
        max_draft_edits: crate::app::agent::config::schema::default_lark_max_draft_edits(),
    });
    config.channels_config.dingtalk = Some(DingTalkConfig {
        client_id: "dingtalk-client".into(),
        client_secret: "dingtalk-secret".into(),
        allowed_users: vec!["alice".into()],
    });
    config.channels_config.qq = Some(QQConfig {
        app_id: "qq-app".into(),
        app_secret: "qq-secret".into(),
        allowed_users: vec!["alice".into()],
        receive_mode: QQReceiveMode::Webhook,
    });
    config.channels_config.nostr = Some(NostrConfig {
        private_key: "nostr-private".into(),
        relays: vec!["wss://relay.example".into()],
        allowed_pubkeys: vec!["alice".into()],
    });
    config.channels_config.clawdtalk = Some(ClawdTalkConfig {
        api_key: "clawdtalk-key".into(),
        connection_id: "conn".into(),
        from_number: "+15550102".into(),
        allowed_destinations: vec!["*".into()],
        webhook_secret: secret("clawdtalk-webhook"),
    });
    config
}

fn assert_all_config_secrets_masked(config: &Config) {
    assert_eq!(config.api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.gateway.paired_tokens,
        vec![MASKED_SECRET.to_string(), String::new(), MASKED_SECRET.to_string()]
    );
    assert_eq!(config.proxy.http_proxy.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.proxy.https_proxy.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.proxy.all_proxy.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.browser.computer_use.api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.web_fetch.api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.web_search.api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.web_search.brave_api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.storage.provider.config.db_url.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.tunnel.cloudflare.as_ref().map(|value| value.token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.tunnel.ngrok.as_ref().map(|value| value.auth_token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.tunnel.custom.as_ref().and_then(|value| value.auth_token.as_deref()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.agents.get("coder").and_then(|value| value.api_key.as_deref()),
        Some(MASKED_SECRET)
    );
    assert_eq!(config.embedding_routes[0].api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(config.embedding_routes[1].api_key, None);
    assert_eq!(
        config.channels_config.telegram.as_ref().map(|value| value.bot_token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.channels_config.discord.as_ref().map(|value| value.bot_token.as_str()),
        Some(MASKED_SECRET)
    );
    let slack = config.channels_config.slack.as_ref().expect("slack config");
    assert_eq!(slack.bot_token, MASKED_SECRET);
    assert_eq!(slack.app_token.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.channels_config.mattermost.as_ref().map(|value| value.bot_token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.channels_config.webhook.as_ref().and_then(|value| value.secret.as_deref()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.channels_config.matrix.as_ref().map(|value| value.access_token.as_str()),
        Some(MASKED_SECRET)
    );
    let whatsapp = config.channels_config.whatsapp.as_ref().expect("whatsapp config");
    assert_eq!(whatsapp.access_token.as_deref(), Some(MASKED_SECRET));
    assert_eq!(whatsapp.app_secret.as_deref(), Some(MASKED_SECRET));
    assert_eq!(whatsapp.verify_token.as_deref(), Some(MASKED_SECRET));
    let linq = config.channels_config.linq.as_ref().expect("linq config");
    assert_eq!(linq.api_token, MASKED_SECRET);
    assert_eq!(linq.signing_secret.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.channels_config.wati.as_ref().map(|value| value.api_token.as_str()),
        Some(MASKED_SECRET)
    );
    let nextcloud = config.channels_config.nextcloud_talk.as_ref().expect("nextcloud talk config");
    assert_eq!(nextcloud.app_token, MASKED_SECRET);
    assert_eq!(nextcloud.webhook_secret.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.channels_config.email.as_ref().map(|value| value.password.as_str()),
        Some(MASKED_SECRET)
    );
    let irc = config.channels_config.irc.as_ref().expect("irc config");
    assert_eq!(irc.server_password.as_deref(), Some(MASKED_SECRET));
    assert_eq!(irc.nickserv_password.as_deref(), Some(MASKED_SECRET));
    assert_eq!(irc.sasl_password.as_deref(), Some(MASKED_SECRET));
    let lark = config.channels_config.lark.as_ref().expect("lark config");
    assert_eq!(lark.app_secret, MASKED_SECRET);
    assert_eq!(lark.encrypt_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(lark.verification_token.as_deref(), Some(MASKED_SECRET));
    let feishu = config.channels_config.feishu.as_ref().expect("feishu config");
    assert_eq!(feishu.app_secret, MASKED_SECRET);
    assert_eq!(feishu.encrypt_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(feishu.verification_token.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        config.channels_config.dingtalk.as_ref().map(|value| value.client_secret.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.channels_config.qq.as_ref().map(|value| value.app_secret.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        config.channels_config.nostr.as_ref().map(|value| value.private_key.as_str()),
        Some(MASKED_SECRET)
    );
    let clawdtalk = config.channels_config.clawdtalk.as_ref().expect("clawdtalk config");
    assert_eq!(clawdtalk.api_key, MASKED_SECRET);
    assert_eq!(clawdtalk.webhook_secret.as_deref(), Some(MASKED_SECRET));
}

#[test]
fn mask_helpers_handle_present_empty_and_absent_values() {
    assert!(is_masked_secret(MASKED_SECRET));
    assert!(!is_masked_secret("secret"));

    let mut optional = secret("secret");
    mask_optional_secret(&mut optional);
    assert_eq!(optional.as_deref(), Some(MASKED_SECRET));

    let mut empty_optional = secret("");
    mask_optional_secret(&mut empty_optional);
    assert_eq!(empty_optional.as_deref(), Some(MASKED_SECRET));

    let mut absent = None;
    mask_optional_secret(&mut absent);
    assert_eq!(absent, None);

    let mut required = "secret".to_string();
    mask_required_secret(&mut required);
    assert_eq!(required, MASKED_SECRET);

    let mut empty_required = String::new();
    mask_required_secret(&mut empty_required);
    assert_eq!(empty_required, "");

    let mut values = vec!["one".to_string(), String::new()];
    mask_vec_secrets(&mut values);
    assert_eq!(values, vec![MASKED_SECRET.to_string(), String::new()]);
}

#[test]
fn restore_helpers_only_replace_masked_values() {
    let mut optional = Some(MASKED_SECRET.to_string());
    restore_optional_secret(&mut optional, &secret("current"));
    assert_eq!(optional.as_deref(), Some("current"));

    let mut optional_to_none = Some(MASKED_SECRET.to_string());
    restore_optional_secret(&mut optional_to_none, &None);
    assert_eq!(optional_to_none, None);

    let mut changed_optional = secret("changed");
    restore_optional_secret(&mut changed_optional, &secret("current"));
    assert_eq!(changed_optional.as_deref(), Some("changed"));

    let mut absent_optional = None;
    restore_optional_secret(&mut absent_optional, &secret("current"));
    assert_eq!(absent_optional, None);

    let mut required = MASKED_SECRET.to_string();
    restore_required_secret(&mut required, "current");
    assert_eq!(required, "current");

    let mut changed_required = "changed".to_string();
    restore_required_secret(&mut changed_required, "current");
    assert_eq!(changed_required, "changed");

    let mut values = vec![MASKED_SECRET.to_string(), "changed".to_string(), MASKED_SECRET.into()];
    restore_vec_secrets(&mut values, &["first".to_string()]);
    assert_eq!(values, vec!["first".to_string(), "changed".to_string(), MASKED_SECRET.to_string()]);
}

#[test]
fn mask_sensitive_fields_masks_every_configured_secret_without_mutating_source() {
    let config = full_config();
    let masked = mask_sensitive_fields(&config);

    assert_eq!(config.api_key.as_deref(), Some("api-key"));
    assert_eq!(
        config.channels_config.telegram.as_ref().map(|value| value.bot_token.as_str()),
        Some("telegram-token")
    );
    assert_all_config_secrets_masked(&masked);
}

#[test]
fn hydrate_config_for_save_restores_every_configured_secret() {
    let current = full_config();
    let incoming = mask_sensitive_fields(&current);

    let restored = hydrate_config_for_save(incoming, &current);

    assert_eq!(
        serde_json::to_value(&restored).expect("restored config should serialize"),
        serde_json::to_value(&current).expect("current config should serialize")
    );
}

#[test]
fn hydrate_config_for_save_restores_masked_fields_and_preserves_user_changes() {
    let current = full_config();
    let mut incoming = mask_sensitive_fields(&current);
    incoming.config_path = "/tmp/incoming/ignored.json".into();
    incoming.workspace_dir = "/tmp/incoming/ignored".into();
    incoming.default_model = Some("new-model".into());
    incoming.web_fetch.api_key = secret("new-web-fetch");
    incoming.agents.insert(
        "new-agent".into(),
        DelegateAgentConfig {
            provider: "openrouter".into(),
            model: "model".into(),
            api_key: Some(MASKED_SECRET.into()),
            ..DelegateAgentConfig::default()
        },
    );
    incoming.embedding_routes.push(EmbeddingRouteConfig {
        hint: "unknown".into(),
        provider: "openai".into(),
        model: "text-embedding-3-large".into(),
        dimensions: None,
        api_key: Some(MASKED_SECRET.into()),
    });
    incoming.channels_config.telegram = Some(TelegramConfig {
        bot_token: "new-telegram-token".into(),
        allowed_users: vec!["bob".into()],
        stream_mode: StreamMode::Partial,
        draft_update_interval_ms: 250,
        interrupt_on_new_message: true,
        mention_only: true,
        group_reply: None,
        base_url: None,
    });

    let restored = hydrate_config_for_save(incoming, &current);

    assert_eq!(restored.config_path, current.config_path);
    assert_eq!(restored.workspace_dir, current.workspace_dir);
    assert_eq!(restored.default_model.as_deref(), Some("new-model"));
    assert_eq!(restored.web_fetch.api_key.as_deref(), Some("new-web-fetch"));
    assert_eq!(
        restored.agents.get("coder").and_then(|value| value.api_key.as_deref()),
        Some("agent-key")
    );
    assert_eq!(
        restored.agents.get("new-agent").and_then(|value| value.api_key.as_deref()),
        Some(MASKED_SECRET)
    );
    assert_eq!(restored.embedding_routes[0].api_key.as_deref(), Some("embedding-key"));
    assert_eq!(restored.embedding_routes[2].api_key.as_deref(), Some(MASKED_SECRET));
    assert_eq!(
        restored.channels_config.telegram.as_ref().map(|value| value.bot_token.as_str()),
        Some("new-telegram-token")
    );
    assert_eq!(
        restored.channels_config.discord.as_ref().map(|value| value.bot_token.as_str()),
        Some("discord-token")
    );
    assert_eq!(
        restored.channels_config.clawdtalk.as_ref().map(|value| value.api_key.as_str()),
        Some("clawdtalk-key")
    );
}

#[test]
fn hydrate_config_for_save_matches_embedding_routes_by_hint_after_reordering() {
    let mut current = full_config();
    current.embedding_routes[1].api_key = secret("public-embedding-key");
    let mut incoming = mask_sensitive_fields(&current);
    incoming.embedding_routes.swap(0, 1);

    let restored = hydrate_config_for_save(incoming, &current);

    assert_eq!(restored.embedding_routes[0].hint, "public");
    assert_eq!(restored.embedding_routes[0].api_key.as_deref(), Some("public-embedding-key"));
    assert_eq!(restored.embedding_routes[1].hint, "semantic");
    assert_eq!(restored.embedding_routes[1].api_key.as_deref(), Some("embedding-key"));
}

#[test]
fn restore_masked_sensitive_fields_leaves_missing_current_sections_masked() {
    let mut incoming = mask_sensitive_fields(&full_config());
    let current = Config::default();

    restore_masked_sensitive_fields(&mut incoming, &current);

    assert_eq!(
        incoming.tunnel.cloudflare.as_ref().map(|value| value.token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        incoming.channels_config.discord.as_ref().map(|value| value.bot_token.as_str()),
        Some(MASKED_SECRET)
    );
    assert_eq!(
        incoming.channels_config.clawdtalk.as_ref().map(|value| value.api_key.as_str()),
        Some(MASKED_SECRET)
    );
}
