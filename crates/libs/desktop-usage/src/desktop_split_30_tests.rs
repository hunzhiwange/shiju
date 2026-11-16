use crate::panel::{Message, State, turn_id_for_step, update};
use crate::view_types::UsageModelInfo;
use session_ui_types::{ChatSessionStep, TokenUsage};

fn raw_key(session_id: &str, step_index: u32) -> (String, String, u32) {
    (session_id.to_string(), format!("legacy-step-{step_index}"), step_index)
}

fn model_info(model_id: &str) -> UsageModelInfo {
    UsageModelInfo {
        provider_id: "provider-id".to_string(),
        provider_name: "Provider".to_string(),
        model_id: model_id.to_string(),
        model_name: "Model".to_string(),
        context_limit: 128_000,
        input_limit: 112_000,
        output_limit: 16_000,
        limit_is_default: false,
        cost_input_per_million: 1.25,
        cost_output_per_million: 5.0,
        cost_cache_read_per_million: 0.125,
        cost_cache_write_per_million: 1.5,
    }
}

#[test]
fn usage_model_info_loaded_replaces_existing_info() {
    let mut state = State::default();
    let session: Option<String> = None;
    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    state.usage_model_info = Some(model_info("old-model"));

    let _task = update(
        &mut state,
        session.as_deref(),
        &steps,
        &raw,
        Message::UsageModelInfoLoaded(Some(model_info("new-model"))),
    );

    let info = state.usage_model_info.as_ref().expect("model info should be stored");
    assert_eq!(info.provider_id, "provider-id");
    assert_eq!(info.provider_name, "Provider");
    assert_eq!(info.model_id, "new-model");
    assert_eq!(info.model_name, "Model");
    assert_eq!(info.context_limit, 128_000);
    assert_eq!(info.output_limit, 16_000);
    assert_eq!(info.cost_input_per_million, 1.25);
    assert_eq!(info.cost_output_per_million, 5.0);
    assert_eq!(info.cost_cache_read_per_million, 0.125);
    assert_eq!(info.cost_cache_write_per_million, 1.5);
}

#[test]
fn usage_model_info_loaded_none_clears_existing_info() {
    let mut state = State::default();
    let session: Option<String> = None;
    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    state.usage_model_info = Some(model_info("old-model"));

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageModelInfoLoaded(None));

    assert!(state.usage_model_info.is_none());
}

#[test]
fn usage_step_toggled_collapses_expanded_step_without_loading() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let session: Option<String> = Some("active-session".to_string());
    state.usage_step_expanded.insert(7);

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(7));

    assert!(!state.usage_step_expanded.contains(&7));
    assert!(state.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_collapses_matching_raw_modal() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let session: Option<String> = Some("active-session".to_string());
    state.usage_step_expanded.insert(7);
    state.usage_step_raw_modal =
        Some(crate::view_types::UsageStepRawModal { step_index: 7, content: "{}".to_string() });

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(7));

    assert!(state.usage_step_raw_modal.is_none());
}

#[test]
fn usage_step_toggled_without_active_session_only_expands_step() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let session: Option<String> = None;

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(3));

    assert!(state.usage_step_expanded.contains(&3));
    assert!(state.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_with_cached_payload_does_not_reload() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 5);
    let session: Option<String> = Some(key.0.clone());
    state
        .usage_step_raw_payloads
        .insert(key.clone(), Ok(Some(serde_json::json!({ "cached": true }))));

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(key.2));

    assert!(state.usage_step_expanded.contains(&key.2));
    assert!(!state.usage_step_raw_loading.contains(&key));
    assert_eq!(
        state.usage_step_raw_payloads.get(&key),
        Some(&Ok(Some(serde_json::json!({ "cached": true }))))
    );
}

#[test]
fn usage_step_toggled_while_loading_does_not_duplicate_load() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 9);
    let session: Option<String> = Some(key.0.clone());
    state.usage_step_raw_loading.insert(key.clone());

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(key.2));

    assert!(state.usage_step_expanded.contains(&key.2));
    assert_eq!(state.usage_step_raw_loading.len(), 1);
    assert!(state.usage_step_raw_loading.contains(&key));
}

