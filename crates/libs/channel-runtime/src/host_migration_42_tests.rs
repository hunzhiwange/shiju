use agent_config::schema::Config;
use crate::ConfiguredChannel;
use crate::manager::channels_config::append_nostr_channel_if_available;
use config_channels::NostrConfig;

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
