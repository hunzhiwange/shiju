#[test]
fn legacy_task_board_settings_ignore_removed_switches() {
    let legacy: desktop_task_model::models::TaskBoardSettings =
        serde_json::from_value(serde_json::json!({
            "task_board_auto_promote_pool_tasks": true,
            "task_board_code_review_enabled": true,
            "task_board_task_split_enabled": true,
            "auto_execute": true,
            "auto_promote_pool_tasks": true,
            "code_review_enabled": true,
            "task_split_enabled": true,
            "max_concurrent": 3,
            "default_priority": 999,
            "auto_promote_delay_seconds": 30
        }))
        .expect("legacy task board settings should decode");
    let encoded = serde_json::to_value(&legacy).expect("legacy settings should serialize");
    for key in
        ["auto_execute", "auto_promote_pool_tasks", "code_review_enabled", "task_split_enabled"]
    {
        assert!(encoded.get(key).is_none(), "legacy decoded setting {key} should be ignored");
    }
}
