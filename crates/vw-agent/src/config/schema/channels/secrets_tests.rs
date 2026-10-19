// 覆盖渠道密钥的加解密往返、可选字段与错误上下文。

use super::{ChannelsConfig, decrypt_channel_secrets, encrypt_channel_secrets};
use crate::app::agent::security::SecretStore;

// 启用与禁用加密的夹具并列存在，用于确认明文模式不会被意外改写。
#[test]
fn channel_secret_round_trip_is_noop_when_encryption_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), false);
    let mut channels = ChannelsConfig::default();

    encrypt_channel_secrets(&store, &mut channels).unwrap();
    decrypt_channel_secrets(&store, &mut channels).unwrap();

    assert!(channels.telegram.is_none());
}

#[test]
fn wecom_secret_round_trip_encrypts_at_rest() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "wecom": {"bot_id": "bot-id", "secret": "plain-secret"}
    }))
    .unwrap();

    encrypt_channel_secrets(&store, &mut channels).unwrap();
    let encrypted = &channels.wecom.as_ref().unwrap().secret;
    assert!(SecretStore::is_encrypted(encrypted));
    assert!(!encrypted.contains("plain-secret"));

    decrypt_channel_secrets(&store, &mut channels).unwrap();
    assert_eq!(channels.wecom.unwrap().secret, "plain-secret");
}

#[test]
fn channel_secret_round_trip_encrypts_remaining_channel_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "discord": {"bot_token": "discord-token"},
        "mattermost": {
            "url": "https://mattermost.example",
            "bot_token": "mattermost-token"
        },
        "linq": {
            "api_token": "linq-token",
            "from_phone": "+10000000001",
            "signing_secret": "linq-signing"
        },
        "nextcloud_talk": {
            "base_url": "https://nextcloud.example",
            "app_token": "nextcloud-token",
            "webhook_secret": "nextcloud-webhook"
        }
    }))
    .unwrap();

    encrypt_channel_secrets(&store, &mut channels).unwrap();

    assert!(SecretStore::is_encrypted(&channels.discord.as_ref().unwrap().bot_token));
    assert!(SecretStore::is_encrypted(&channels.mattermost.as_ref().unwrap().bot_token));
    assert!(SecretStore::is_encrypted(&channels.linq.as_ref().unwrap().api_token));
    assert!(SecretStore::is_encrypted(
        channels.linq.as_ref().unwrap().signing_secret.as_ref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(&channels.nextcloud_talk.as_ref().unwrap().app_token));
    assert!(SecretStore::is_encrypted(
        channels.nextcloud_talk.as_ref().unwrap().webhook_secret.as_ref().unwrap()
    ));

    decrypt_channel_secrets(&store, &mut channels).unwrap();

    assert_eq!(channels.discord.unwrap().bot_token, "discord-token");
    assert_eq!(channels.mattermost.unwrap().bot_token, "mattermost-token");
    let linq = channels.linq.unwrap();
    assert_eq!(linq.api_token, "linq-token");
    assert_eq!(linq.signing_secret.as_deref(), Some("linq-signing"));
    let nextcloud = channels.nextcloud_talk.unwrap();
    assert_eq!(nextcloud.app_token, "nextcloud-token");
    assert_eq!(nextcloud.webhook_secret.as_deref(), Some("nextcloud-webhook"));
}

