// 覆盖会话流事件转换、历史整理及代理选项合并。

use super::*;
use crate::config::DelegateAgentConfig;
use axum::http::header;

// 流式事件先转换为稳定会话片段，再验证模型和工具选项不会泄漏到不兼容后端。
#[test]
fn tool_content_to_session_text_wraps_plain_output() {
    let text = tool_content_to_session_text("done");

    assert!(text.contains("[Tool results]"));
    assert!(text.contains("done"));
}

#[test]
fn tool_content_to_session_text_extracts_json_tool_result_and_id() {
    let text = tool_content_to_session_text(
        r#"{"tool_call_id":"call-1","content":"line one\nline two","ignored":true}"#,
    );

    assert!(text.contains(r#"<tool_result id="call-1">"#));
    assert!(text.contains("line one\nline two"));
    assert!(!text.contains("ignored"));
}

#[test]
fn tool_content_to_session_text_falls_back_to_original_json_without_string_content() {
    let input = r#"{"tool_call_id":"call-1","content":{"ok":true}}"#;
    let text = tool_content_to_session_text(input);

    assert!(text.contains(r#"<tool_result id="call-1">"#));
    assert!(text.contains(input));
}

#[test]
fn split_stream_model_ref_handles_provider_and_model() {
    let model_ref = split_stream_model_ref(Some("openai/gpt-4.1"));

    assert_eq!(model_ref.provider_id, "openai");
    assert_eq!(model_ref.model_id, "gpt-4.1");
}

#[test]
fn split_stream_model_ref_trims_empty_and_provider_only_values() {
    let empty = split_stream_model_ref(Some("  "));
    let provider_only = split_stream_model_ref(Some("anthropic"));

    assert_eq!(empty.provider_id, "");
    assert_eq!(empty.model_id, "");
    assert_eq!(provider_only.provider_id, "");
    assert_eq!(provider_only.model_id, "anthropic");
}

#[test]
fn normalize_tool_ids_drops_empty_items() {
    let ids = normalize_tool_ids(vec![" shell ".to_string(), " ".to_string()]);

    assert_eq!(ids, Some(vec!["shell".to_string()]));
}

#[test]
fn normalize_tool_ids_deduplicates_and_returns_none_when_empty() {
    let ids = normalize_tool_ids(vec![
        " shell ".to_string(),
        "shell".to_string(),
        " file_read ".to_string(),
    ]);
    let empty = normalize_tool_ids(vec![" ".to_string(), "".to_string()]);

    assert_eq!(ids, Some(vec!["shell".to_string(), "file_read".to_string()]));
    assert_eq!(empty, None);
}

#[test]
fn forced_child_model_overrides_parent_selection() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        model: "qwen3.7-plus".to_string(),
        force_model_selection: true,
        ..DelegateAgentConfig::default()
    };

    assert_eq!(
        delegated_model_override(&agent, Some("wrong-provider/qwen3.7-plus".to_string())),
        Some("token-plan/qwen3.7-plus".to_string())
    );
}

#[test]
fn incomplete_forced_child_binding_is_ignored() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        force_model_selection: true,
        ..DelegateAgentConfig::default()
    };

    assert_eq!(
        delegated_model_override(&agent, Some("token-plan/qwen3.7-plus".to_string())),
        Some("token-plan/qwen3.7-plus".to_string())
    );
}

#[test]
fn child_soft_preference_does_not_bypass_profile_fit() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        model: "qwen3.7-plus".to_string(),
        ..DelegateAgentConfig::default()
    };

    assert_eq!(delegated_model_override(&agent, None), None);
}

