//! Task-board initialization contracts moved with their production state.
#[test]
fn legacy_task_board_switches_are_ignored_after_capability_cutover() {
    let cfg = serde_json::json!({
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
    });

    let settings = crate::state::State::new(&cfg, None, 0).task_board_settings;
    let encoded = serde_json::to_value(&settings).expect("settings should serialize");
    for key in
        ["auto_execute", "auto_promote_pool_tasks", "code_review_enabled", "task_split_enabled"]
    {
        assert!(encoded.get(key).is_none(), "legacy setting {key} should not be persisted");
    }

    let legacy: desktop_task_model::models::TaskBoardSettings =
        serde_json::from_value(cfg).expect("legacy task board settings should decode");
    let legacy_encoded = serde_json::to_value(&legacy).expect("legacy settings should serialize");
    for key in
        ["auto_execute", "auto_promote_pool_tasks", "code_review_enabled", "task_split_enabled"]
    {
        assert!(
            legacy_encoded.get(key).is_none(),
            "legacy decoded setting {key} should be ignored"
        );
    }
}

#[test]
fn task_board_initial_state_preserves_configured_agent_timers_and_draft_defaults() {
    let state = crate::state::State::new(
        &serde_json::json!({
            "task_board_worktree_pixel_office": true,
            "task_board_clear_prompt_after_create": false,
            "task_board_close_after_create": false,
            "task_board_close_after_edit": false
        }),
        Some("agent-a".into()),
        100,
    );
    assert_eq!(state.task_board_last_acp_agent.as_deref(), Some("agent-a"));
    assert_eq!(state.task_board_bulk_acp_agent.as_deref(), Some("agent-a"));
    assert_eq!(state.task_board_next_refresh_at_ms, 60_100);
    assert_eq!(state.task_board_next_scheduler_tick_at_ms, 1_100);
    assert_eq!(state.task_board_next_auto_review_tick_at_ms, 30_100);
    assert_eq!(state.task_board_next_auto_promote_tick_at_ms, 30_100);
    assert_eq!(state.task_board_next_auto_merge_tick_at_ms, 10_100);
    assert!(state.task_board_worktree_pixel_office);
    assert!(!state.task_board_clear_prompt_after_create);
    assert!(!state.task_board_close_after_create);
    assert!(!state.task_board_close_after_edit);
    assert!(state.task_board_tasks_pending_persistence.is_empty());
    assert!(state.task_board_selected_tasks.is_empty());
    assert_eq!(state.task_board_bulk_priority_input, "999");
    assert!(state.task_board_sort_by_priority);
    assert!(!state.task_board_sort_ascending);
    assert!(state.task_board_logs_auto_scroll);
}
