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