// 历史整理保留角色与工具结果，只把最后一个用户输入提升为本轮查询。
#[test]
fn prepare_stream_history_maps_roles_tool_results_and_last_user_query() {
    let prepared = prepare_stream_history(
        vec![
            serde_json::json!({"role": "system", "content": "system rules"}),
            serde_json::json!({"role": "assistant", "content": "previous answer"}),
            serde_json::json!({
                "role": "tool",
                "content": "{\"tool_call_id\":\"call-1\",\"content\":\"tool ok\"}"
            }),
            serde_json::json!({"role": "unknown", "content": "fallback user"}),
            serde_json::json!({"role": "user", "content": "next question"}),
        ],
        false,
        false,
    );

    assert_eq!(prepared.query, "next question");
    assert_eq!(prepared.history.len(), 4);
    assert!(matches!(prepared.history[0].role, ui_models::ChatRole::System));
    assert!(matches!(prepared.history[1].role, ui_models::ChatRole::Assistant));
    assert!(matches!(prepared.history[2].role, ui_models::ChatRole::Tool));
    assert!(prepared.history[2].content.contains(r#"<tool_result id="call-1">"#));
    assert!(prepared.history[2].content.contains("tool ok"));
    assert!(matches!(prepared.history[3].role, ui_models::ChatRole::User));
    assert_eq!(prepared.history[3].content, "fallback user");
}

#[test]
fn prepare_stream_history_promotes_last_prior_user_unless_resume_history_only() {
    let messages = vec![
        serde_json::json!({"role": "user", "content": "resume from here"}),
        serde_json::json!({"role": "assistant", "content": "latest assistant"}),
    ];

    let normal = prepare_stream_history(messages.clone(), false, false);
    assert_eq!(normal.query, "resume from here");
    assert_eq!(normal.history.len(), 1);
    assert!(matches!(normal.history[0].role, ui_models::ChatRole::Assistant));

    let resume_only = prepare_stream_history(messages, true, false);
    assert!(resume_only.query.is_empty());
    assert_eq!(resume_only.history.len(), 2);
    assert!(matches!(resume_only.history[0].role, ui_models::ChatRole::User));
}

#[test]
fn prepare_stream_history_accepts_structured_attachment_blocks() {
    let messages = vec![serde_json::json!({
        "role": "user",
        "content": [
            {"type": "text", "text": "inspect"},
            {"type": "image", "mimeType": "image/png", "data": "QUJDRA=="},
            {"type": "resource", "resource": {"uri": "file:///notes.md", "text": "body"}}
        ]
    })];

    let native = prepare_stream_history(messages.clone(), false, false);
    assert!(native.query.contains("inspect"));
    assert!(native.query.contains("[IMAGE:data:image/png;base64,QUJDRA==]"));
    assert!(native.query.contains("[Resource: file:///notes.md]\nbody"));

    let acp = prepare_stream_history(messages, false, true);
    let blocks: serde_json::Value = serde_json::from_str(&acp.query).expect("ACP blocks");
    assert_eq!(blocks[1]["type"], "image");
    assert_eq!(blocks[2]["type"], "resource");
}

// 不同流事件映射到稳定载荷字段，结束与错误状态不得伪装成普通文本增量。
// 错误用例固定失败发生在持久化之前，避免跨会话或跨作用域修改已有数据。
#[test]
fn stream_payload_for_non_done_event_encodes_delta_step_finish_and_error_state() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = Some("provider/old".to_string());
    let mut recorder = StreamPatchRecorder::default();

    let context = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::ContextLoaded {
            paths: vec!["/workspace/AGENTS.md".to_string()],
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "",
        None,
        "/unused",
    )
    .expect("context payload");
    assert_eq!(
        context,
        serde_json::json!({
            "type": "chat.context_loaded",
            "paths": ["/workspace/AGENTS.md"]
        })
    );

    let delta = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::Delta("hello".to_string()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "",
        None,
        "/unused",
    )
    .expect("delta payload");
    assert_eq!(delta, serde_json::json!({"type": "chat.delta", "delta": "hello"}));
    assert_eq!(assistant_text, "hello");

    let usage = ui_models::TokenUsage {
        input_tokens: 1,
        output_tokens: 2,
        cached_tokens: 3,
        reasoning_tokens: 4,
    };
    let finish = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::StepFinish {
            step_index: 7,
            finished_ms: 99,
            usage: usage.clone(),
            finish_reason: Some("stop".to_string()),
            model: Some("provider/new".to_string()),
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "",
        None,
        "/unused",
    )
    .expect("finish payload");
    assert_eq!(finish["type"], "chat.step_finish");
    assert_eq!(finish["step_index"], 7);
    assert_eq!(finish["finished_ms"], 99);
    assert_eq!(finish["usage"], serde_json::to_value(&usage).unwrap());
    assert_eq!(finish_reason.as_deref(), Some("stop"));
    assert_eq!(assistant_model.as_deref(), Some("provider/new"));

    let error = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::Error("failed".to_string()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "",
        None,
        "/unused",
    )
    .expect("error payload");
    assert_eq!(error, serde_json::json!({"type": "chat.error", "error": "failed"}));
}

#[test]
fn compaction_lifecycle_events_encode_stable_gateway_payloads() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut recorder = StreamPatchRecorder::default();
    let mut encode = |phase| {
        stream_payload_for_non_done_event(
            agent_session::processor::StreamEvent::Compaction { phase },
            &mut assistant_text,
            &mut finish_reason,
            &mut assistant_model,
            &mut recorder,
            "session-id",
            None,
            "/unused",
        )
        .expect("compaction payload")
    };

    assert_eq!(
        encode(agent_session::processor::CompactionPhase::Started),
        serde_json::json!({"type": "chat.compaction", "phase": "started"})
    );
    assert_eq!(
        encode(agent_session::processor::CompactionPhase::Completed),
        serde_json::json!({"type": "chat.compaction", "phase": "completed"})
    );
    assert!(assistant_text.is_empty());
}

