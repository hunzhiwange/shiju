use super::*;

#[test]
fn wecom_config_requires_bot_id_and_secret() {
    let valid = WeComConfig { bot_id: "bot".into(), secret: "secret".into() };
    assert!(valid.has_required_credentials());
    assert!(
        !WeComConfig { bot_id: " ".into(), secret: "secret".into() }.has_required_credentials()
    );
    assert!(!WeComConfig { bot_id: "bot".into(), secret: "\t".into() }.has_required_credentials());
}

#[test]
fn wecom_config_round_trips_json() {
    let config: WeComConfig =
        serde_json::from_str(r#"{"bot_id":"bot","secret":"secret"}"#).unwrap();
    assert_eq!(config.bot_id, "bot");
    assert_eq!(config.secret, "secret");
}

#[test]
fn wecom_delivery_webhook_defaults_and_round_trips_without_receiving_profile() {
    use crate::channels::ChannelsConfig;
    let defaults = serde_json::to_value(ChannelsConfig::default()).unwrap();
    assert!(defaults["wecom_webhook"].is_null());
    let channels: ChannelsConfig = serde_json::from_value(serde_json::json!({
        "cli": true, "wecom_webhook": {"webhook_url": "https://example.invalid/hook?key=fixture"}
    }))
    .unwrap();
    let encoded = serde_json::to_value(&channels).unwrap();
    assert_eq!(encoded["wecom_webhook"]["webhook_url"], "https://example.invalid/hook?key=fixture");
    assert!(channels.resolve_channel_instance_profiles().unwrap().is_empty());
    assert!(!format!("{channels:?}").contains("key=fixture"));
}

#[test]
fn wecom_delivery_bot_debug_redacts_secret() {
    let config = WeComConfig { bot_id: "bot".into(), secret: "private-delivery-secret".into() };
    assert!(!format!("{config:?}").contains("private-delivery-secret"));
}
