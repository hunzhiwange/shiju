//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use iced::widget::text_editor;

#[test]
fn prioritize_chat_ui_chunk_starts_keeps_base_first_on_initial_plan() {
    let (chunk_starts, anchor_chunk_start) =
        prioritize_chat_ui_chunk_starts(160, 64, 96, true, None, 32);

    assert_eq!(anchor_chunk_start, Some(64));
    assert_eq!(chunk_starts, vec![32, 64, 96]);
}

#[test]
fn prioritize_chat_ui_chunk_starts_prefers_forward_chunks_when_scrolling_down() {
    let (chunk_starts, anchor_chunk_start) =
        prioritize_chat_ui_chunk_starts(192, 72, 104, true, Some(32), 32);

    assert_eq!(anchor_chunk_start, Some(64));
    assert_eq!(chunk_starts, vec![96, 64, 128, 32]);
}

#[test]
fn prioritize_chat_ui_chunk_starts_prefers_backward_chunks_when_scrolling_up() {
    let (chunk_starts, anchor_chunk_start) =
        prioritize_chat_ui_chunk_starts(192, 40, 72, true, Some(96), 32);

    assert_eq!(anchor_chunk_start, Some(32));
    assert_eq!(chunk_starts, vec![32, 64, 0, 96]);
}

