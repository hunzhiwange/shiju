use super::*;

#[test]
fn stream_part_builders_fill_expected_message_boundaries() {
    let base = stream_part_base("ses-1", "msg-assistant", "prt-base".to_string());
    let step_start = build_stream_step_start_part(
        "ses-1",
        "msg-assistant",
        "prt-start".to_string(),
        Some("snapshot-before".to_string()),
    );
    let step_finish = build_stream_step_finish_part(
        "ses-1",
        "msg-assistant",
        "prt-finish".to_string(),
        "snapshot-after".to_string(),
    );
    let patch = build_stream_patch_part(
        "ses-1",
        "msg-assistant",
        "prt-patch".to_string(),
        "hash".to_string(),
        vec!["src/main.rs".to_string()],
        Some((7, 3)),
    );

    assert_eq!(base.session_id, "ses-1");
    assert_eq!(base.message_id, "msg-assistant");

    match step_start {
        session_engine::message::Part::StepStart(part) => {
            assert_eq!(part.base.id, "prt-start");
            assert_eq!(part.snapshot.as_deref(), Some("snapshot-before"));
        }
        _ => panic!("expected step start part"),
    }

    match step_finish {
        session_engine::message::Part::StepFinish(part) => {
            assert_eq!(part.base.id, "prt-finish");
            assert_eq!(part.reason, "tool_round");
            assert_eq!(part.snapshot.as_deref(), Some("snapshot-after"));
            assert_eq!(part.tokens.total, Some(0));
        }
        _ => panic!("expected step finish part"),
    }

    match patch {
        session_engine::message::Part::Patch(part) => {
            assert_eq!(part.base.id, "prt-patch");
            assert_eq!(part.hash, "hash");
            assert_eq!(part.files, vec!["src/main.rs".to_string()]);
            assert_eq!(part.additions, Some(7));
            assert_eq!(part.deletions, Some(3));
        }
        _ => panic!("expected patch part"),
    }
}

#[test]
fn generated_stream_parts_and_preallocated_ids_are_non_empty() {
    let start = new_stream_step_start_part("ses-1", "msg-a", None).expect("start part");
    let finish =
        new_stream_step_finish_part("ses-1", "msg-a", "snap".to_string()).expect("finish part");
    let patch = new_stream_patch_part(
        "ses-1",
        "msg-a",
        "hash".to_string(),
        vec!["file".to_string()],
        Some((2, 1)),
    )
    .expect("patch part");
    let ids = preallocate_stream_turn_message_ids().expect("preallocated ids");

    assert!(matches!(start, session_engine::message::Part::StepStart(_)));
    assert!(matches!(finish, session_engine::message::Part::StepFinish(_)));
    assert!(matches!(patch, session_engine::message::Part::Patch(_)));
    assert!(!ids.assistant_id.is_empty());
    assert!(!ids.user_id.is_empty());
    assert_ne!(ids.assistant_id, ids.user_id);
}

#[tokio::test]
async fn collect_gateway_chat_stream_payloads_returns_output_on_done() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type": "chat.delta", "delta": "hello"})).expect("send delta");
    tx.send(serde_json::json!({
        "type": "chat.done",
        "message_id": "msg-a",
        "parent_message_id": "msg-u"
    }))
    .expect("send done");

    let mut seen_types = Vec::new();
    let collected = collect_gateway_chat_stream_payloads(rx, |payload| {
        if let Some(kind) = payload.get("type").and_then(Value::as_str) {
            seen_types.push(kind.to_string());
        }
    })
    .await
    .expect("collect stream");

    assert_eq!(collected.output, "hello");
    assert_eq!(collected.message_id.as_deref(), Some("msg-a"));
    assert_eq!(collected.parent_message_id.as_deref(), Some("msg-u"));
    assert_eq!(seen_types, vec!["chat.delta", "chat.done"]);
}

#[tokio::test]
async fn collect_gateway_chat_stream_payloads_rejects_close_without_done() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type": "chat.delta", "delta": "partial"})).expect("send delta");
    drop(tx);

    let error = collect_gateway_chat_stream_payloads(rx, |_| {})
        .await
        .expect_err("missing done should fail");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.to_string().contains("closed before completion"));
}

#[test]
fn merge_gateway_request_options_fills_missing_agent_defaults() {
    let options = merge_gateway_request_options(
        Some(serde_json::json!({"custom": "kept"})),
        Some("agent-a".to_string()),
        Some(vec!["shell".to_string(), "file_read".to_string()]),
        Some("agent system".to_string()),
        Some(0.25),
        Some(0.75),
        None,
        None,
        None,
    );

    assert_eq!(options.get("custom").and_then(Value::as_str), Some("kept"));
    assert_eq!(options.get("agent").and_then(Value::as_str), Some("agent-a"));
    assert_eq!(options.get("allowed_tools"), Some(&serde_json::json!(["shell", "file_read"])));
    assert_eq!(options.get("chat_system_prompt").and_then(Value::as_str), Some("agent system"));
    assert_eq!(options.get("temperature").and_then(Value::as_f64), Some(0.25));
    assert_eq!(options.get("top_p").and_then(Value::as_f64), Some(0.75));
}