#[test]
fn gateway_stream_preserves_existing_tool_payloads() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut recorder = StreamPatchRecorder::default();
    let payload = serde_json::json!({
        "status": "completed",
        "input": "{\"command\":\"pwd\"}",
        "output": "/workspace"
    });

    let event = agent_session::processor::StreamEvent::Tool(agent_session::processor::ToolEvent {
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        kind: agent_session::processor::ToolEventKind::Complete,
        status: agent_session::processor::ToolCallStatus::Completed,
        occurred_ms: 123,
        payload: payload.clone(),
    });
    let stream_payload = stream_payload_for_non_done_event(
        event,
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "",
        None,
        "/unused",
    )
    .expect("tool payload");
    let legacy_delta = format!("tool shell\n{payload}\n");

    assert_eq!(stream_payload, serde_json::json!({"type": "chat.delta", "delta": legacy_delta}));
    assert_eq!(assistant_text, legacy_delta);
}

#[test]
fn gateway_tool_event_preserves_step_and_payload() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut recorder = StreamPatchRecorder::default();

    let _ = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::StepStart {
            step_index: 6,
            created_ms: 10,
            model: None,
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        None,
        "/unused",
    );

    let start_payload = serde_json::json!({
        "status": "running",
        "input": "{\"command\":\"pwd\"}"
    });
    let start = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::Tool(agent_session::processor::ToolEvent {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            kind: agent_session::processor::ToolEventKind::Start,
            status: agent_session::processor::ToolCallStatus::Running,
            occurred_ms: 100,
            payload: start_payload.clone(),
        }),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        None,
        "/unused",
    )
    .expect("tool start payload");

    let complete_payload = serde_json::json!({
        "status": "completed",
        "input": "{\"command\":\"pwd\"}",
        "output": "/workspace"
    });
    let complete = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::Tool(agent_session::processor::ToolEvent {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            kind: agent_session::processor::ToolEventKind::Complete,
            status: agent_session::processor::ToolCallStatus::Completed,
            occurred_ms: 123,
            payload: complete_payload.clone(),
        }),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        None,
        "/unused",
    )
    .expect("tool complete payload");

    assert_eq!(start["type"], "chat.tool");
    assert_eq!(start["step_index"], 6);
    assert_eq!(start["call_id"], "call-1");
    assert_eq!(start["kind"], "start");
    assert_eq!(start["occurred_ms"], 100);
    assert_eq!(start["payload"], start_payload);

    assert_eq!(complete["type"], "chat.tool");
    assert_eq!(complete["step_index"], start["step_index"]);
    assert_eq!(complete["call_id"], start["call_id"]);
    assert_eq!(complete["name"], "shell");
    assert_eq!(complete["kind"], "complete");
    assert_eq!(complete["status"], "completed");
    assert!(complete["occurred_ms"].as_u64() >= start["occurred_ms"].as_u64());
    assert_eq!(complete["payload"], complete_payload);
    assert_eq!(
        assistant_text,
        format!("tool shell\n{start_payload}\ntool shell\n{complete_payload}\n")
    );
}