#[test]
fn chunk_scheduler_retains_anchor_and_expanded_text() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = (0..260)
        .map(|idx| ChatMessage {
            role: if idx == 259 { ChatRole::Assistant } else { ChatRole::User },
            content: format!("message {idx}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.chat_virtualizer.sync_len(app.chat.len(), 100.0);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.scroll_to_item_anchor(170, 0.0, 0.0);
    app.current_session_runtime_mut().is_requesting = true;

    let anchor_key = (170_u64) << 32;
    let expanded_key = (210_u64) << 32;
    let think_key = (211_u64) << 32;
    let tail_key = (259_u64) << 32;
    let evicted_key = (96_u64) << 32;
    app.chat_message_expanded.insert(210);
    app.chat_think_expanded.insert(think_key);
    app.chat_special_text_editors.insert(anchor_key, text_editor::Content::with_text("anchor"));
    app.chat_special_text_editors.insert(expanded_key, text_editor::Content::with_text("expanded"));
    app.chat_think_editors.insert(think_key, text_editor::Content::with_text("think"));
    app.chat_special_text_editors.insert(tail_key, text_editor::Content::with_text("tail"));
    app.chat_special_text_editors.insert(evicted_key, text_editor::Content::with_text("evicted"));

    app.prune_chat_heavy_editor_caches(64, 96);

    assert!(app.chat_special_text_editors.contains_key(&anchor_key));
    assert!(app.chat_special_text_editors.contains_key(&expanded_key));
    assert!(app.chat_think_editors.contains_key(&think_key));
    assert!(app.chat_special_text_editors.contains_key(&tail_key));
    assert!(!app.chat_special_text_editors.contains_key(&evicted_key));
}

#[test]
fn apply_prepared_chat_ui_phase_preserves_explore_summary_animation_history() {
    let (mut app, _task) = App::new();
    let first = concat!(
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n"
    );
    let second = concat!(
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n",
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n"
    );

    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: first.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.chat_message_ids = vec![None];
    app.apply_prepared_chat_ui_phase(prepare_chat_ui_chunk_phase(&app.chat, 0, true, false));

    let key = explore_summary_animation_key(0, 0);
    let initial_state = app
        .chat_explore_summary_animations
        .get(&key)
        .expect("initial explore summary animation state should exist");
    assert_eq!(initial_state.previous_summary_text, "1 次读取");
    assert_eq!(initial_state.current_summary_text, "1 次读取");
    assert_eq!(initial_state.changed_at_ms, None);

    app.chat[0].content = second.to_string();
    app.apply_prepared_chat_ui_phase(prepare_chat_ui_chunk_phase(&app.chat, 0, true, false));

    let updated_state = app
        .chat_explore_summary_animations
        .get(&key)
        .expect("updated explore summary animation state should exist");
    assert_eq!(updated_state.previous_summary_text, "1 次读取");
    assert_eq!(updated_state.current_summary_text, "2 次读取");
    assert!(updated_state.changed_at_ms.is_some());
}

#[test]
fn prepare_chat_ui_chunk_phase_uses_display_text_for_tool_editor_cache() {
    let chat = vec![ChatMessage {
        role: ChatRole::Tool,
        content: concat!(
            "tool grep\n",
            "{\"status\":\"completed\",\"input\":\"selectors\",\"output\":\"2 matches\"}\n"
        )
        .to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let PreparedChatUiPhase::Base(chunk) = prepare_chat_ui_chunk_phase(&chat, 0, true, false)
    else {
        panic!("base phase should be returned");
    };

    assert_eq!(chunk.message_editor_texts[0].as_deref(), Some("2 matches"));
    assert_eq!(chunk.visible_texts[0].as_deref(), Some(chat[0].content.as_str()));
}

#[test]
fn rebuild_active_session_message_meta_keeps_tool_outside_step_indexing() {
    let (mut app, _task) = App::new();
    app.active_session_view_state.updated_ms = 3_000;
    app.chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "先给出结论".to_string(),
            model: Some("message-model-a".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Tool,
            content: "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}\n".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            model: None,
            role: ChatRole::User,
            content: "继续".to_string(),
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    app.upsert_active_session_step(ChatSessionStep {
        index: 1,
        turn_id: None,
        started_ms: 1_000,
        finished_ms: Some(1_400),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: Some("step-model-a".to_string()),
        tool_timings: Vec::new(),
    });
    app.upsert_active_session_step(ChatSessionStep {
        index: 2,
        turn_id: None,
        started_ms: 2_000,
        finished_ms: Some(2_200),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: Some("step-model-b".to_string()),
        tool_timings: Vec::new(),
    });

    app.rebuild_active_session_message_meta();

    assert_eq!(app.active_session_view_state.message_meta_texts.len(), 3);
    assert!(
        app.active_session_view_state.message_meta_texts[0]
            .as_deref()
            .is_some_and(|text| text.contains("message-model-a"))
    );
    assert_eq!(app.active_session_view_state.message_meta_texts[1], None);
    assert_eq!(app.active_session_view_state.message_meta_texts[2], None);
}

#[test]
fn rebuild_active_session_message_meta_does_not_relabel_unrecorded_model() {
    let (mut app, _task) = App::new();
    app.chat = vec![
        ChatMessage {
            model: None,
            role: ChatRole::User,
            content: "问题".to_string(),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "回答".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    app.upsert_active_session_step(ChatSessionStep {
        index: 1,
        turn_id: None,
        started_ms: 1_000,
        finished_ms: Some(1_400),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("stop".to_string()),
        model: None,
        tool_timings: Vec::new(),
    });

    app.current_session_runtime_mut().model = "model-b".to_string();
    app.rebuild_active_session_message_meta();

    assert_eq!(app.active_session_view_state.message_meta_texts, vec![None, None]);
}

#[test]
fn message_model_binding_updates_only_the_target_step() {
    let mut messages = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "first question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "first answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: "second question".to_string(),
            model: Some("history-model".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "second answer".to_string(),
            model: Some("history-model".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    bind_message_model_to_step(&mut messages, 1, "provider/model-a");

    assert_eq!(messages[0].model.as_deref(), Some("provider/model-a"));
    assert_eq!(messages[1].model.as_deref(), Some("provider/model-a"));
    assert_eq!(messages[2].model.as_deref(), Some("history-model"));
    assert_eq!(messages[3].model.as_deref(), Some("history-model"));
}

#[test]
fn legacy_backfill_does_not_overwrite_persisted_message_models() {
    let mut messages = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "question".to_string(),
            model: Some("persisted-model".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    let steps = vec![ChatSessionStep {
        index: 1,
        turn_id: None,
        started_ms: 1,
        finished_ms: Some(2),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some("step-model".to_string()),
        tool_timings: Vec::new(),
    }];

    backfill_message_models_from_steps(&mut messages, &steps);

    assert_eq!(messages[0].model.as_deref(), Some("persisted-model"));
    assert_eq!(messages[1].model.as_deref(), Some("step-model"));
}
