#[test]
fn automation_defaults_cover_scheduler() {
    let scheduler = super::SchedulerConfig::default();
    assert!(scheduler.enabled);
    assert_eq!(scheduler.max_tasks, 64);
    assert_eq!(scheduler.max_concurrent, 4);
}

#[test]
fn heartbeat_deserializes_legacy_aliases() {
    let heartbeat: super::HeartbeatConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "interval_minutes": 5,
        "channel": "telegram",
        "recipient": "chat-1"
    }))
    .unwrap();

    assert!(heartbeat.enabled);
    assert_eq!(heartbeat.target.as_deref(), Some("telegram"));
    assert_eq!(heartbeat.to.as_deref(), Some("chat-1"));
}

#[test]
fn cron_deserialization_applies_enabled_default() {
    let cron: super::CronConfig = serde_json::from_value(serde_json::json!({
        "max_run_history": 12
    }))
    .unwrap();

    assert!(cron.enabled);
    assert_eq!(cron.max_run_history, 12);
}
