//! 验证自动化调度、心跳兼容字段与定时任务启用状态的配置契约。
//! 重点锁定省略字段时的默认行为，以及旧版字段名仍可读取的迁移边界。

// 调度器默认值决定后台任务的基础并发上限，需避免配置升级后意外漂移。
#[test]
fn automation_defaults_cover_scheduler() {
    let scheduler = super::SchedulerConfig::default();
    assert!(scheduler.enabled);
    assert_eq!(scheduler.max_tasks, 64);
    assert_eq!(scheduler.max_concurrent, 4);
}

// 旧版心跳配置仍由现有用户文件使用，别名必须映射到当前字段。
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

// `enabled` 缺失时保持启用，确保旧配置升级后定时任务不会静默停用。
#[test]
fn cron_deserialization_applies_enabled_default() {
    let cron: super::CronConfig = serde_json::from_value(serde_json::json!({
        "max_run_history": 12
    }))
    .unwrap();

    assert!(cron.enabled);
    assert_eq!(cron.max_run_history, 12);
}
