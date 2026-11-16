use crate::message::chat::stream::bind_message_model_to_step;
use crate::session::backfill_message_models_from_steps;
use crate::session::loading::prepare_bounded_session_open;
use session_ui_types::{ChatMessage, ChatRole, ChatSessionStep, SessionMessageRecord, TokenUsage};

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
fn bounded_summary(message_count: usize) -> vw_api_session::GatewaySessionUiSummary {
    vw_api_session::GatewaySessionUiSummary {
        agent_binding: None,
        full_access: None,
        id: "session-page".to_string(),
        title: "Paged".to_string(),
        updated_ms: 42,
        message_count,
        call_count: 0,
        last_content: None,
    }
}

fn page_record(seq_no: u64) -> SessionMessageRecord {
    SessionMessageRecord {
        seq_no,
        message_id: Some(format!("message-{seq_no}")),
        message: ChatMessage {
            role: session_ui_types::ChatRole::User,
            content: format!("message {seq_no}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    }
}

#[test]
fn session_open_page_keeps_initial_history_bounded() {
    let page = vw_api_session::GatewaySessionMessagePage {
        items: (9_937..=10_000).map(page_record).collect(),
        next_cursor: Some("older-page".to_string()),
        previous_cursor: None,
        total_count: 10_000,
    };

    let prepared = prepare_bounded_session_open(bounded_summary(10_000), page, |_| ());

    assert_eq!(prepared.messages.len(), 64);
    let timeline = prepared.timeline_page.expect("bounded open should carry timeline page");
    assert_eq!(timeline.entries.len(), 64);
    assert_eq!(timeline.total_count, 10_000);
    assert_eq!(timeline.before_cursor.as_deref(), Some("older-page"));
}

#[test]
fn session_open_page_becomes_ready_from_latest_page_without_heavy_state() {
    let page = vw_api_session::GatewaySessionMessagePage {
        items: vec![page_record(10_000)],
        next_cursor: None,
        previous_cursor: None,
        total_count: 10_000,
    };

    let prepared = prepare_bounded_session_open(bounded_summary(10_000), page, |_| ());

    assert!(prepared.base_ui_chunk.is_some());
    assert!(prepared.steps.is_empty());
    assert!(prepared.calls.is_empty());
    assert_eq!(prepared.updated_ms, 42);
}

#[test]
fn agent_unified_selection_bounded_load_preserves_trusted_identity() {
    let mut summary = bounded_summary(0);
    summary.agent_binding = Some(vw_api_chat_identity::SessionAgentIdentity {
        agent: "acp-codex".into(),
        label: Some("编程助手".into()),
        acp_agent: Some("codex".into()),
    });
    let prepared = prepare_bounded_session_open(
        summary,
        vw_api_session::GatewaySessionMessagePage {
            items: vec![],
            next_cursor: None,
            previous_cursor: None,
            total_count: 0,
        },
        |_| (),
    );
    assert_eq!(prepared.agent_binding.unwrap().acp_agent.as_deref(), Some("codex"));
}
