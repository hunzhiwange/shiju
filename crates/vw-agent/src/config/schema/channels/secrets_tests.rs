use super::{ChannelsConfig, decrypt_channel_secrets, encrypt_channel_secrets};
use crate::app::agent::security::SecretStore;

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
fn channel_secret_round_trip_encrypts_representative_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "telegram": {"bot_token": "telegram-token", "allowed_users": []},
        "slack": {"bot_token": "slack-token", "app_token": "slack-app"},
        "webhook": {"port": 8080, "secret": "webhook-secret"},
        "matrix": {
            "homeserver": "https://matrix.example",
            "access_token": "matrix-token",
            "room_id": "!room:example",
            "allowed_users": []
        },
        "whatsapp": {
            "access_token": "wa-token",
            "app_secret": "wa-secret",
            "verify_token": "wa-verify"
        },
        "irc": {
            "server": "irc.example",
            "nickname": "bot",
            "username": "bot",
            "server_password": "server-pass",
            "nickserv_password": "nick-pass",
            "sasl_password": "sasl-pass"
        },
        "lark": {
            "app_id": "app",
            "app_secret": "lark-secret",
            "encrypt_key": "lark-encrypt",
            "verification_token": "lark-verify"
        },
        "dingtalk": {"client_id": "client", "client_secret": "ding-secret"},
        "qq": {"app_id": "qq", "app_secret": "qq-secret"},
        "nostr": {"private_key": "nostr-secret"},
        "clawdtalk": {
            "api_key": "clawd-key",
            "connection_id": "conn",
            "from_number": "+10000000000",
            "webhook_secret": "clawd-webhook"
        }
    }))
    .unwrap();

    encrypt_channel_secrets(&store, &mut channels).unwrap();

    assert!(SecretStore::is_encrypted(&channels.telegram.as_ref().unwrap().bot_token));
    assert!(SecretStore::is_encrypted(
        channels.slack.as_ref().unwrap().app_token.as_ref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(channels.webhook.as_ref().unwrap().secret.as_ref().unwrap()));
    assert!(SecretStore::is_encrypted(&channels.matrix.as_ref().unwrap().access_token));
    assert!(SecretStore::is_encrypted(
        channels.irc.as_ref().unwrap().sasl_password.as_ref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(&channels.lark.as_ref().unwrap().app_secret));
    assert!(SecretStore::is_encrypted(&channels.dingtalk.as_ref().unwrap().client_secret));
    assert!(SecretStore::is_encrypted(&channels.qq.as_ref().unwrap().app_secret));
    assert!(SecretStore::is_encrypted(&channels.nostr.as_ref().unwrap().private_key));

    decrypt_channel_secrets(&store, &mut channels).unwrap();

    assert_eq!(channels.telegram.unwrap().bot_token, "telegram-token");
    assert_eq!(channels.slack.unwrap().app_token.as_deref(), Some("slack-app"));
    assert_eq!(channels.irc.unwrap().sasl_password.as_deref(), Some("sasl-pass"));
    assert_eq!(channels.clawdtalk.unwrap().webhook_secret.as_deref(), Some("clawd-webhook"));
}

#[test]
fn channel_instance_secret_round_trip_encrypts_named_instances() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "matrix_instances": [{
            "id": "planner",
            "homeserver": "https://matrix.example",
            "access_token": "matrix-token",
            "room_id": "!room:example",
            "allowed_users": []
        }],
        "feishu_instances": [{
            "id": "cn",
            "app_id": "feishu-app",
            "app_secret": "feishu-secret",
            "encrypt_key": "feishu-encrypt",
            "verification_token": "feishu-verify"
        }],
        "dingtalk_instances": [{
            "id": "1",
            "client_id": "dingtalk-client",
            "client_secret": "dingtalk-secret"
        }]
    }))
    .unwrap();

    encrypt_channel_secrets(&store, &mut channels).unwrap();

    assert!(SecretStore::is_encrypted(&channels.matrix_instances[0].config.access_token));
    assert!(SecretStore::is_encrypted(&channels.feishu_instances[0].config.app_secret));
    assert!(SecretStore::is_encrypted(
        channels.feishu_instances[0].config.encrypt_key.as_ref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(
        channels.feishu_instances[0].config.verification_token.as_ref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(&channels.dingtalk_instances[0].config.client_secret));

    decrypt_channel_secrets(&store, &mut channels).unwrap();

    assert_eq!(channels.matrix_instances[0].config.access_token, "matrix-token");
    assert_eq!(channels.feishu_instances[0].config.app_secret, "feishu-secret");
    assert_eq!(channels.feishu_instances[0].config.encrypt_key.as_deref(), Some("feishu-encrypt"));
    assert_eq!(
        channels.feishu_instances[0].config.verification_token.as_deref(),
        Some("feishu-verify")
    );
    assert_eq!(channels.dingtalk_instances[0].config.client_secret, "dingtalk-secret");
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
fn channel_secret_decrypt_error_identifies_named_instance_field() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "matrix_instances": [{
            "id": "planner",
            "homeserver": "https://matrix.example",
            "access_token": "enc2:not-hex",
            "room_id": "!room:example",
            "allowed_users": []
        }]
    }))
    .unwrap();

    let err = decrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

    assert!(err.contains(
        "Failed to decrypt config.channels_config.matrix_instances.planner.access_token"
    ));
    assert_eq!(channels.matrix_instances[0].config.access_token, "enc2:not-hex");
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
fn channel_secret_decrypt_errors_identify_additional_legacy_fields() {
    for (payload, expected_path) in [
        (
            serde_json::json!({
                "cli": true,
                "telegram": {"bot_token": "enc2:not-hex", "allowed_users": []}
            }),
            "config.channels_config.telegram.bot_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "slack": {"bot_token": "enc2:not-hex"}
            }),
            "config.channels_config.slack.bot_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "mattermost": {
                    "url": "https://mattermost.example",
                    "bot_token": "enc2:not-hex"
                }
            }),
            "config.channels_config.mattermost.bot_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "webhook": {"port": 8080, "secret": "enc2:not-hex"}
            }),
            "config.channels_config.webhook.secret",
        ),
        (
            serde_json::json!({
                "cli": true,
                "matrix": {
                    "homeserver": "https://matrix.example",
                    "access_token": "enc2:not-hex",
                    "room_id": "!room:example",
                    "allowed_users": []
                }
            }),
            "config.channels_config.matrix.access_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "whatsapp": {"access_token": "enc2:not-hex"}
            }),
            "config.channels_config.whatsapp.access_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "linq": {
                    "api_token": "enc2:not-hex",
                    "from_phone": "+10000000001"
                }
            }),
            "config.channels_config.linq.api_token",
        ),
        (
            serde_json::json!({
                "cli": true,
                "linq": {
                    "api_token": "plain-token",
                    "from_phone": "+10000000001",
                    "signing_secret": "enc2:not-hex"
                }
            }),
            "config.channels_config.linq.signing_secret",
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let store = SecretStore::new(tmp.path(), true);
        let mut channels: ChannelsConfig = serde_json::from_value(payload).unwrap();

        let err = decrypt_channel_secrets(&store, &mut channels).unwrap_err().to_string();

        assert!(err.contains(expected_path), "{expected_path}: {err}");
    }
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
fn channel_secret_processing_rejects_unsafe_instance_before_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true,
        "telegram": {"bot_token": "telegram-token", "allowed_users": []},
        "matrix_instances": [{
            "id": "unsafe/id",
            "homeserver": "https://matrix.example",
            "access_token": "matrix-token",
            "room_id": "!room:example",
            "allowed_users": []
        }]
    }))
    .unwrap();
    let mut encrypt_channels = channels.clone();
    let mut decrypt_channels = channels;

    let encrypt_err =
        encrypt_channel_secrets(&store, &mut encrypt_channels).unwrap_err().to_string();
    let decrypt_err =
        decrypt_channel_secrets(&store, &mut decrypt_channels).unwrap_err().to_string();

    let expected_error = "matrix channel instance id 'unsafe/id' must not contain ':', '/' or '\\'";
    assert!(encrypt_err.contains(expected_error));
    assert!(decrypt_err.contains(expected_error));
    assert_eq!(encrypt_channels.telegram.unwrap().bot_token, "telegram-token");
    assert_eq!(decrypt_channels.telegram.unwrap().bot_token, "telegram-token");
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
