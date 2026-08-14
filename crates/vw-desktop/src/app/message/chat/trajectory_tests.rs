use crate::app::App;
use crate::app::components::chat_panel::trajectory::TrajectoryDetailsTab;
use crate::app::models::{ChatSessionStep, TokenUsage};
use crate::app::state::ChatPanelMode;

fn completed_step(index: u32) -> ChatSessionStep {
    ChatSessionStep {
        index,
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: Some("test-model".to_string()),
    }
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
        .insert(4, serde_json::json!({ "already": "loaded" }));

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );

    assert!(app.trajectory_raw_loading.contains(&("active-session".to_string(), 3)));
    assert!(!app.trajectory_raw_loading.contains(&("active-session".to_string(), 4)));
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
fn trajectory_raw_refresh_ignores_stale_session() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active-session".to_string());
    app.active_session_view_state.steps = vec![completed_step(7)];
    app.active_session_view_state.step_llm_raw.insert(7, serde_json::json!({ "current": true }));
    app.trajectory_raw_loading.insert(("stale-session".to_string(), 7));

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "stale-session".to_string(),
            step_index: 7,
            result: Ok(Some(serde_json::json!({ "stale": true }))),
        },
    );

    assert_eq!(
        app.active_session_view_state.step_llm_raw.get(&7),
        Some(&serde_json::json!({ "current": true }))
    );
    assert!(!app.trajectory_raw_loading.contains(&("stale-session".to_string(), 7)));

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "active-session".to_string(),
            step_index: 8,
            result: Ok(Some(serde_json::json!({ "wrong_step": true }))),
        },
    );
    assert!(!app.active_session_view_state.step_llm_raw.contains_key(&8));

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: "active-session".to_string(),
            step_index: 7,
            result: Err("temporary gateway failure".to_string()),
        },
    );
    assert_eq!(
        app.active_session_view_state.step_llm_raw.get(&7),
        Some(&serde_json::json!({ "current": true }))
    );
    assert_eq!(
        app.trajectory_raw_errors.get(&("active-session".to_string(), 7)).map(String::as_str),
        Some("temporary gateway failure")
    );
}

#[test]
fn trajectory_raw_refresh_error_is_recoverable() {
    let (mut app, _) = App::new();
    let key = ("active-session".to_string(), 7);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.steps = vec![completed_step(key.1)];

    let _task = super::update(
        &mut app,
        super::ChatMessage::TrajectoryRawLoaded {
            session_id: key.0.clone(),
            step_index: key.1,
            result: Err("temporary gateway failure".to_string()),
        },
    );
    assert!(app.trajectory_raw_errors.contains_key(&key));
    assert!(!app.active_session_view_state.step_llm_raw.contains_key(&key.1));

    let _task = super::update(
        &mut app,
        super::ChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory),
    );
    assert!(app.trajectory_raw_loading.contains(&key));
    assert!(!app.trajectory_raw_errors.contains_key(&key));
}
