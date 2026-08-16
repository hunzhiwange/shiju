use crate::app::App;
use crate::app::components::chat_panel::trajectory::TrajectoryDetailsTab;
use crate::app::models::{ChatSessionStep, TokenUsage};
use crate::app::state::ChatPanelMode;

fn completed_step(index: u32) -> ChatSessionStep {
    completed_step_for_turn(index, &format!("turn-{index}"))
}

fn completed_step_for_turn(index: u32, turn_id: &str) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: Some(turn_id.to_string()),
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: Some("test-model".to_string()),
        tool_timings: Vec::new(),
    }
}

#[test]
fn trajectory_raw_cache_keys_include_turn_id() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("shared-session".to_string());
    app.active_session_view_state.steps = vec![
        completed_step_for_turn(1, "turn-a"),
        completed_step_for_turn(1, "turn-b"),
    ];
    let first_key = ("shared-session".to_string(), "turn-a".to_string(), 1);
    let second_key = ("shared-session".to_string(), "turn-b".to_string(), 1);
    for key in [&first_key, &second_key] {
        app.trajectory_raw_loading.insert(key.clone());
        app.trajectory_raw_attempts.insert(key.clone(), 1);
    }

    for (key, value) in [
        (&first_key, serde_json::json!({"output": {"text": "first"}})),
        (&second_key, serde_json::json!({"output": {"text": "second"}})),
    ] {
        let _task = super::update(
            &mut app,
            super::ChatMessage::TrajectoryRawLoaded {
                session_id: key.0.clone(),
                turn_id: key.1.clone(),
                step_index: key.2,
                attempt: 1,
                result: Ok(Some(value)),
            },
        );
    }

    assert_eq!(app.active_session_view_state.step_llm_raw.len(), 2);
    assert_eq!(
        app.active_session_view_state.step_llm_raw[&first_key]["output"]["text"],
        "first"
    );
    assert_eq!(
        app.active_session_view_state.step_llm_raw[&second_key]["output"]["text"],
        "second"
    );
}

#[test]
fn trajectory_raw_refresh_normalizes_blank_turn_id() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active-session".to_string());
    app.active_session_view_state.steps = vec![completed_step_for_turn(12, "  \t")];
    let legacy_key = (
        "active-session".to_string(),
        "legacy-step-12".to_string(),
        12,
    );

    let task = super::trajectory::refresh_missing_raw_task(&mut app, "active-session", None);

    assert!(task.units() > 0);
    assert!(app.trajectory_raw_loading.contains(&legacy_key));
    assert_eq!(app.trajectory_raw_attempts.get(&legacy_key), Some(&1));
}

mod trajectory_raw {
    use super::*;

    #[test]
    fn raw_modal_and_trajectory_share_reconstructed_payload() {
        let (mut app, _) = App::new();
        app.active_session_id = Some("shared-session".to_string());
        app.active_session_view_state.steps = vec![completed_step_for_turn(1, "turn-a")];
        let key = ("shared-session".to_string(), "turn-a".to_string(), 1);
        let reconstructed = serde_json::json!({
            "system": ["system prompt"],
            "messages": [{"role": "user", "content": "question"}],
            "request": {"config": {"model": "provider/model"}, "tools": []},
            "output": {"text": "answer", "tool_calls": []}
        });

        let _task = crate::app::message::view::panel::update(
            &mut app,
            crate::app::message::ViewMessage::UsageTurnStepRawLoaded(
                key.0.clone(),
                key.1.clone(),
                key.2,
                Ok(Some(reconstructed.clone())),
            ),
        );
        let projection = crate::app::components::chat_panel::trajectory::project_trajectory(
            &app.active_session_view_state.steps,
            &app.active_session_view_state.step_llm_raw,
        );
        let _task = crate::app::message::view::panel::update(
            &mut app,
            crate::app::message::ViewMessage::UsageTurnStepRawOpen(key.1.clone(), key.2),
        );

        assert!(!projection.records.is_empty());
        assert_eq!(app.active_session_view_state.step_llm_raw.get(&key), Some(&reconstructed));
        assert!(!app.usage_step_raw_payloads.contains_key(&key));
        let modal = app.usage_step_raw_modal.as_ref().expect("shared raw modal");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&modal.content).unwrap(), reconstructed);
    }
}

fn raw_key(session_id: &str, step_index: u32) -> (String, String, u32) {
    (session_id.to_string(), format!("turn-{step_index}"), step_index)
}