#[test]
fn stream_payload_for_non_done_event_handles_step_start_post_tool_round_and_done() {
    let root = tempfile::tempdir().expect("temporary root");
    let ids = StreamTurnMessageIds::new("assistant-id", "user-id");
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut recorder = StreamPatchRecorder::default();

    let start = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::StepStart {
            step_index: 3,
            created_ms: 11,
            model: Some("provider/model".to_string()),
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        Some(&ids),
        &root.path().to_string_lossy(),
    )
    .expect("start payload");

    assert_eq!(start["type"], "chat.step_start");
    assert_eq!(start["step_index"], 3);
    assert_eq!(start["created_ms"], 11);
    assert_eq!(start["model"], "provider/model");
    assert_eq!(recorder.active_step.as_ref().map(|step| step.step_index), Some(3));
    assert_eq!(recorder.parts.len(), 1);

    let post_tool = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::PostToolRound { step_index: 3 },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        Some(&ids),
        &root.path().to_string_lossy(),
    )
    .expect("post tool payload");
    assert_eq!(post_tool, serde_json::json!({"type": "chat.post_tool_round", "step_index": 3}));

    let done = stream_payload_for_non_done_event(
        agent_session::processor::StreamEvent::Done(ui_models::TokenUsage::default()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut recorder,
        "session-id",
        Some(&ids),
        &root.path().to_string_lossy(),
    );
    assert!(done.is_none());
}

// 请求选项按显式参数、代理默认值和后端能力合并，显式选择始终拥有最高优先级。
#[test]
fn merge_gateway_request_options_forwards_allowed_tools_to_acp() {
    let options = merge_gateway_request_options(
        None,
        None,
        Some(vec!["file_read".to_string()]),
        None,
        None,
        None,
        Some("codex".to_string()),
        None,
        None,
    );

    assert_eq!(options.get("allowed_tools").and_then(|value| value.as_array()).unwrap().len(), 1);
    assert_eq!(
        options.get("acp_allowed_tools").and_then(|value| value.as_array()).unwrap().len(),
        1
    );
}

#[test]
fn delegated_gateway_options_preserve_tool_registration_and_pass_skill_policy() {
    let options = merge_delegate_capability_policy_options(
        merge_gateway_request_options(
            None,
            Some("reviewer".to_string()),
            Some(vec!["file_read".to_string()]),
            None,
            None,
            None,
            Some("codex".to_string()),
            None,
            None,
        ),
        Some(vec!["review".to_string()]),
        Some(vw_config_types::agent::DelegateCapabilityPolicy::Strict),
        Some(vec!["rustcodegraph".to_string()]),
    );

    assert_eq!(options["allowed_tools"], serde_json::json!(["file_read"]));
    assert!(options.get("delegate_tool_policy").is_none());
    assert_eq!(options["delegate_default_allowed_skills"], serde_json::json!(["review"]));
    assert_eq!(options["delegate_skill_policy"], "strict");
    assert_eq!(
        options["delegate_trusted_user_selected_skills"],
        serde_json::json!(["rustcodegraph"]),
    );
}

#[test]
fn merge_gateway_request_options_preserves_explicit_options_over_agent_defaults() {
    let options = merge_gateway_request_options(
        Some(serde_json::json!({
            "chat_system_prompt": "explicit system",
            "temperature": 0.8,
            "top_p": 0.9
        })),
        Some("agent-a".to_string()),
        Some(vec![" shell ".to_string()]),
        Some("agent system".to_string()),
        Some(0.1),
        Some(0.2),
        None,
        Some(vec!["file_read".to_string()]),
        None,
    );

    assert_eq!(options.get("agent").and_then(Value::as_str), Some("agent-a"));
    assert_eq!(options.get("chat_system_prompt").and_then(Value::as_str), Some("explicit system"));
    assert_eq!(options.get("temperature").and_then(Value::as_f64), Some(0.8));
    assert_eq!(options.get("top_p").and_then(Value::as_f64), Some(0.9));
    assert_eq!(
        options
            .get("allowed_tools")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(" shell ")
    );
    assert_eq!(
        options
            .get("acp_allowed_tools")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some("file_read")
    );
    assert_eq!(options.get("acp_test").and_then(Value::as_bool), Some(true));
}

#[test]
fn merge_gateway_request_options_replaces_non_object_options_with_object() {
    let options = merge_gateway_request_options(
        Some(serde_json::json!("not an object")),
        None,
        None,
        Some("system".to_string()),
        Some(0.3),
        Some(0.7),
        None,
        None,
        None,
    );

    assert_eq!(options.get("chat_system_prompt").and_then(Value::as_str), Some("system"));
    assert_eq!(options.get("temperature").and_then(Value::as_f64), Some(0.3));
    assert_eq!(options.get("top_p").and_then(Value::as_f64), Some(0.7));
}