#[test]
fn usage_step_toggled_uncached_step_does_not_load_raw_payload() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 11);
    let session: Option<String> = Some(key.0.clone());

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepToggled(key.2));

    assert!(state.usage_step_expanded.contains(&key.2));
    assert!(!state.usage_step_raw_loading.contains(&key));
    assert!(!state.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn usage_step_raw_open_uncached_step_starts_loading() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 11);
    let session: Option<String> = Some(key.0.clone());
    state.usage_step_expanded.insert(key.2);

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepRawOpen(key.2));

    assert!(state.usage_step_raw_loading.contains(&key));
    assert!(state.usage_step_raw_modal.is_none());
}

#[test]
fn usage_step_blank_turn_id_uses_legacy_cache_key() {
    let mut steps = Vec::new();

    steps.push(ChatSessionStep {
        index: 12,
        turn_id: Some("  \t".to_string()),
        started_ms: 1,
        finished_ms: Some(2),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: None,
        tool_timings: Vec::new(),
    });

    assert_eq!(turn_id_for_step(&steps, 12), "legacy-step-12");
}

#[test]
fn usage_step_raw_loaded_caches_explicit_empty_payload() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 21);
    let session: Option<String> = Some(key.0.clone());
    state.usage_step_raw_loading.insert(key.clone());

    let _task = update(
        &mut state,
        session.as_deref(),
        &steps,
        &raw,
        Message::UsageStepRawLoaded(key.0.clone(), key.2, Ok(None)),
    );

    assert!(!state.usage_step_raw_loading.contains(&key));
    assert_eq!(state.usage_step_raw_payloads.get(&key), Some(&Ok(None)));
}

#[test]
fn usage_step_raw_loaded_after_click_opens_modal() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 22);
    let session: Option<String> = Some(key.0.clone());
    state.usage_step_expanded.insert(key.2);
    state.usage_step_raw_loading.insert(key.clone());

    let _task = update(
        &mut state,
        session.as_deref(),
        &steps,
        &raw,
        Message::UsageStepRawLoaded(
            key.0.clone(),
            key.2,
            Ok(Some(serde_json::json!({ "messages": [] }))),
        ),
    );

    let modal = state.usage_step_raw_modal.as_ref().expect("modal should open after loading");
    assert_eq!(modal.step_index, key.2);
    assert!(modal.content.contains("\"messages\""));
}

#[test]
fn usage_step_raw_loaded_for_stale_session_discards_result_and_clears_loading() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let key = raw_key("stale-session", 13);
    let session: Option<String> = Some("active-session".to_string());
    state.usage_step_raw_loading.insert(key.clone());

    let _task = update(
        &mut state,
        session.as_deref(),
        &steps,
        &raw,
        Message::UsageStepRawLoaded(
            key.0.clone(),
            key.2,
            Ok(Some(serde_json::json!({ "stale": true }))),
        ),
    );

    assert!(!state.usage_step_raw_loading.contains(&key));
    assert!(!state.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn usage_step_raw_open_builds_modal_from_cached_payload() {
    let mut state = State::default();

    let steps = Vec::new();
    let mut raw = std::collections::HashMap::new();
    let key = raw_key("active-session", 4);
    let session: Option<String> = Some(key.0.clone());
    raw.insert(
        key.clone(),
        serde_json::json!({
            "messages": [{"role": "user"}]
        }),
    );

    let _task =
        update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepRawOpen(key.2));

    let modal = state.usage_step_raw_modal.as_ref().expect("modal should open");
    assert_eq!(modal.step_index, 4);
    assert!(modal.content.contains("\"role\": \"user\""));
}

#[test]
fn usage_step_raw_open_ignores_missing_or_empty_payload() {
    let mut state = State::default();

    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    let session: Option<String> = Some("active-session".to_string());
    state.usage_step_raw_payloads.insert(raw_key("active-session", 8), Ok(None));

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepRawOpen(8));
    assert!(state.usage_step_raw_modal.is_none());

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepRawOpen(9));
    assert!(state.usage_step_raw_modal.is_none());
    assert!(state.usage_step_raw_loading.contains(&raw_key("active-session", 9)));
}

#[test]
fn usage_step_raw_close_clears_modal() {
    let mut state = State::default();
    let session: Option<String> = None;
    let steps = Vec::new();
    let raw = std::collections::HashMap::new();
    state.usage_step_raw_modal =
        Some(crate::view_types::UsageStepRawModal { step_index: 1, content: "{}".to_string() });

    let _task = update(&mut state, session.as_deref(), &steps, &raw, Message::UsageStepRawClose);
    assert!(state.usage_step_raw_modal.is_none());
}