#[test]
fn trajectory_mode_toggle_defaults_to_conversation() {
    let (mut app, _) = App::new();

    assert_eq!(app.chat_panel_mode, ChatPanelMode::Conversation);

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );
    assert_eq!(app.chat_panel_mode, ChatPanelMode::Trajectory);

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Conversation),
    );
    assert_eq!(app.chat_panel_mode, ChatPanelMode::Conversation);
}

#[test]
fn trajectory_mode_toggle_refreshes_missing_completed_raw() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active-session".to_string());
    app.active_session_view_state.steps = vec![completed_step(3), completed_step(4)];
    app.active_session_view_state
        .step_llm_raw
        .insert(raw_key("active-session", 4), serde_json::json!({ "already": "loaded" }));

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );

    assert!(app.trajectory_raw_loading.contains(&raw_key("active-session", 3)));
    assert_eq!(app.trajectory_raw_attempts.get(&raw_key("active-session", 3)), Some(&1));
    assert!(!app.trajectory_raw_loading.contains(&raw_key("active-session", 4)));
}

#[test]
fn trajectory_mode_refreshes_known_raw_steps_without_session_step_metadata() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("raw-only-session".to_string());
    app.active_session_view_state
        .llm_raw_step_indices
        .extend([raw_key("raw-only-session", 3), raw_key("raw-only-session", 7)]);

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );

    assert!(app.trajectory_raw_loading.contains(&raw_key("raw-only-session", 3)));
    assert!(app.trajectory_raw_loading.contains(&raw_key("raw-only-session", 7)));
}

#[test]
fn trajectory_interactions_update_state() {
    let (mut app, _) = App::new();

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectorySearchChanged("工具 调用".to_string()),
    );
    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRecordSelected("tool:call-7".to_string()),
    );
    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryDetailsTabSelected(TrajectoryDetailsTab::Raw),
    );

    assert_eq!(app.trajectory_search_text, "工具 调用");
    assert_eq!(app.trajectory_details.selected_record_id(), Some("tool:call-7"));
    assert_eq!(app.trajectory_details.active_tab(), TrajectoryDetailsTab::Raw);

    let _task = super::update(&mut app, super::ChatMessage::TrajectoryDetailsClosed);
    assert_eq!(app.trajectory_details.selected_record_id(), None);
    assert_eq!(app.trajectory_details.active_tab(), TrajectoryDetailsTab::Summary);
}

#[test]
fn trajectory_details_json_interactions_update_state_and_copy() {
    let (mut app, _) = App::new();
    app.trajectory_details.select("user:1");

    let toggle = super::update(
        &mut app,
        super::ChatMessage::TrajectoryDetailsJsonToggled("/source".to_string()),
    );
    assert_eq!(toggle.units(), 0);
    assert!(app.trajectory_details.json_collapsed_paths().contains("/source"));

    let copy = super::update(
        &mut app,
        super::ChatMessage::TrajectoryDetailsJsonCopied(
            serde_json::json!({"kind": "user", "rpcId": "rpc-1"}),
        ),
    );
    assert_eq!(copy.units(), 1);
}

mod trajectory_controls {
    use super::*;
    use crate::app::components::chat_panel::trajectory::TrajectoryDurationMode;

    #[test]
    fn select_expands_record_ancestors() {
        let (mut app, _) = App::new();
        app.active_session_view_state.steps = vec![completed_step(1)];
        app.active_session_view_state.step_llm_raw.insert(
            raw_key("", 1),
            serde_json::json!({
                "messages": [{"role": "user", "content": "question"}],
                "output": {
                    "text": "working",
                    "tool_calls": [{
                        "id": "call-1",
                        "name": "read_file",
                        "arguments": {"path": "Cargo.toml"}
                    }]
                }
            }),
        );
        app.trajectory_controls.toggle_turn(1);
        app.trajectory_controls.toggle_tool_step(1);

        let _task = super::super::update(
            &mut app,
            super::super::ChatMessage::TrajectoryRecordSelected("tool:call-1".to_string()),
        );

        assert!(!app.trajectory_controls.is_turn_collapsed(1));
        assert!(!app.trajectory_controls.is_tool_step_collapsed(1));
        assert_eq!(app.trajectory_details.selected_record_id(), Some("tool:call-1"));
    }