#[test]
fn channel_secret_round_trip_preserves_absent_optional_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "slack": {"bot_token": "slack-token"},
        "webhook": {"port": 8080},
        "whatsapp": {},
        "linq": {"api_token": "linq-token", "from_phone": "+10000000001"},
        "nextcloud_talk": {
            "base_url": "https://nextcloud.example",
            "app_token": "nextcloud-token"
        },
        "irc": {"server": "irc.example", "nickname": "bot"},
        "lark": {"app_id": "app", "app_secret": "lark-secret"},
        "clawdtalk": {
            "api_key": "clawd-key",
            "connection_id": "conn",
            "from_number": "+10000000000"
        }
    }))
    .unwrap();

    encrypt_channel_secrets(&store, &mut channels).unwrap();
    decrypt_channel_secrets(&store, &mut channels).unwrap();

    assert_eq!(channels.slack.unwrap().app_token, None);
    assert_eq!(channels.webhook.unwrap().secret, None);
    let whatsapp = channels.whatsapp.unwrap();
    assert_eq!(whatsapp.access_token, None);
    assert_eq!(whatsapp.app_secret, None);
    assert_eq!(whatsapp.verify_token, None);
    assert_eq!(channels.linq.unwrap().signing_secret, None);
    assert_eq!(channels.nextcloud_talk.unwrap().webhook_secret, None);
    let irc = channels.irc.unwrap();
    assert_eq!(irc.server_password, None);
    assert_eq!(irc.nickserv_password, None);
    assert_eq!(irc.sasl_password, None);
    let lark = channels.lark.unwrap();
    assert_eq!(lark.encrypt_key, None);
    assert_eq!(lark.verification_token, None);
    assert_eq!(channels.clawdtalk.unwrap().webhook_secret, None);
}

#[test]
fn channel_secret_decrypt_error_identifies_legacy_optional_field() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "slack": {
            "bot_token": "plain-token",
            "app_token": "enc2:not-hex"
        }
    }))
    .unwrap();

    let err = decrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(err.contains("Failed to decrypt config.channels_config.slack.app_token"));
    assert_eq!(channels.slack.unwrap().app_token.as_deref(), Some("enc2:not-hex"));
}

#[test]
fn channel_secret_decrypt_error_identifies_legacy_required_field() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "discord": {"bot_token": "enc2:not-hex"}
    }))
    .unwrap();

    let err = decrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(err.contains("Failed to decrypt config.channels_config.discord.bot_token"));
    assert_eq!(channels.discord.unwrap().bot_token, "enc2:not-hex");
}

#[test]
fn channel_secret_decrypt_error_identifies_named_instance_optional_field() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "feishu_instances": [{
            "id": "cn",
            "app_id": "feishu-app",
            "app_secret": "plain-secret",
            "encrypt_key": "enc2:not-hex"
        }]
    }))
    .unwrap();

    let err = decrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(
        err.contains("Failed to decrypt config.channels_config.feishu_instances.cn.encrypt_key")
    );
    assert_eq!(channels.feishu_instances[0].config.encrypt_key.as_deref(), Some("enc2:not-hex"));
}

#[test]
fn channel_secret_encrypt_error_identifies_legacy_field() {
    let tmp = tempfile::tempdir().unwrap();
    let blocked_path = tmp.path().join("blocked");
    std::fs::write(&blocked_path, "not a directory").unwrap();
    let store = SecretStore::new(&blocked_path, true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "telegram": {"bot_token": "telegram-token", "allowed_users": []}
    }))
    .unwrap();

    let err = encrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(err.contains("Failed to encrypt config.channels_config.telegram.bot_token"));
    assert_eq!(channels.telegram.unwrap().bot_token, "telegram-token");
}

#[test]
fn channel_secret_encrypt_error_identifies_named_instance_field() {
    let tmp = tempfile::tempdir().unwrap();
    let blocked_path = tmp.path().join("blocked");
    std::fs::write(&blocked_path, "not a directory").unwrap();
    let store = SecretStore::new(&blocked_path, true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "dingtalk_instances": [{
            "id": "ops",
            "client_id": "dingtalk-client",
            "client_secret": "dingtalk-secret"
        }]
    }))
    .unwrap();

    let err = encrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(
        err.contains(
            "Failed to encrypt config.channels_config.dingtalk_instances.ops.client_secret"
        )
    );
    assert_eq!(channels.dingtalk_instances[0].config.client_secret, "dingtalk-secret");
}
