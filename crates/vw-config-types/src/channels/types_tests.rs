#[test]
fn task_619_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("types_tests.rs"));
}

use super::{
    DiscordConfig, GroupReplyConfig, GroupReplyMode, IMessageConfig, MattermostConfig, SlackConfig,
    StreamMode, TelegramConfig, WebhookConfig, clone_group_reply_allowed_sender_ids,
    resolve_group_reply_mode,
};

#[test]
fn stream_mode_uses_lowercase_names_and_defaults_to_off() {
    let config: TelegramConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "telegram-token",
        "allowed_users": []
    }))
    .unwrap();

    assert_eq!(config.stream_mode, StreamMode::Off);
    assert_eq!(config.draft_update_interval_ms, 1000);
    assert!(!config.interrupt_on_new_message);
    assert!(!config.mention_only);
    assert!(config.group_reply.is_none());
    assert!(config.base_url.is_none());
    assert_eq!(serde_json::to_value(StreamMode::Partial).unwrap(), "partial");
}

#[test]
fn group_reply_mode_serializes_and_reports_mention_requirement() {
    assert!(GroupReplyMode::MentionOnly.requires_mention());
    assert!(!GroupReplyMode::AllMessages.requires_mention());
    assert_eq!(serde_json::to_value(GroupReplyMode::MentionOnly).unwrap(), "mention_only");
}

#[test]
fn resolve_group_reply_mode_prefers_group_config_then_legacy_then_default() {
    let group_reply =
        GroupReplyConfig { mode: Some(GroupReplyMode::AllMessages), allowed_sender_ids: vec![] };

    assert_eq!(
        resolve_group_reply_mode(Some(&group_reply), Some(true), GroupReplyMode::MentionOnly),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        resolve_group_reply_mode(None, Some(true), GroupReplyMode::AllMessages),
        GroupReplyMode::MentionOnly
    );
    assert_eq!(
        resolve_group_reply_mode(None, Some(false), GroupReplyMode::MentionOnly),
        GroupReplyMode::AllMessages
    );
    assert_eq!(
        resolve_group_reply_mode(None, None, GroupReplyMode::MentionOnly),
        GroupReplyMode::MentionOnly
    );
}

#[test]
fn clone_group_reply_allowed_sender_ids_clones_or_returns_empty() {
    let group_reply = GroupReplyConfig {
        mode: None,
        allowed_sender_ids: vec!["alice".to_string(), "bob".to_string()],
    };

    assert_eq!(
        clone_group_reply_allowed_sender_ids(Some(&group_reply)),
        vec!["alice".to_string(), "bob".to_string()]
    );
    assert!(clone_group_reply_allowed_sender_ids(None).is_empty());
}

#[test]
fn telegram_effective_group_reply_uses_group_config_before_legacy() {
    let config: TelegramConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "telegram-token",
        "allowed_users": ["alice"],
        "stream_mode": "partial",
        "draft_update_interval_ms": 250,
        "interrupt_on_new_message": true,
        "mention_only": true,
        "group_reply": {
            "mode": "all_messages",
            "allowed_sender_ids": ["sender-a"]
        },
        "base_url": "https://telegram.example"
    }))
    .unwrap();

    assert_eq!(config.stream_mode, StreamMode::Partial);
    assert_eq!(config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert_eq!(config.group_reply_allowed_sender_ids(), vec!["sender-a".to_string()]);
    assert_eq!(config.base_url.as_deref(), Some("https://telegram.example"));

    let legacy: TelegramConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "telegram-token",
        "allowed_users": [],
        "mention_only": true
    }))
    .unwrap();

    assert_eq!(legacy.effective_group_reply_mode(), GroupReplyMode::MentionOnly);
    assert!(legacy.group_reply_allowed_sender_ids().is_empty());
}