    #[test]
    fn messages_update_controls_and_session_change_clears_collapsed_groups() {
        let (mut app, _) = App::new();
        app.active_session_id = Some("session-1".to_string());
        app.active_session_view_state.steps = vec![completed_step(1)];
        app.active_session_view_state.step_llm_raw.insert(
            raw_key("session-1", 1),
            serde_json::json!({
                "messages": [{"role": "user", "content": "question"}],
                "output": {
                    "text": "working",
                    "tool_calls": [{"id": "call-1", "name": "read_file", "arguments": {}}]
                }
            }),
        );

        let _task =
            super::super::update(&mut app, super::super::ChatMessage::TrajectoryDurationToggled);
        let _task =
            super::super::update(&mut app, super::super::ChatMessage::TrajectoryTurnsToggled);
        let _task =
            super::super::update(&mut app, super::super::ChatMessage::TrajectoryCallsToggled);

        assert_eq!(app.trajectory_controls.duration_mode(), TrajectoryDurationMode::Equal);
        assert!(app.trajectory_controls.is_turn_collapsed(1));
        assert!(app.trajectory_controls.is_tool_step_collapsed(1));

        app.active_session_id = Some("session-2".to_string());
        let switched = app.trajectory_controls.for_session(app.active_session_id.as_deref());
        assert_eq!(switched.duration_mode(), TrajectoryDurationMode::Equal);
        assert!(!switched.is_turn_collapsed(1));
        assert!(!switched.is_tool_step_collapsed(1));

        let _task = super::super::update(
            &mut app,
            super::super::ChatMessage::TrajectorySearchChanged(String::new()),
        );

        assert_eq!(app.trajectory_controls.duration_mode(), TrajectoryDurationMode::Equal);
        assert!(!app.trajectory_controls.is_turn_collapsed(1));
        assert!(!app.trajectory_controls.is_tool_step_collapsed(1));
    }
}

#[test]
fn trajectory_raw_refresh_ignores_stale_session() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active-session".to_string());
    app.active_session_view_state.steps = vec![completed_step(7)];
    let current_key = raw_key("active-session", 7);
    app.active_session_view_state
        .step_llm_raw
        .insert(current_key.clone(), serde_json::json!({ "current": true }));
    let stale_key = raw_key("stale-session", 7);
    app.trajectory_raw_loading.insert(stale_key.clone());
    app.trajectory_raw_attempts.insert(stale_key.clone(), 1);
    app.trajectory_raw_errors.insert(stale_key.clone(), "old error".to_string());

    let stale_retry = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawRetry {
            session_id: stale_key.0.clone(),
            turn_id: stale_key.1.clone(),
            step_index: stale_key.2,
            attempt: 1,
        },
    );
    assert_eq!(stale_retry.units(), 0);
    assert!(!app.trajectory_raw_loading.contains(&stale_key));
    assert!(!app.trajectory_raw_attempts.contains_key(&stale_key));
    assert!(!app.trajectory_raw_errors.contains_key(&stale_key));

    app.trajectory_raw_loading.insert(stale_key.clone());
    app.trajectory_raw_attempts.insert(stale_key.clone(), 1);
    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "stale-session".to_string(),
            turn_id: stale_key.1.clone(),
            step_index: 7,
            attempt: 1,
            result: Ok(Some(serde_json::json!({ "stale": true }))),
        },
    );

    assert_eq!(
        app.active_session_view_state.step_llm_raw.get(&current_key),
        Some(&serde_json::json!({ "current": true }))
    );
    assert!(!app.trajectory_raw_loading.contains(&stale_key));
    assert!(!app.trajectory_raw_attempts.contains_key(&stale_key));
    assert!(!app.trajectory_raw_errors.contains_key(&stale_key));

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "active-session".to_string(),
            turn_id: "turn-8".to_string(),
            step_index: 8,
            attempt: 1,
            result: Ok(Some(serde_json::json!({ "wrong_step": true }))),
        },
    );
    assert!(!app.active_session_view_state.step_llm_raw.contains_key(&raw_key("active-session", 8)));

    let active_key = current_key.clone();
    app.trajectory_raw_loading.insert(active_key.clone());
    app.trajectory_raw_attempts.insert(active_key.clone(), 1);
    app.trajectory_raw_errors.insert(active_key.clone(), "old error".to_string());
    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "active-session".to_string(),
            turn_id: active_key.1.clone(),
            step_index: 7,
            attempt: 1,
            result: Err("temporary gateway failure".to_string()),
        },
    );
    assert_eq!(
        app.active_session_view_state.step_llm_raw.get(&current_key),
        Some(&serde_json::json!({ "current": true }))
    );
    assert!(!app.trajectory_raw_loading.contains(&active_key));
    assert!(!app.trajectory_raw_attempts.contains_key(&active_key));
    assert!(!app.trajectory_raw_errors.contains_key(&active_key));
}