#[test]
fn chat_reasoning_effort_reaches_acp_options_without_leaking_to_non_acp() {
    use vw_api_types::ReasoningEffortDto;

    for (effort, expected) in [
        (ReasoningEffortDto::High, "high"),
        (ReasoningEffortDto::Xhigh, "xhigh"),
        (ReasoningEffortDto::Max, "max"),
    ] {
        let acp_options = merge_gateway_request_options(
            None,
            None,
            None,
            None,
            None,
            None,
            Some("codex".to_string()),
            None,
            Some(effort),
        );
        assert_eq!(acp_options.get("reasoning_effort").and_then(Value::as_str), Some(expected));

        let non_acp_options = merge_gateway_request_options(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(effort),
        );
        assert!(non_acp_options.get("reasoning_effort").is_none());
    }

    let default_options = merge_gateway_request_options(
        None,
        None,
        None,
        None,
        None,
        None,
        Some("codex".to_string()),
        None,
        None,
    );
    assert!(default_options.get("reasoning_effort").is_none());
}

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
        agent_session::message::Part::StepStart(part) => {
            assert_eq!(part.base.id, "prt-start");
            assert_eq!(part.snapshot.as_deref(), Some("snapshot-before"));
        }
        _ => panic!("expected step start part"),
    }

    match step_finish {
        agent_session::message::Part::StepFinish(part) => {
            assert_eq!(part.base.id, "prt-finish");
            assert_eq!(part.reason, "tool_round");
            assert_eq!(part.snapshot.as_deref(), Some("snapshot-after"));
            assert_eq!(part.tokens.total, Some(0));
        }
        _ => panic!("expected step finish part"),
    }

    match patch {
        agent_session::message::Part::Patch(part) => {
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

    assert!(matches!(start, agent_session::message::Part::StepStart(_)));
    assert!(matches!(finish, agent_session::message::Part::StepFinish(_)));
    assert!(matches!(patch, agent_session::message::Part::Patch(_)));
    assert!(!ids.assistant_id.is_empty());
    assert!(!ids.user_id.is_empty());
    assert_ne!(ids.assistant_id, ids.user_id);
}

#[test]
fn token_info_from_usage_maps_gateway_fields() {
    let usage = ui_models::TokenUsage {
        input_tokens: 3,
        output_tokens: 5,
        cached_tokens: 7,
        reasoning_tokens: 11,
    };
    let tokens = token_info_from_usage(&usage);

    assert_eq!(tokens.total, Some(15));
    assert_eq!(tokens.input, 3);
    assert_eq!(tokens.output, 5);
    assert_eq!(tokens.reasoning, 11);
    assert_eq!(tokens.cache.read, 7);
    assert_eq!(tokens.cache.write, 0);
}

// 流收集器只有观察到完成事件才返回聚合输出，连接提前关闭必须视为不完整响应。
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

#[tokio::test]
async fn resolve_delegate_request_overrides_without_agent_returns_explicit_values() {
    let overrides = resolve_delegate_request_overrides(
        Some("  ".to_string()),
        Some(vec![" shell ".to_string(), "shell".to_string(), " ".to_string()]),
        Some("provider/model".to_string()),
    )
    .await
    .expect("overrides");

    assert_eq!(overrides.agent, None);
    assert_eq!(overrides.model.as_deref(), Some("provider/model"));
    assert_eq!(overrides.allowed_tools, Some(vec!["shell".to_string()]));
    assert_eq!(overrides.chat_system_prompt, None);
    assert_eq!(overrides.temperature, None);
    assert_eq!(overrides.top_p, None);
}

#[tokio::test]
async fn resolve_main_request_overrides_preserves_registered_tool_selection() {
    let overrides = resolve_delegate_request_overrides(
        Some("main".to_string()),
        Some(vec!["file_read".to_string(), "memory_store".to_string(), "schedule".to_string()]),
        Some("provider/model".to_string()),
    )
    .await
    .expect("main overrides");

    assert_eq!(overrides.agent.as_deref(), Some("main"));
    assert_eq!(
        overrides.allowed_tools,
        Some(vec!["file_read".to_string(), "memory_store".to_string(), "schedule".to_string(),])
    );
}

#[tokio::test]
async fn resolve_delegate_request_overrides_rejects_unknown_agent() {
    let error = resolve_delegate_request_overrides(
        Some("agent-that-does-not-exist-for-stream-tests".to_string()),
        None,
        None,
    )
    .await
    .expect_err("unknown agent");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("unknown agent:"));
}

