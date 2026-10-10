//! 验证各扩展聊天通道的默认值、兼容格式、后端选择与必填字段约束。
//! 覆盖云端/本地后端判定、具名实例扁平化及各协议的序列化兼容面。

// 此哨兵保护拆分后的扩展通道测试文件仍由父模块编译。
#[test]
fn task_618_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("other_tests.rs"));
}

use super::{
    DingTalkConfig, IrcConfig, LinqConfig, NextcloudTalkConfig, NostrConfig, QQConfig, QQReceiveMode,
    WatiConfig, WhatsAppConfig, default_nostr_relays,
};

// WhatsApp 两种后端依赖互斥凭据组，不完整或同时配置时都不能误判。
#[test]
fn whatsapp_backend_helpers_cover_cloud_web_incomplete_and_ambiguous_configs() {
    let empty: WhatsAppConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    let incomplete_cloud: WhatsAppConfig = serde_json::from_value(serde_json::json!({
        "phone_number_id": "phone-id",
        "access_token": "test-access-token"
    }))
    .unwrap();
    let cloud: WhatsAppConfig = serde_json::from_value(serde_json::json!({
        "phone_number_id": "phone-id",
        "access_token": "test-access-token",
        "verify_token": "test-verify-token"
    }))
    .unwrap();
    let web: WhatsAppConfig = serde_json::from_value(serde_json::json!({
        "session_path": "/tmp/test-whatsapp-session"
    }))
    .unwrap();
    let ambiguous: WhatsAppConfig = serde_json::from_value(serde_json::json!({
        "phone_number_id": "phone-id",
        "access_token": "test-access-token",
        "verify_token": "test-verify-token",
        "session_path": "/tmp/test-whatsapp-session"
    }))
    .unwrap();

    assert_eq!(empty.backend_type(), "cloud");
    assert!(!empty.is_cloud_config());
    assert!(!empty.is_web_config());
    assert!(!empty.is_ambiguous_config());
    assert_eq!(incomplete_cloud.backend_type(), "cloud");
    assert!(!incomplete_cloud.is_cloud_config());
    assert!(cloud.is_cloud_config());
    assert!(!cloud.is_web_config());
    assert_eq!(web.backend_type(), "web");
    assert!(!web.is_cloud_config());
    assert!(web.is_web_config());
    assert_eq!(ambiguous.backend_type(), "cloud");
    assert!(ambiguous.is_cloud_config());
    assert!(ambiguous.is_web_config());
    assert!(ambiguous.is_ambiguous_config());
}


#[test]
fn whatsapp_deserialization_defaults_every_optional_field_and_allowed_numbers() {
    let config: WhatsAppConfig = serde_json::from_value(serde_json::json!({})).unwrap();

    assert!(config.access_token.is_none());
    assert!(config.phone_number_id.is_none());
    assert!(config.verify_token.is_none());
    assert!(config.app_secret.is_none());
    assert!(config.session_path.is_none());
    assert!(config.pair_phone.is_none());
    assert!(config.pair_code.is_none());
    assert!(config.allowed_numbers.is_empty());
}

#[test]
fn wati_and_irc_deserialization_apply_service_defaults_and_preserve_overrides() {
    let wati: WatiConfig = serde_json::from_value(serde_json::json!({
        "api_token": "test-wati-token"
    }))
    .unwrap();
    let default_irc: IrcConfig = serde_json::from_value(serde_json::json!({
        "server": "irc.example",
        "nickname": "vibe-window"
    }))
    .unwrap();
    let overridden_irc: IrcConfig = serde_json::from_value(serde_json::json!({
        "server": "irc.example",
        "port": 6667,
        "nickname": "vibe-window",
        "channels": ["#agents"],
        "allowed_users": ["alice"],
        "verify_tls": false
    }))
    .unwrap();

    assert_eq!(wati.api_url, "https://live-mt-server.wati.io");
    assert!(wati.tenant_id.is_none());
    assert!(wati.allowed_numbers.is_empty());
    assert_eq!(default_irc.port, 6697);
    assert!(default_irc.channels.is_empty());
    assert!(default_irc.allowed_users.is_empty());
    assert!(default_irc.username.is_none());
    assert!(default_irc.server_password.is_none());
    assert!(default_irc.nickserv_password.is_none());
    assert!(default_irc.sasl_password.is_none());
    assert!(default_irc.verify_tls.is_none());
    assert_eq!(overridden_irc.port, 6667);
    assert_eq!(overridden_irc.channels, vec!["#agents".to_string()]);
    assert_eq!(overridden_irc.allowed_users, vec!["alice".to_string()]);
    assert_eq!(overridden_irc.verify_tls, Some(false));
}