#[test]
fn trajectory_raw_refresh_error_is_recoverable() {
    let (mut app, _) = App::new();
    let key = raw_key("active-session", 7);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.steps = vec![completed_step(key.2)];
    app.trajectory_raw_loading.insert(key.clone());
    app.trajectory_raw_attempts.insert(key.clone(), 3);

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 3,
            result: Err("temporary gateway failure".to_string()),
        },
    );
    assert!(app.trajectory_raw_errors.contains_key(&key));
    assert!(!app.active_session_view_state.step_llm_raw.contains_key(&key));

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );
    assert!(app.trajectory_raw_loading.contains(&key));
    assert_eq!(app.trajectory_raw_attempts.get(&key), Some(&1));
    assert!(!app.trajectory_raw_errors.contains_key(&key));
}

#[test]
fn trajectory_raw_none_schedules_bounded_retry() {
    let (mut app, _) = App::new();
    let key = raw_key("active-session", 7);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.steps = vec![completed_step(key.2)];
    app.trajectory_raw_loading.insert(key.clone());
    app.trajectory_raw_attempts.insert(key.clone(), 1);

    let retry = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 1,
            result: Ok(None),
        },
    );

    assert!(retry.units() > 0, "空响应应调度下一次有限重试");
    assert!(app.trajectory_raw_loading.contains(&key));
    assert_eq!(app.trajectory_raw_attempts.get(&key), Some(&2));
    assert!(!app.trajectory_raw_errors.contains_key(&key));

    let third = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 2,
            result: Ok(None),
        },
    );
    assert!(third.units() > 0);
    assert_eq!(app.trajectory_raw_attempts.get(&key), Some(&3));

    let exhausted = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 3,
            result: Ok(None),
        },
    );
    assert_eq!(exhausted.units(), 0);
    assert!(!app.trajectory_raw_loading.contains(&key));
    assert!(!app.trajectory_raw_attempts.contains_key(&key));
    assert!(app.trajectory_raw_errors.get(&key).is_some_and(|error| error.contains("3 次")));
}

#[test]
fn trajectory_raw_retry_stops_after_success() {
    let (mut app, _) = App::new();
    let key = raw_key("active-session", 8);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.steps = vec![completed_step(key.2)];
    app.trajectory_raw_loading.insert(key.clone());
    app.trajectory_raw_attempts.insert(key.clone(), 2);

    let next = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 2,
            result: Ok(Some(serde_json::json!({ "loaded": true }))),
        },
    );

    assert_eq!(next.units(), 0);
    assert_eq!(
        app.active_session_view_state.step_llm_raw.get(&key),
        Some(&serde_json::json!({ "loaded": true }))
    );
    assert!(!app.trajectory_raw_loading.contains(&key));
    assert!(!app.trajectory_raw_attempts.contains_key(&key));
    assert!(!app.trajectory_raw_errors.contains_key(&key));

    let stale_retry = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawRetry {
            session_id: key.0.clone(),
            turn_id: key.1.clone(),
            step_index: key.2,
            attempt: 3,
        },
    );
    assert_eq!(stale_retry.units(), 0);
}

#[test]
fn trajectory_raw_retry_exhaustion_reports_error() {
    let (mut app, _) = App::new();
    let key = raw_key("active-session", 9);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.steps = vec![completed_step(key.2)];
    app.trajectory_raw_loading.insert(key.clone());
    app.trajectory_raw_attempts.insert(key.clone(), 1);

    for attempt in 1..=3 {
        let next = super::update(
            &mut app,
            super::ChatMessage::TrajectoryRawLoaded {
                session_id: key.0.clone(),
                turn_id: key.1.clone(),
                step_index: key.2,
                attempt,
                result: Err("temporary gateway failure".to_string()),
            },
        );
        if attempt < 3 {
            assert!(next.units() > 0);
            assert_eq!(app.trajectory_raw_attempts.get(&key), Some(&(attempt + 1)));
            assert!(!app.trajectory_raw_errors.contains_key(&key));
        } else {
            assert_eq!(next.units(), 0);
        }
    }

    assert!(!app.trajectory_raw_loading.contains(&key));
    assert!(!app.trajectory_raw_attempts.contains_key(&key));
    assert_eq!(
        app.trajectory_raw_errors.get(&key).map(String::as_str),
        Some("temporary gateway failure")
    );

    let fourth_attempt = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawRetry {
            session_id: key.0,
            turn_id: key.1,
            step_index: key.2,
            attempt: 4,
        },
    );
    assert_eq!(fourth_attempt.units(), 0);
}