#[test]
fn merge_gateway_request_options_existing_acp_request_reuses_allowed_tools() {
    use vw_api_chat_identity::ReasoningEffortDto;

    let options = merge_gateway_request_options(
        Some(serde_json::json!({"acp_test": true})),
        None,
        Some(vec!["shell".to_string()]),
        None,
        None,
        None,
        None,
        None,
        Some(ReasoningEffortDto::High),
    );

    assert_eq!(options.get("acp_test").and_then(Value::as_bool), Some(true));
    assert_eq!(options.get("acp_allowed_tools"), Some(&serde_json::json!(["shell"])));
    assert_eq!(options.get("reasoning_effort").and_then(Value::as_str), Some("high"));
}

#[test]
fn merge_gateway_request_options_non_acp_removes_reasoning_aliases() {
    let options = merge_gateway_request_options(
        Some(serde_json::json!({
            "reasoning_effort": "stale",
            "thought_level": "legacy",
            "custom": true
        })),
        None,
        Some(vec!["shell".to_string()]),
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert_eq!(options.get("custom").and_then(Value::as_bool), Some(true));
    assert!(options.get("reasoning_effort").is_none());
    assert!(options.get("thought_level").is_none());
    assert!(options.get("acp_test").is_none());
    assert!(options.get("acp_allowed_tools").is_none());
}

#[test]
fn merge_gateway_request_options_explicit_acp_tools_enable_acp() {
    let options = merge_gateway_request_options(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(vec!["file_read".to_string()]),
        None,
    );

    assert_eq!(options.get("acp_test").and_then(Value::as_bool), Some(true));
    assert_eq!(options.get("acp_allowed_tools"), Some(&serde_json::json!(["file_read"])));
}

#[test]
fn stream_patch_recorder_no_active_step_is_a_noop() {
    let mut recorder = StreamPatchRecorder::default();

    recorder.finish_tool_round("ses-1", "msg-a", "/unused", 1);

    assert!(recorder.active_step.is_none());
    assert!(recorder.take_parts().is_empty());
}

// 步骤记录器只提交当前活跃步骤，错配事件会丢弃暂存状态而不写入快照。
#[test]
fn stream_patch_recorder_begin_step_records_start_and_active_step() {
    let root = tempfile::tempdir().expect("temporary root");
    let mut recorder = StreamPatchRecorder::default();

    recorder.begin_step("ses-1", "msg-a", &root.path().to_string_lossy(), 4);

    let active = recorder.active_step.as_ref().expect("active step");
    assert_eq!(active.step_index, 4);
    let parts = recorder.take_parts();
    assert_eq!(parts.len(), 1);
    assert!(matches!(parts.first(), Some(session_engine::message::Part::StepStart(_))));
}

#[test]
fn stream_patch_recorder_mismatched_step_discards_active_step_without_snapshot_io() {
    let mut recorder = StreamPatchRecorder {
        active_step: Some(StreamStepSnapshot {
            step_index: 2,
            snapshot: Some("snapshot-before".to_string()),
        }),
        parts: Vec::new(),
    };

    recorder.finish_tool_round("ses-1", "msg-a", "/unused", 3);

    assert!(recorder.active_step.is_none());
    assert!(recorder.take_parts().is_empty());
}

#[test]
fn stream_patch_recorder_take_parts_drains_recorded_parts() {
    let part = build_stream_patch_part(
        "ses-1",
        "msg-a",
        "prt-1".to_string(),
        "hash-1".to_string(),
        vec!["src/lib.rs".to_string()],
        None,
    );
    let mut recorder = StreamPatchRecorder { active_step: None, parts: vec![part] };

    let first = recorder.take_parts();
    let second = recorder.take_parts();

    assert_eq!(first.len(), 1);
    assert!(matches!(first.first(), Some(session_engine::message::Part::Patch(_))));
    assert!(second.is_empty());
}

#[test]
fn patch_line_stats_accumulate_all_file_diffs() {
    let diffs = vec![
        snapshot::FileDiff {
            file: "src/lib.rs".to_string(),
            before: String::new(),
            after: String::new(),
            additions: 4,
            deletions: 1,
            status: Some(snapshot::DiffStatus::Modified),
        },
        snapshot::FileDiff {
            file: "src/main.rs".to_string(),
            before: String::new(),
            after: String::new(),
            additions: 3,
            deletions: 2,
            status: Some(snapshot::DiffStatus::Modified),
        },
    ];

    assert_eq!(patch_line_stats(&diffs), (7, 3));
}

#[tokio::test]
async fn collect_gateway_chat_stream_payloads_concatenates_deltas_and_ignores_other_events() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type": "chat.step_start", "step_index": 0}))
        .expect("send step start");
    tx.send(serde_json::json!({"type": "chat.delta", "delta": 42})).expect("send malformed delta");
    tx.send(serde_json::json!({"type": "chat.delta", "delta": "hello"})).expect("send first delta");
    tx.send(serde_json::json!({"type": "chat.delta", "delta": " world"}))
        .expect("send second delta");
    tx.send(serde_json::json!({"type": "chat.done", "message_id": null})).expect("send done");

    let collected = collect_gateway_chat_stream_payloads(rx, |_| {}).await.expect("collect stream");

    assert_eq!(collected.output, "hello world");
    assert_eq!(collected.message_id, None);
    assert_eq!(collected.parent_message_id, None);
}

