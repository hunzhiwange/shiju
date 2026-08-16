#[test]
fn reliability_defaults_match_documented_values() {
    let config = super::ReliabilityConfig::default();
    assert_eq!(config.provider_retries, 3);
    assert_eq!(config.provider_backoff_ms, 500);
    assert_eq!(config.channel_initial_backoff_secs, 1);
    assert_eq!(config.channel_max_backoff_secs, 60);
    assert_eq!(config.scheduler_poll_secs, 60);
    assert_eq!(config.scheduler_retries, 3);
}

#[test]
fn reliability_serializes_only_effective_fields() {
    let serialized = serde_json::to_value(super::ReliabilityConfig::default()).unwrap();
    let keys = serialized.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>();

    assert_eq!(
        keys,
        vec![
            "channel_initial_backoff_secs",
            "channel_max_backoff_secs",
            "provider_backoff_ms",
            "provider_retries",
            "scheduler_poll_secs",
            "scheduler_retries",
        ]
    );
}
