use super::*;
use crate::app::models::TokenUsage;
use std::sync::Arc;
use vw_shared::session::info::{Info, TimeInfo};

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
        started_ms,
        finished_ms,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some(model.to_string()),
    }
}

fn session_info(id: &str, directory: &str, updated_ms: u64) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: id.to_string(),
        version: "0.0.0".to_string(),
        time: TimeInfo { created: 1, updated: updated_ms, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

#[test]
fn message_meta_labels_use_step_start_for_user_and_finish_for_assistant() {
    let chat = vec![
        message_with_model(ChatRole::User, "question", "message-model"),
        message(ChatRole::System, "context"),
        message_with_model(ChatRole::Assistant, "answer", "message-model"),
        message(ChatRole::Tool, "result"),
    ];
    let step = step(1, 1_000, Some(1_500), "model-a");
    let step_index_map = HashMap::from([(1, StepUiMeta::from(&step))]);

    let labels = build_message_meta_labels(&chat, &step_index_map);

    let started = crate::app::components::chat_panel::utils::format_chat_time_label(1_000);
    let finished = crate::app::components::chat_panel::utils::format_chat_time_label(1_500);
    assert_eq!(
        labels,
        vec![
            Some(format!("message-model · {started}")),
            None,
            Some(format!("message-model · {finished}")),
            None,
        ]
    );
}

#[test]
fn message_meta_labels_omit_missing_models_but_keep_assistant_step_alignment() {
    let chat = vec![
        message(ChatRole::Assistant, "first"),
        message_with_model(ChatRole::User, "second question", "message-model-b"),
        message_with_model(ChatRole::Assistant, "second answer", "message-model-b"),
    ];
    let mut first = step(1, 1_000, Some(1_100), "unused");
    first.model = None;
    let second = step(2, 2_000, Some(2_100), "model-b");
    let step_index_map =
        HashMap::from([(1, StepUiMeta::from(&first)), (2, StepUiMeta::from(&second))]);

    let labels = build_message_meta_labels(&chat, &step_index_map);

    assert_eq!(labels[0], None);
    assert!(labels[1].as_deref().is_some_and(|label| label.contains("message-model-b")));
    assert!(labels[2].as_deref().is_some_and(|label| label.contains("message-model-b")));
}

#[test]
fn refresh_tail_message_meta_appends_then_replaces_changed_label() {
    let (mut app, _task) = App::new();
    app.chat = vec![message_with_model(ChatRole::User, "question", "message-model")];
    app.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"));

    app.refresh_active_session_tail_message_meta(0);
    assert_eq!(app.active_session_view_state.message_meta_texts.len(), 1);
    assert!(
        app.active_session_view_state.message_meta_texts[0]
            .as_deref()
            .is_some_and(|label| label.contains("message-model"))
    );

    app.upsert_active_session_step(step(1, 2_000, Some(2_500), "model-b"));
    app.refresh_active_session_tail_message_meta(0);
    assert!(
        app.active_session_view_state.message_meta_texts[0]
            .as_deref()
            .is_some_and(|label| label.contains("message-model"))
    );
}

#[test]
fn refresh_tail_message_meta_rebuilds_gaps_and_truncates_out_of_range_entries() {
    let (mut app, _task) = App::new();
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    app.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"));

    app.refresh_active_session_tail_message_meta(1);
    assert_eq!(app.active_session_view_state.message_meta_texts.len(), 2);

    app.active_session_view_state.message_meta_texts.push(Some("stale".to_string()));
    app.refresh_active_session_tail_message_meta(1);
    assert_eq!(app.active_session_view_state.message_meta_texts.len(), 2);

    app.active_session_view_state.message_meta_texts.push(Some("stale".to_string()));
    app.refresh_active_session_tail_message_meta(2);
    assert_eq!(app.active_session_view_state.message_meta_texts.len(), 2);
}

#[test]
fn question_index_updates_incrementally_for_appended_user_message() {
    let (mut app, _task) = App::new();
    app.chat = vec![message(ChatRole::Assistant, "existing answer")];
    app.rebuild_active_session_message_meta();

    app.chat.push(message(ChatRole::User, "new question"));
    app.sync_active_session_user_question_index();

    assert_eq!(app.active_session_view_state.user_question_indexed_chat_len, 2);
    assert_eq!(app.active_session_view_state.user_question_markers.len(), 1);
    let marker = &app.active_session_view_state.user_question_markers[0];
    assert_eq!(marker.message_idx, 1);
    assert_eq!(marker.preview, "new question");
}

#[test]
fn active_session_step_mutations_keep_steps_calls_and_index_map_consistent() {
    let (mut app, _task) = App::new();
    let calls = vec![serde_json::json!({"name": "read"})];

    app.replace_active_session_steps(vec![step(2, 2_000, None, "model-b")]);
    app.replace_active_session_calls(calls.clone());
    app.upsert_active_session_step(step(1, 1_000, Some(1_500), "model-a"));
    app.upsert_active_session_step(step(2, 2_000, Some(2_500), "model-c"));

    assert_eq!(
        app.active_session_view_state.steps.iter().map(|step| step.index).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(app.active_session_view_state.steps[1].model.as_deref(), Some("model-c"));
    assert_eq!(
        app.active_session_view_state.step_index_map.get(&2).and_then(|meta| meta.model.as_deref()),
        Some("model-c")
    );
    assert_eq!(app.active_session_view_state.calls, calls);

    app.clear_active_session_steps();
    assert!(app.active_session_view_state.steps.is_empty());
    assert!(app.active_session_view_state.calls.is_empty());
    assert!(app.active_session_view_state.step_index_map.is_empty());
}

#[test]
fn active_session_clear_removes_step_raw_payloads() {
    let (mut app, _task) = App::new();
    let raw = serde_json::json!({
        "messages": [{"role": "user", "content": "retained raw payload"}]
    });
    app.replace_active_session_step_llm_raw(std::collections::HashMap::from([(1, raw)]));
    let estimated_with_raw = app.active_session_view_state.estimated_bytes();

    assert!(estimated_with_raw > 0);
    app.clear_active_session_steps();

    assert!(app.active_session_view_state.step_llm_raw.is_empty());
    assert!(app.active_session_view_state.estimated_bytes() < estimated_with_raw);
}

#[test]
fn sync_active_session_from_chat_only_advances_session_timestamp() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.sessions = vec![session_info("active", "/active", 2_000)];
    app.active_session_view_state.updated_ms = 1_000;
    app.chat = vec![message(ChatRole::User, "question")];

    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
    assert_eq!(app.active_session_view_state.message_meta_texts, vec![None]);

    app.sessions[0].time.updated = 500;
    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
}

#[test]
fn cache_pruning_retains_requesting_sessions_and_removes_inactive_sessions() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("requesting".to_string());
    app.chat = vec![message(ChatRole::User, "kept")];
    app.chat_message_ids = vec![Some("message-1".to_string())];
    app.get_session_runtime_mut("requesting").is_requesting = true;
    app.store_session_chat_snapshot(
        "inactive".to_string(),
        shared_chat_messages(vec![message(ChatRole::User, "removed")]),
        vec![Some("message-2".to_string())],
    );

    app.cache_active_session_chat();

    assert!(app.cached_chat_messages("requesting").is_some());
    assert_eq!(
        app.cached_chat_message_ids("requesting"),
        Some(vec![Some("message-1".to_string())])
    );
    assert!(app.cached_chat_messages("inactive").is_none());
    assert!(app.cached_chat_message_ids("inactive").is_none());

    app.get_session_runtime_mut("requesting").is_requesting = false;
    app.prune_inactive_session_chat_cache();
    assert!(app.cached_chat_messages("requesting").is_none());
    assert!(app.cached_chat_message_ids("requesting").is_none());
}

