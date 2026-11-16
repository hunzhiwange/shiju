use super::*;
use super::manager::append_nostr_channel_if_available;
use crate::app::agent::config::schema::NostrConfig;

fn channel_names(channels: &[ConfiguredChannel]) -> Vec<&str> {
    channels.iter().map(|channel| channel.display_name.as_ref()).collect()
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

#[tokio::test]
async fn channel_project_binding_qq_webhook_uses_default_instance() {
    let channel = super::qq::QQChannel::new("app".into(), "secret".into(), vec!["*".into()]);
    let messages = channel.parse_webhook_payload(&serde_json::json!({"op": 0, "t": "C2C_MESSAGE_CREATE", "d": {"id": "msg", "content": "hello", "author": {"user_openid": "user"}}})).await;
    let message = &messages[0];
    assert_eq!(message.instance_id, "default");
    assert_eq!(message.channel, "qq");
}
