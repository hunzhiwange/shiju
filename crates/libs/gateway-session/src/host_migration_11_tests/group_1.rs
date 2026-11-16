use super::*;

#[test]
fn codex_skills_defaults_gateway_empty_defaults_replace_untrusted_request_allowance() {
    let options = merge_delegate_capability_policy_options(
        serde_json::json!({"delegate_default_allowed_skills":["unconfigured"],"delegate_skill_policy":"unrestricted","allowed_tools":["file_read"]}),
        None,
        Some(config_agent::DelegateCapabilityPolicy::Strict),
        None,
    );
    assert_eq!(options["delegate_default_allowed_skills"], serde_json::json!([]));
    assert_eq!(options["delegate_skill_policy"], "strict");
    assert_eq!(options["allowed_tools"], serde_json::json!(["file_read"]));
}

#[test]
fn stream_payload_for_non_done_event_encodes_delta_step_finish_and_error_state() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = Some("provider/old".to_string());
    let mut current_step_index = None;

    let context = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::ContextLoaded {
            paths: vec!["/workspace/AGENTS.md".to_string()],
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "",
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
        session_engine::processor_events::StreamEvent::Delta("hello".to_string()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "",
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
        session_engine::processor_events::StreamEvent::StepFinish {
            step_index: 7,
            finished_ms: 99,
            usage: usage.clone(),
            finish_reason: Some("stop".to_string()),
            model: Some("provider/new".to_string()),
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "",
    )
    .expect("finish payload");
    assert_eq!(finish["type"], "chat.step_finish");
    assert_eq!(finish["step_index"], 7);
    assert_eq!(finish["finished_ms"], 99);
    assert_eq!(finish["usage"], serde_json::to_value(&usage).unwrap());
    assert_eq!(finish_reason.as_deref(), Some("stop"));
    assert_eq!(assistant_model.as_deref(), Some("provider/new"));

    let error = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::Error("failed".to_string()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "",
    )
    .expect("error payload");
    assert_eq!(error, serde_json::json!({"type": "chat.error", "error": "failed"}));
}

#[test]
fn compaction_lifecycle_events_encode_stable_gateway_payloads() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut current_step_index = None;
    let mut encode = |phase| {
        stream_payload_for_non_done_event(
            session_engine::processor_events::StreamEvent::Compaction { phase },
            &mut assistant_text,
            &mut finish_reason,
            &mut assistant_model,
            &mut current_step_index,
            "session-id",
        )
        .expect("compaction payload")
    };

    assert_eq!(
        encode(session_engine::processor_events::CompactionPhase::Started),
        serde_json::json!({"type": "chat.compaction", "phase": "started"})
    );
    assert_eq!(
        encode(session_engine::processor_events::CompactionPhase::Completed),
        serde_json::json!({"type": "chat.compaction", "phase": "completed"})
    );
    assert!(assistant_text.is_empty());
}

#[test]
fn gateway_stream_preserves_existing_tool_payloads() {
    let mut assistant_text = "Checking the repo.".to_string();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut current_step_index = None;
    let payload = serde_json::json!({
        "status": "completed",
        "input": "{\"command\":\"pwd\"}",
        "output": "/workspace"
    });

    let event = session_engine::processor_events::StreamEvent::Tool(session_engine::processor_events::ToolEvent {
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        kind: session_engine::processor_events::ToolEventKind::Complete,
        status: session_engine::processor_events::ToolCallStatus::Completed,
        occurred_ms: 123,
        payload: payload.clone(),
    });
    let stream_payload = stream_payload_for_non_done_event(
        event,
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "",
    )
    .expect("tool payload");
    let legacy_delta = format!("\ntool shell\n{payload}\n");

    assert_eq!(stream_payload, serde_json::json!({"type": "chat.delta", "delta": legacy_delta}));
    assert_eq!(assistant_text, format!("Checking the repo.{legacy_delta}"));
}

#[test]
fn gateway_tool_event_preserves_step_and_payload() {
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut current_step_index = None;

    let _ = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::StepStart {
            step_index: 6,
            created_ms: 10,
            model: None,
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
    );

    let start_payload = serde_json::json!({
        "status": "running",
        "input": "{\"command\":\"pwd\"}"
    });
    let start = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::Tool(session_engine::processor_events::ToolEvent {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            kind: session_engine::processor_events::ToolEventKind::Start,
            status: session_engine::processor_events::ToolCallStatus::Running,
            occurred_ms: 100,
            payload: start_payload.clone(),
        }),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
    )
    .expect("tool start payload");

    let complete_payload = serde_json::json!({
        "status": "completed",
        "input": "{\"command\":\"pwd\"}",
        "output": "/workspace"
    });
    let complete = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::Tool(session_engine::processor_events::ToolEvent {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            kind: session_engine::processor_events::ToolEventKind::Complete,
            status: session_engine::processor_events::ToolCallStatus::Completed,
            occurred_ms: 123,
            payload: complete_payload.clone(),
        }),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
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
    let mut assistant_text = String::new();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut current_step_index = None;

    let start = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::StepStart {
            step_index: 3,
            created_ms: 11,
            model: Some("provider/model".to_string()),
        },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
    )
    .expect("start payload");

    assert_eq!(start["type"], "chat.step_start");
    assert_eq!(start["step_index"], 3);
    assert_eq!(start["created_ms"], 11);
    assert_eq!(start["model"], "provider/model");
    assert_eq!(current_step_index, Some(3));

    let post_tool = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::PostToolRound { step_index: 3 },
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
    )
    .expect("post tool payload");
    assert_eq!(post_tool, serde_json::json!({"type": "chat.post_tool_round", "step_index": 3}));

    let done = stream_payload_for_non_done_event(
        session_engine::processor_events::StreamEvent::Done(ui_models::TokenUsage::default()),
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
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
        Some(config_agent::DelegateCapabilityPolicy::Strict),
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
fn chat_reasoning_effort_reaches_acp_and_model_options() {
    use vw_api_chat_identity::ReasoningEffortDto;

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
        assert_eq!(non_acp_options.get("reasoning_effort").and_then(Value::as_str), Some(expected));
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