#[test]
fn restore_chat_uses_cached_messages_ids_and_missing_id_defaults() {
    let (mut app, _task) = App::new();
    app.store_session_chat_snapshot(
        "with-ids".to_string(),
        shared_chat_messages(vec![message(ChatRole::Assistant, "cached")]),
        vec![Some("message-1".to_string())],
    );
    app.session_chat_cache.insert(
        "without-ids".to_string(),
        shared_chat_messages(vec![
            message(ChatRole::User, "first"),
            message(ChatRole::Assistant, "second"),
        ]),
    );

    app.restore_chat_for_session("with-ids");
    assert_eq!(app.chat[0].content, "cached");
    assert_eq!(app.chat_message_ids, vec![Some("message-1".to_string())]);

    app.restore_chat_for_session("without-ids");
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat_message_ids, vec![None, None]);

    app.restore_chat_for_session("missing");
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
}

#[test]
fn known_session_directory_prefers_loaded_sessions_then_searches_projects() {
    let (mut app, _task) = App::new();
    app.sessions = vec![session_info("duplicate", "/loaded", 1)];
    app.project_sessions.insert(
        "/project".to_string(),
        vec![
            session_info("duplicate", "/project-duplicate", 1),
            session_info("project-only", "/project-only", 1),
        ],
    );

    assert_eq!(app.known_session_directory("duplicate").as_deref(), Some("/loaded"));
    assert_eq!(app.known_session_directory("project-only").as_deref(), Some("/project-only"));
    assert_eq!(app.known_session_directory("missing"), None);
}

#[test]
fn history_reset_boundary_replaces_previous_value_and_is_session_scoped() {
    let (mut app, _task) = App::new();
    app.mark_session_history_reset_boundary("session-a".to_string(), vec![Some("old".to_string())]);
    app.mark_session_history_reset_boundary(
        "session-a".to_string(),
        vec![Some("retained".to_string()), None],
    );

    assert_eq!(
        app.session_history_reset_boundary("session-a"),
        Some([Some("retained".to_string()), None].as_slice())
    );
    assert_eq!(app.session_history_reset_boundary("session-b"), None);
}

#[test]
fn active_shared_chat_messages_reuses_only_a_length_matching_active_snapshot() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = vec![message(ChatRole::User, "live")];
    let matching = shared_chat_messages(vec![message(ChatRole::User, "cached")]);
    app.store_session_chat_snapshot("active".to_string(), matching.clone(), vec![None]);

    let reused = app.active_shared_chat_messages();
    assert!(Arc::ptr_eq(&reused, &matching));
    assert_eq!(reused[0].content, "cached");

    app.chat.push(message(ChatRole::Assistant, "new"));
    let rebuilt = app.active_shared_chat_messages();
    assert!(!Arc::ptr_eq(&rebuilt, &matching));
    assert_eq!(rebuilt.len(), 2);
    assert_eq!(rebuilt[0].content, "live");
}

#[test]
fn switch_stashes_previous_chat_without_cloning_payload() {
    let (mut app, _task) = App::new();
    let content = "large message payload ".repeat(4_096);
    let expected_pointer = content.as_ptr();
    app.active_session_id = Some("previous".to_string());
    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    app.stash_active_session_view();

    let snapshot = app.session_view_cache.get("previous").expect("previous view should be cached");
    assert_eq!(snapshot.chat()[0].content.as_ptr(), expected_pointer);
    assert!(app.chat.is_empty());
}