// 简单通道共享空允许列表语义，防止不同配置类型出现不一致缺省行为。
#[test]
fn simple_channel_configs_default_optional_and_allowlist_fields() {
    let linq: LinqConfig = serde_json::from_value(serde_json::json!({
        "api_token": "test-linq-token",
        "from_phone": "+15550000000"
    }))
    .unwrap();
    let nextcloud: NextcloudTalkConfig = serde_json::from_value(serde_json::json!({
        "base_url": "https://nextcloud.example",
        "app_token": "test-nextcloud-token"
    }))
    .unwrap();
    let dingtalk: DingTalkConfig = serde_json::from_value(serde_json::json!({
        "client_id": "test-client-id",
        "client_secret": "test-client-secret"
    }))
    .unwrap();

    assert!(linq.signing_secret.is_none());
    assert!(linq.allowed_senders.is_empty());
    assert!(nextcloud.webhook_secret.is_none());
    assert!(nextcloud.allowed_users.is_empty());
    assert!(dingtalk.allowed_users.is_empty());
}


// QQ 模式采用稳定外部名称，未知值必须报错而不是静默回退。
#[test]
fn qq_receive_mode_defaults_renames_variants_and_rejects_unknown_values() {
    let default_config: QQConfig = serde_json::from_value(serde_json::json!({
        "app_id": "test-app-id",
        "app_secret": "test-app-secret"
    }))
    .unwrap();
    let websocket: QQConfig = serde_json::from_value(serde_json::json!({
        "app_id": "test-app-id",
        "app_secret": "test-app-secret",
        "receive_mode": "websocket"
    }))
    .unwrap();
    let invalid = serde_json::from_value::<QQConfig>(serde_json::json!({
        "app_id": "test-app-id",
        "app_secret": "test-app-secret",
        "receive_mode": "polling"
    }));

    assert_eq!(default_config.receive_mode, QQReceiveMode::Webhook);
    assert!(default_config.allowed_users.is_empty());
    assert_eq!(websocket.receive_mode, QQReceiveMode::Websocket);
    assert_eq!(serde_json::to_value(QQReceiveMode::Webhook).unwrap(), "webhook");
    assert!(invalid.is_err());
}

// 未提供中继与显式空列表语义不同：前者使用推荐值，后者尊重用户禁用。
#[test]
fn nostr_relays_use_documented_defaults_but_preserve_an_explicit_empty_list() {
    let expected = vec![
        "wss://relay.damus.io".to_string(),
        "wss://nos.lol".to_string(),
        "wss://relay.primal.net".to_string(),
        "wss://relay.snort.social".to_string(),
    ];
    let default_config: NostrConfig = serde_json::from_value(serde_json::json!({
        "private_key": "test-private-key"
    }))
    .unwrap();
    let explicit_empty: NostrConfig = serde_json::from_value(serde_json::json!({
        "private_key": "test-private-key",
        "relays": []
    }))
    .unwrap();

    assert_eq!(default_nostr_relays(), expected);
    assert_eq!(default_config.relays, expected);
    assert!(default_config.allowed_pubkeys.is_empty());
    assert!(explicit_empty.relays.is_empty());
}