#[test]
fn stream_turn_message_ids_new_converts_and_preserves_both_ids() {
    let ids = StreamTurnMessageIds::new(String::from("msg-assistant"), "msg-user");

    assert_eq!(ids.assistant_id, "msg-assistant");
    assert_eq!(ids.user_id, "msg-user");
}

#[test]
fn stream_empty_token_info_zeros_every_usage_field() {
    let tokens = stream_empty_token_info();

    assert_eq!(tokens.total, Some(0));
    assert_eq!(tokens.input, 0);
    assert_eq!(tokens.output, 0);
    assert_eq!(tokens.reasoning, 0);
    assert_eq!(tokens.cache.read, 0);
    assert_eq!(tokens.cache.write, 0);
}

#[test]
fn tool_content_to_session_text_extracts_json_output_without_tool_call_id() {
    let text = tool_content_to_session_text(r#"{"content":"result only"}"#);

    assert_eq!(text, "[Tool results]\n<tool_result>\nresult only\n</tool_result>");
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
    use vw_api_types::ReasoningEffortDto;

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
    assert!(matches!(parts.first(), Some(agent_session::message::Part::StepStart(_))));
}

#[test]
fn stream_patch_recorder_mismatched_step_discards_active_step_without_snapshot_io() {
    let mut recorder = StreamPatchRecorder {
        current_step_index: Some(2),
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
    let mut recorder =
        StreamPatchRecorder { current_step_index: None, active_step: None, parts: vec![part] };

    let first = recorder.take_parts();
    let second = recorder.take_parts();

    assert_eq!(first.len(), 1);
    assert!(matches!(first.first(), Some(agent_session::message::Part::Patch(_))));
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

fn docs_stream_request(session_id: Option<&str>) -> GatewayChatStreamRequest {
    GatewayChatStreamRequest {
        session_id: session_id.map(vw_api_types::id::SessionId::from),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "请列出 docs 文件列表"
        })],
        model: Some("test-provider/test-model".to_string()),
        ..GatewayChatStreamRequest::default()
    }
}

fn make_docs_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let docs = temp.path().join("docs");
    std::fs::create_dir(&docs).expect("docs directory");
    std::fs::write(docs.join("guide.md"), "# Guide\n").expect("docs file");
    temp
}

