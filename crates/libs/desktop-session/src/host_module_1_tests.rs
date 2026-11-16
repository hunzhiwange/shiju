//! 活动会话真实领域状态的增量更新契约。
use crate::session::active::ActiveSessionViewState;
use session_ui_types::{ChatMessage, ChatRole, ChatSessionStep, TokenUsage};
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn message_with_model(role: ChatRole, content: &str, model: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: Some(model.to_string()),
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn step(index: u32, started_ms: u64, finished_ms: Option<u64>, model: &str) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms,
        finished_ms,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some(model.to_string()),
        tool_timings: Vec::new(),
    }
}

#[test]
fn refresh_tail_message_meta_appends_then_replaces_changed_label() {
    let mut state = ActiveSessionViewState::<()>::default();
    let chat = vec![message_with_model(ChatRole::User, "question", "message-model")];
    state.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"), |_, _| ());

    state.refresh_active_session_tail_message_meta(0, &chat);
    assert_eq!(state.message_meta_texts.len(), 1);
    assert!(
        state.message_meta_texts[0]
            .as_ref()
            .map(|meta| meta
                .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)))
            .is_some_and(|label| label.contains("message-model"))
    );

    state.upsert_active_session_step(step(1, 2_000, Some(2_500), "model-b"), |_, _| ());
    state.refresh_active_session_tail_message_meta(0, &chat);
    assert!(
        state.message_meta_texts[0]
            .as_ref()
            .map(|meta| meta
                .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)))
            .is_some_and(|label| label.contains("message-model"))
    );
}

#[test]
fn refresh_tail_message_meta_rebuilds_gaps_and_truncates_out_of_range_entries() {
    let mut state = ActiveSessionViewState::<()>::default();
    let chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    state.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"), |_, _| ());

    state.refresh_active_session_tail_message_meta(1, &chat);
    assert_eq!(state.message_meta_texts.len(), 2);

    state.message_meta_texts.push(Some(desktop_i18n::chat::MessageMeta::Model {
        model: "stale".to_string(),
        time_ms: None,
    }));
    state.refresh_active_session_tail_message_meta(1, &chat);
    assert_eq!(state.message_meta_texts.len(), 2);

    state.message_meta_texts.push(Some(desktop_i18n::chat::MessageMeta::Model {
        model: "stale".to_string(),
        time_ms: None,
    }));
    state.refresh_active_session_tail_message_meta(2, &chat);
    assert_eq!(state.message_meta_texts.len(), 2);
}

#[test]
fn question_index_updates_incrementally_for_appended_user_message() {
    let mut state = ActiveSessionViewState::<()>::default();
    let mut chat = vec![message(ChatRole::Assistant, "existing answer")];
    state.rebuild_active_session_message_meta(&chat);

    chat.push(message(ChatRole::User, "new question"));
    state.sync_active_session_user_question_index(&chat);

    assert_eq!(state.user_question_indexed_chat_len, 2);
    assert_eq!(state.user_question_markers.len(), 1);
    let marker = &state.user_question_markers[0];
    assert_eq!(marker.message_idx, 1);
    assert_eq!(marker.preview, "new question");
}

#[test]
fn active_session_step_mutations_keep_steps_calls_and_index_map_consistent() {
    let mut state = ActiveSessionViewState::<()>::default();
    let calls = vec![serde_json::json!({"name": "read"})];

    state.replace_active_session_steps(vec![step(2, 2_000, None, "model-b")], |_, _| ());
    state.replace_active_session_calls(calls.clone());
    state.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"), |_, _| ());
    state.upsert_active_session_step(step(2, 2_000, Some(2_500), "model-c"), |_, _| ());

    assert_eq!(state.steps.iter().map(|step| step.index).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(state.steps[1].model.as_deref(), Some("model-c"));
    assert_eq!(
        state.step_index_map.get(&2).and_then(|meta| meta.model.as_deref()),
        Some("model-c")
    );
    assert_eq!(state.calls, calls);

    state.clear_active_session_steps(|_, _| ());
    assert!(state.steps.is_empty());
    assert!(state.calls.is_empty());
    assert!(state.step_index_map.is_empty());
}

#[test]
fn active_session_step_mutations_normalize_blank_turn_ids() {
    let mut state = ActiveSessionViewState::<()>::default();
    let mut loaded = step(1, 1_000, Some(1_500), "model-a");
    loaded.turn_id = Some("  \t".to_string());
    state.replace_active_session_steps(vec![loaded], |_, _| ());
    assert_eq!(state.steps[0].turn_id, None);

    let mut streamed = step(2, 2_000, Some(2_500), "model-b");
    streamed.turn_id = Some(String::new());
    state.upsert_active_session_step(streamed, |_, _| ());
    assert_eq!(state.steps[1].turn_id, None);
}

#[test]
fn active_session_clear_removes_step_raw_payloads() {
    let mut state = ActiveSessionViewState::<()>::default();
    let raw = serde_json::json!({
        "messages": [{"role": "user", "content": "retained raw payload"}]
    });
    state.replace_active_session_step_llm_raw(
        std::collections::HashMap::from([(
            ("session-1".to_string(), "turn-1".to_string(), 1),
            raw,
        )]),
        |_, _| (),
    );
    let estimated_with_raw = state.estimated_bytes();

    assert!(estimated_with_raw > 0);
    state.clear_active_session_steps(|_, _| ());

    assert!(state.step_llm_raw.is_empty());
    assert!(state.estimated_bytes() < estimated_with_raw);
}
