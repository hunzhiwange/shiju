//! 验证可靠性配置的重试与退避默认值及其稳定序列化字段集合。
//! 这些值直接影响故障恢复节奏，新增内部字段不得意外进入持久化格式。

// 锁定提供方、通道与调度器三类恢复策略的默认预算。
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

// 序列化键集合是外部配置契约，只允许输出当前生效的公开字段。
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