#[tokio::test]
async fn collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids() {
    let temp = make_docs_workspace();
    let session_id = format!("ses_stream_docs_{}", uuid::Uuid::new_v4());
    let request = docs_stream_request(Some(&session_id));
    let mut seen_types = Vec::new();

    let collected = collect_gateway_chat_stream_with_observer(
        temp.path().to_string_lossy().to_string(),
        request,
        |payload| {
            if let Some(kind) = payload.get("type").and_then(Value::as_str) {
                seen_types.push(kind.to_string());
            }
        },
    )
    .await
    .expect("docs stream should complete");

    assert!(collected.output.contains("docs 目录下的文件"));
    assert!(collected.output.contains("docs/guide.md"));
    assert!(collected.message_id.as_deref().is_some_and(|id| id.starts_with("msg_")));
    assert!(collected.parent_message_id.as_deref().is_some_and(|id| id.starts_with("msg_")));
    assert_eq!(seen_types, vec!["chat.delta", "chat.done"]);

    let stored = agent_session::message::messages(&session_id, None)
        .await
        .expect("persisted messages should load");
    assert_eq!(stored.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_turn_registers_parent_before_thread_tool_use() {
    let temp = make_docs_workspace();
    let directory = temp.path().to_string_lossy().to_string();
    let session_id = format!("ses_stream_parent_{}", uuid::Uuid::new_v4());
    let request = docs_stream_request(Some(&session_id));
    let tenant = crate::session::ui_store::current_tenant_scope();
    let scope = crate::session::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .expect("project scope");

    assert!(
        crate::session::ui_store::load_agent_session_scoped_for_tenant(
            &tenant,
            &session_id,
            Some(&scope),
        )
        .is_none()
    );

    let tenant_for_stream = tenant.clone();
    let directory_for_stream = directory.clone();
    let receiver = with_tenant_instance(tenant.clone(), directory.clone(), move || {
        Box::pin(async move {
            start_gateway_chat_stream(tenant_for_stream, directory_for_stream, request).await
        })
    })
    .await
    .expect("first gateway stream should start");

    let parent = crate::session::ui_store::load_agent_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .expect("parent session must exist before streamed tool activity");
    assert_eq!(parent.project_id, scope);
    assert_eq!(parent.directory, directory);
    assert_eq!(parent.parent_id, None);
    drop(receiver);
}

// 轨迹提交一次性持久化步骤、工具和原始产物，不依赖后续 UI 快照保存。
#[test]
fn stream_trajectory_commit_persists_steps_tools_and_raw_without_ui_save() {
    let project = tempfile::tempdir().expect("project directory");
    let project_path = project.path().to_string_lossy().to_string();
    let scope = crate::session::ui_store::resolve_session_scope_id(Some(&project_path), None)
        .expect("project scope");
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = format!("stream-raw-{}", uuid::Uuid::new_v4());
    let ids = StreamTurnMessageIds::new("turn-stream", "user-stream");
    let raw = serde_json::json!({
        "session_id": session_id.clone(),
        "step_index": 1,
        "model": "provider/model",
        "system": ["system"],
        "messages": [{"role": "user", "content": "inspect"}],
        "message_sources": [null],
        "request": {"config": {"model": "provider/model"}, "tools": [{"name": "shell"}]},
        "output": {"text": "done"}
    });
    let artifact =
        crate::session::llm_raw::LlmRawArtifact::from_full_raw_rounds(&ids.assistant_id, &[raw])
            .expect("raw artifact");
    raw_commit::register(&tenant, Some(&scope), artifact).expect("register raw artifact");

    let mut recorder = StreamTrajectoryRecorder::new(ids.assistant_id.clone());
    recorder.record(&agent_session::processor::StreamEvent::StepStart {
        step_index: 1,
        created_ms: 10,
        model: Some("provider/model".to_string()),
    });
    recorder.record(&agent_session::processor::StreamEvent::Tool(
        agent_session::processor::ToolEvent {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            kind: agent_session::processor::ToolEventKind::Complete,
            status: agent_session::processor::ToolCallStatus::Completed,
            occurred_ms: 15,
            payload: serde_json::json!({"status": "completed", "output": "ok"}),
        },
    ));
    recorder.record(&agent_session::processor::StreamEvent::StepFinish {
        step_index: 1,
        finished_ms: 20,
        usage: ui_models::TokenUsage::default(),
        finish_reason: Some("stop".to_string()),
        model: Some("provider/model".to_string()),
    });

    commit_stream_session_trajectory(
        &tenant,
        Some(&scope),
        &session_id,
        &project_path,
        "inspect",
        "done",
        Some("provider/model"),
        &ids,
        &recorder,
        5,
        25,
    )
    .expect("stream endpoint should commit trajectory");

    let stored = crate::session::ui_store::load_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .expect("stored session");
    assert_eq!(stored.steps.len(), 1);
    assert_eq!(stored.steps[0].turn_id.as_deref(), Some("turn-stream"));
    assert_eq!(stored.steps[0].tool_timings[0].call_id, "call-1");
    let stored_info = crate::session::ui_store::load_agent_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .expect("stored session metadata");
    assert_eq!(stored_info.directory, project_path);
    assert_eq!(stored_info.title, "inspect");
    let stored_raw = crate::session::ui_store::load_llm_raw_turn_step_scoped_for_tenant(
        &tenant,
        &session_id,
        "turn-stream",
        1,
        Some(&scope),
    )
    .expect("raw read")
    .expect("raw value");
    assert_eq!(stored_raw["output"]["text"], "done");
}

#[tokio::test]
async fn stream_processing_elapsed_persists_and_reloads_explicit_request_bounds() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let session_id = format!("ses_stream_timing_{}", uuid::Uuid::new_v4());

    let stored = crate::project::instance::provide(temp.path(), None, move || {
        Box::pin(async move {
            let usage = ui_models::TokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 0,
                reasoning_tokens: 0,
            };
            let (assistant_id, _) = persist_stream_chat_turn_for_agent(
                &session_id,
                "question",
                "answer",
                Some("provider/model"),
                &usage,
                Some("stop"),
                None,
                &[],
                "gateway",
                None,
                Some(agent_session::message::AssistantTime {
                    created: 1_000,
                    completed: Some(1_250),
                    processing_elapsed_ms: Some(200),
                    llm_elapsed_ms: Some(125),
                }),
            )
            .await
            .expect("timed stream turn should persist");
            agent_session::message::get(&session_id, &assistant_id).await
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("assistant message should reload");

    let agent_session::message::Info::Assistant(info) = stored.info else {
        panic!("stored message should be assistant");
    };
    assert_eq!(info.time.created, 1_000);
    assert_eq!(info.time.completed, Some(1_250));
    assert_eq!(info.time.processing_elapsed_ms, Some(200));
    assert_eq!(info.time.llm_elapsed_ms, Some(125));
}

// 用户消息时间取请求开始时刻，避免长时间生成导致会话排序被响应完成时间扭曲。
#[tokio::test]
async fn persist_stream_turn_uses_request_start_for_user_timestamp() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let session_id = format!("ses_stream_user_time_{}", uuid::Uuid::new_v4());

    let stored = crate::project::instance::provide(temp.path(), None, move || {
        Box::pin(async move {
            let usage = ui_models::TokenUsage::default();
            persist_stream_chat_turn_for_agent(
                &session_id,
                "question",
                "answer",
                Some("provider/model"),
                &usage,
                Some("stop"),
                None,
                &[],
                "gateway",
                None,
                Some(agent_session::message::AssistantTime {
                    created: 1_000,
                    completed: Some(9_000),
                    processing_elapsed_ms: Some(7_500),
                    llm_elapsed_ms: Some(500),
                }),
            )
            .await
            .expect("stream turn should persist");
            agent_session::message::messages(&session_id, None).await
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("persisted messages should load");

    let user = stored
        .iter()
        .find(|message| matches!(message.info, agent_session::message::Info::User(_)))
        .expect("user message should exist");
    let agent_session::message::Info::User(info) = &user.info else {
        unreachable!("matched user info");
    };
    assert_eq!(info.time.created, 1_000);
    let agent_session::message::Part::Text(part) = &user.parts[0] else {
        panic!("user text part should exist");
    };
    assert_eq!(part.time.as_ref().map(|time| time.start), Some(1_000));
    assert_eq!(part.time.as_ref().and_then(|time| time.end), Some(1_000));
}

#[tokio::test]
async fn chat_stream_returns_sse_response_for_docs_request() {
    let temp = make_docs_workspace();
    let request = docs_stream_request(None);
    let request_id = format!("chat-resume-{}", uuid::Uuid::new_v4());
    let mut headers = HeaderMap::new();
    headers.insert(
        crate::app::agent::gateway::api::handlers::resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );
    let response = chat_stream(
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) }),
        headers,
        Json(request.clone()),
    )
    .await
    .expect("chat stream response")
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).and_then(|value| value.to_str().ok()),
        Some("no-cache")
    );
    assert_eq!(
        response.headers().get(header::CONNECTION).and_then(|value| value.to_str().ok()),
        Some("keep-alive")
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("sse body");
    let body = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(body.contains(r#""type":"chat.delta""#), "{body}");
    assert!(body.contains(r#""type":"chat.done""#), "{body}");
    assert!(body.contains("docs/guide.md"));
    let event_ids = body
        .lines()
        .filter_map(|line| line.strip_prefix("id:"))
        .map(str::trim)
        .map(|id| id.parse::<u64>().expect("numeric SSE id"))
        .collect::<Vec<_>>();
    assert_eq!(event_ids, vec![1, 2]);

    let mut resume_headers = HeaderMap::new();
    resume_headers.insert(
        crate::app::agent::gateway::api::handlers::resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );
    resume_headers.insert("last-event-id", "1".parse().expect("valid cursor header"));
    let resumed = chat_stream(
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) }),
        resume_headers,
        Json(request),
    )
    .await
    .expect("resumed chat stream response")
    .into_response();
    let resumed_body =
        axum::body::to_bytes(resumed.into_body(), usize::MAX).await.expect("resumed SSE body");
    let resumed_body = String::from_utf8(resumed_body.to_vec()).expect("utf8 resumed body");
    assert!(!resumed_body.contains(r#""type":"chat.delta""#));
    assert!(resumed_body.contains("id: 2") || resumed_body.contains("id:2"));
    assert!(resumed_body.contains(r#""type":"chat.done""#));
}