#[tokio::test]
async fn collect_gateway_chat_stream_payloads_returns_stream_error() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type": "chat.error", "error": "provider failed"}))
        .expect("send error");

    let error = collect_gateway_chat_stream_payloads(rx, |_| {})
        .await
        .expect_err("stream error should fail collection");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.to_string().contains("provider failed"));
}

#[tokio::test]
async fn collect_gateway_chat_stream_payloads_uses_fallback_for_malformed_error() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type": "chat.error", "error": {"code": 500}}))
        .expect("send malformed error");

    let error = collect_gateway_chat_stream_payloads(rx, |_| {})
        .await
        .expect_err("malformed stream error should fail collection");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.to_string().contains("gateway chat stream failed"));
}

fn pending_stream_ui_session(
    entries: &[(ui_models::ChatRole, &str, Option<&str>)],
) -> ui_models::ChatSession {
    ui_models::ChatSession {
        id: "pending-stream".to_string(),
        title: "pending".to_string(),
        messages: entries
            .iter()
            .map(|(role, content, _)| ui_models::ChatMessage {
                role: *role,
                content: (*content).to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
            .collect(),
        message_ids: entries.iter().map(|(_, _, id)| id.map(str::to_string)).collect(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 1,
    }
}

#[test]
fn stream_ui_commit_replaces_current_draft_despite_tool_markers_or_partial_text() {
    use ui_models::ChatRole::{Assistant, User};
    for draft in
        ["", "partial", "tool read\n{\"status\":\"completed\"}\n<think>reason</think>answer"]
    {
        let mut session =
            pending_stream_ui_session(&[(User, "question", None), (Assistant, draft, None)]);
        let final_message =
            pending_stream_ui_session(&[(Assistant, "<think>reason</think>answer", None)])
                .messages
                .remove(0);
        let user = session.messages[0].clone();
        upsert_stream_ui_message(&mut session, "user-current", None, user);
        upsert_stream_ui_message(
            &mut session,
            "assistant-current",
            Some("user-current"),
            final_message.clone(),
        );
        upsert_stream_ui_message(
            &mut session,
            "assistant-current",
            Some("user-current"),
            final_message.clone(),
        );
        assert_eq!(session.messages.len(), 2, "draft: {draft}");
        assert_eq!(session.messages[1], final_message);
        assert_eq!(
            session.message_ids,
            vec![Some("user-current".into()), Some("assistant-current".into())]
        );
    }
}

#[test]
fn stream_ui_commit_does_not_claim_another_turns_identical_answer() {
    use ui_models::ChatRole::{Assistant, User};
    let mut session = pending_stream_ui_session(&[
        (User, "previous", Some("user-previous")),
        (Assistant, "answer", None),
        (User, "current", Some("user-current")),
        (Assistant, "old response", Some("assistant-existing")),
        (User, "next", Some("user-next")),
        (Assistant, "answer", None),
    ]);
    let final_message = session.messages[1].clone();
    upsert_stream_ui_message(
        &mut session,
        "assistant-current",
        Some("user-current"),
        final_message,
    );
    assert_eq!(session.messages.len(), 7);
    assert_eq!(session.message_ids[1], None);
    assert_eq!(session.message_ids[3].as_deref(), Some("assistant-existing"));
    assert_eq!(session.message_ids[5], None);
    assert_eq!(session.message_ids[6].as_deref(), Some("assistant-current"));
}

#[test]
fn shell_session_running_update_retains_gateway_metadata_without_finishing() {
    let event = session_engine::processor_events::ToolEvent {
        call_id: "bash-call".into(),
        name: "bash".into(),
        kind: session_engine::processor_events::ToolEventKind::Update,
        status: session_engine::processor_events::ToolCallStatus::Running,
        occurred_ms: 10,
        payload: serde_json::json!({"status":"running", "metadata":{"process_id":"owned", "next_cursor":3, "state":"running"}}),
    };
    let mapped =
        processor_event_from_prompt_event(session_port::PromptEvent::Tool(event));
    let mut recorder = StreamTrajectoryRecorder::new("assistant".into());
    recorder.record(&mapped);
    let session_engine::processor_events::StreamEvent::Tool(event) = mapped else { panic!("tool event") };
    assert_eq!(event.payload["metadata"]["process_id"], "owned");
    assert_eq!(event.payload["metadata"]["next_cursor"], 3);
    assert_eq!(event.status, session_engine::processor_events::ToolCallStatus::Running);
    assert_eq!(recorder.steps[0].tool_timings[0].finished_ms, None);
}