#[test]
fn discord_effective_group_reply_covers_defaults_legacy_and_allowed_senders() {
    let default_config: DiscordConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "discord-token",
        "guild_id": null
    }))
    .unwrap();

    assert!(default_config.allowed_users.is_empty());
    assert!(!default_config.listen_to_bots);
    assert!(!default_config.mention_only);
    assert_eq!(default_config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert!(default_config.group_reply_allowed_sender_ids().is_empty());

    let group_config: DiscordConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "discord-token",
        "guild_id": "guild",
        "allowed_users": ["alice"],
        "listen_to_bots": true,
        "mention_only": true,
        "group_reply": {
            "mode": "all_messages",
            "allowed_sender_ids": ["sender-a", "sender-b"]
        }
    }))
    .unwrap();

    assert_eq!(group_config.guild_id.as_deref(), Some("guild"));
    assert_eq!(group_config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert_eq!(
        group_config.group_reply_allowed_sender_ids(),
        vec!["sender-a".to_string(), "sender-b".to_string()]
    );
}

#[test]
fn slack_effective_group_reply_uses_default_or_group_config() {
    let default_config: SlackConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "slack-token",
        "app_token": null,
        "channel_id": null
    }))
    .unwrap();

    assert_eq!(default_config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert!(default_config.group_reply_allowed_sender_ids().is_empty());

    let group_config: SlackConfig = serde_json::from_value(serde_json::json!({
        "bot_token": "slack-token",
        "app_token": "app-token",
        "channel_id": "channel",
        "allowed_users": ["alice"],
        "group_reply": {
            "mode": "mention_only",
            "allowed_sender_ids": ["sender-a"]
        }
    }))
    .unwrap();

    assert_eq!(group_config.effective_group_reply_mode(), GroupReplyMode::MentionOnly);
    assert_eq!(group_config.group_reply_allowed_sender_ids(), vec!["sender-a".to_string()]);
}

#[test]
fn mattermost_effective_group_reply_handles_optional_legacy_and_group_config() {
    let default_config: MattermostConfig = serde_json::from_value(serde_json::json!({
        "url": "https://mattermost.example",
        "bot_token": "mattermost-token",
        "channel_id": null
    }))
    .unwrap();

    assert!(default_config.allowed_users.is_empty());
    assert!(default_config.thread_replies.is_none());
    assert!(default_config.mention_only.is_none());
    assert_eq!(default_config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert!(default_config.group_reply_allowed_sender_ids().is_empty());

    let legacy: MattermostConfig = serde_json::from_value(serde_json::json!({
        "url": "https://mattermost.example",
        "bot_token": "mattermost-token",
        "channel_id": "channel",
        "thread_replies": true,
        "mention_only": true
    }))
    .unwrap();

    assert_eq!(legacy.effective_group_reply_mode(), GroupReplyMode::MentionOnly);

    let group_config: MattermostConfig = serde_json::from_value(serde_json::json!({
        "url": "https://mattermost.example",
        "bot_token": "mattermost-token",
        "channel_id": "channel",
        "mention_only": true,
        "group_reply": {
            "mode": "all_messages",
            "allowed_sender_ids": ["sender-a"]
        }
    }))
    .unwrap();

    assert_eq!(group_config.effective_group_reply_mode(), GroupReplyMode::AllMessages);
    assert_eq!(group_config.group_reply_allowed_sender_ids(), vec!["sender-a".to_string()]);
}

#[test]
fn webhook_and_imessage_deserialize_simple_fields() {
    let webhook: WebhookConfig = serde_json::from_value(serde_json::json!({
        "port": 8080,
        "secret": "secret"
    }))
    .unwrap();
    let imessage: IMessageConfig = serde_json::from_value(serde_json::json!({
        "allowed_contacts": ["alice@example.com"]
    }))
    .unwrap();

    assert_eq!(webhook.port, 8080);
    assert_eq!(webhook.secret.as_deref(), Some("secret"));
    assert_eq!(imessage.allowed_contacts, vec!["alice@example.com".to_string()]);
}
