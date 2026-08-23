//! 用量面板消息处理测试，覆盖模型限额信息和步骤原始用量的加载状态。
//! 重点验证展开、缓存与会话切换不会触发重复请求或遗留旧弹窗。

use crate::app::models::{ChatSessionStep, TokenUsage};
use crate::app::{App, message::ViewMessage, state::UsageModelInfo};

fn test_app() -> App {
    App::new().0
}

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
    let mut app = test_app();
    app.usage_model_info = Some(model_info("old-model"));

    let _task =
        super::update(&mut app, ViewMessage::UsageModelInfoLoaded(Some(model_info("new-model"))));

    let info = app.usage_model_info.as_ref().expect("model info should be stored");
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
    let mut app = test_app();
    app.usage_model_info = Some(model_info("old-model"));

    let _task = super::update(&mut app, ViewMessage::UsageModelInfoLoaded(None));

    assert!(app.usage_model_info.is_none());
}

#[test]
fn usage_step_toggled_collapses_expanded_step_without_loading() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_expanded.insert(7);

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(7));

    assert!(!app.usage_step_expanded.contains(&7));
    assert!(app.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_collapses_matching_raw_modal() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_expanded.insert(7);
    app.usage_step_raw_modal =
        Some(crate::app::state::UsageStepRawModal { step_index: 7, content: "{}".to_string() });

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(7));

    assert!(app.usage_step_raw_modal.is_none());
}

#[test]
fn usage_step_toggled_without_active_session_only_expands_step() {
    let mut app = test_app();
    app.active_session_id = None;

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(3));

    assert!(app.usage_step_expanded.contains(&3));
    assert!(app.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_with_cached_payload_does_not_reload() {
    let mut app = test_app();
    let key = raw_key("active-session", 5);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_payloads
        .insert(key.clone(), Ok(Some(serde_json::json!({ "cached": true }))));

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.2));

    assert!(app.usage_step_expanded.contains(&key.2));
    assert!(!app.usage_step_raw_loading.contains(&key));
    assert_eq!(
        app.usage_step_raw_payloads.get(&key),
        Some(&Ok(Some(serde_json::json!({ "cached": true }))))
    );
}

#[test]
fn usage_step_toggled_while_loading_does_not_duplicate_load() {
    let mut app = test_app();
    let key = raw_key("active-session", 9);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.2));

    assert!(app.usage_step_expanded.contains(&key.2));
    assert_eq!(app.usage_step_raw_loading.len(), 1);
    assert!(app.usage_step_raw_loading.contains(&key));
}

#[test]
fn usage_step_toggled_uncached_step_does_not_load_raw_payload() {
    let mut app = test_app();
    let key = raw_key("active-session", 11);
    app.active_session_id = Some(key.0.clone());

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.2));

    assert!(app.usage_step_expanded.contains(&key.2));
    assert!(!app.usage_step_raw_loading.contains(&key));
    assert!(!app.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn usage_step_raw_open_uncached_step_starts_loading() {
    let mut app = test_app();
    let key = raw_key("active-session", 11);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_expanded.insert(key.2);

    let _task = super::update(&mut app, ViewMessage::UsageStepRawOpen(key.2));

    assert!(app.usage_step_raw_loading.contains(&key));
    assert!(app.usage_step_raw_modal.is_none());
}

#[test]
fn usage_step_blank_turn_id_uses_legacy_cache_key() {
    let mut app = test_app();
    app.active_session_view_state.steps.push(ChatSessionStep {
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

    assert_eq!(super::turn_id_for_step(&app, 12), "legacy-step-12");
}

#[test]
fn usage_step_raw_loaded_for_active_session_caches_success_and_error() {
    let cases = [
        Ok(Some(serde_json::json!({ "request": "cached" }))),
        Err("payload unavailable".to_string()),
    ];

    for (step_index, result) in cases.into_iter().enumerate() {
        let mut app = test_app();
        let key = raw_key("active-session", step_index as u32);
        app.active_session_id = Some(key.0.clone());
        app.usage_step_raw_loading.insert(key.clone());

        let _task = super::update(
            &mut app,
            ViewMessage::UsageStepRawLoaded(key.0.clone(), key.2, result.clone()),
        );

        assert!(!app.usage_step_raw_loading.contains(&key));
        if let Ok(Some(raw)) = result {
            assert_eq!(app.active_session_view_state.step_llm_raw.get(&key), Some(&raw));
            assert!(app.active_session_view_state.llm_raw_step_indices.contains(&key));
            assert!(!app.usage_step_raw_payloads.contains_key(&key));
        } else {
            assert_eq!(app.usage_step_raw_payloads.get(&key), Some(&result));
        }
    }
}

#[test]
fn usage_step_raw_loaded_caches_explicit_empty_payload() {
    let mut app = test_app();
    let key = raw_key("active-session", 21);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_loading.insert(key.clone());

    let _task =
        super::update(&mut app, ViewMessage::UsageStepRawLoaded(key.0.clone(), key.2, Ok(None)));

    assert!(!app.usage_step_raw_loading.contains(&key));
    assert_eq!(app.usage_step_raw_payloads.get(&key), Some(&Ok(None)));
}

#[test]
fn usage_step_raw_loaded_after_click_opens_modal() {
    let mut app = test_app();
    let key = raw_key("active-session", 22);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_expanded.insert(key.2);
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(
        &mut app,
        ViewMessage::UsageStepRawLoaded(
            key.0.clone(),
            key.2,
            Ok(Some(serde_json::json!({ "messages": [] }))),
        ),
    );

    let modal = app.usage_step_raw_modal.as_ref().expect("modal should open after loading");
    assert_eq!(modal.step_index, key.2);
    assert!(modal.content.contains("\"messages\""));
}

#[test]
fn usage_step_raw_loaded_for_stale_session_discards_result_and_clears_loading() {
    let mut app = test_app();
    let key = raw_key("stale-session", 13);
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(
        &mut app,
        ViewMessage::UsageStepRawLoaded(
            key.0.clone(),
            key.2,
            Ok(Some(serde_json::json!({ "stale": true }))),
        ),
    );

    assert!(!app.usage_step_raw_loading.contains(&key));
    assert!(!app.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn usage_step_raw_open_builds_modal_from_cached_payload() {
    let mut app = test_app();
    let key = raw_key("active-session", 4);
    app.active_session_id = Some(key.0.clone());
    app.active_session_view_state.step_llm_raw.insert(
        key.clone(),
        serde_json::json!({
            "messages": [{"role": "user"}]
        }),
    );

    let _task = super::update(&mut app, ViewMessage::UsageStepRawOpen(key.2));

    let modal = app.usage_step_raw_modal.as_ref().expect("modal should open");
    assert_eq!(modal.step_index, 4);
    assert!(modal.content.contains("\"role\": \"user\""));
}

#[test]
fn usage_step_raw_open_ignores_missing_or_empty_payload() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_raw_payloads.insert(raw_key("active-session", 8), Ok(None));

    let _task = super::update(&mut app, ViewMessage::UsageStepRawOpen(8));
    assert!(app.usage_step_raw_modal.is_none());

    let _task = super::update(&mut app, ViewMessage::UsageStepRawOpen(9));
    assert!(app.usage_step_raw_modal.is_none());
    assert!(app.usage_step_raw_loading.contains(&raw_key("active-session", 9)));
}

#[test]
fn usage_step_raw_close_clears_modal() {
    let mut app = test_app();
    app.usage_step_raw_modal =
        Some(crate::app::state::UsageStepRawModal { step_index: 1, content: "{}".to_string() });

    let _task = super::update(&mut app, ViewMessage::UsageStepRawClose);
    assert!(app.usage_step_raw_modal.is_none());
}

#[test]
fn unrelated_message_preserves_usage_panel_state() {
    let mut app = test_app();
    let key = raw_key("active-session", 17);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_expanded.insert(key.2);
    app.usage_step_raw_loading.insert(key.clone());
    app.usage_step_raw_modal =
        Some(crate::app::state::UsageStepRawModal { step_index: key.2, content: "{}".to_string() });

    let _task = super::update(&mut app, ViewMessage::OpenUsage);

    assert!(app.usage_step_expanded.contains(&key.2));
    assert!(app.usage_step_raw_loading.contains(&key));
    assert!(app.usage_step_raw_modal.is_some());
}
