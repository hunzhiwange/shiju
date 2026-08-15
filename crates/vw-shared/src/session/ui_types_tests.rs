#[test]
fn token_usage_defaults_reasoning_tokens() {
    let usage: super::TokenUsage =
        serde_json::from_str(r#"{"input_tokens":1,"output_tokens":2,"cached_tokens":3}"#).unwrap();

    assert_eq!(usage.reasoning_tokens, 0);
}

#[test]
fn chat_message_skips_empty_think_timing() {
    let message = super::ChatMessage {
        role: super::ChatRole::Assistant,
        content: "done".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };

    let value = serde_json::to_value(message).unwrap();

    assert_eq!(value["role"], "Assistant");
    assert!(value.get("model").is_none());
    assert!(value.get("think_timing").is_none());
    assert!(value.get("processing_timing").is_none());
}

#[test]
fn chat_message_model_is_optional_and_round_trips() {
    let legacy: super::ChatMessage =
        serde_json::from_str(r#"{"role":"Assistant","content":"old"}"#).unwrap();
    assert_eq!(legacy.model, None);

    let message = super::ChatMessage {
        role: super::ChatRole::Assistant,
        content: "new".to_string(),
        model: Some("provider/model-a".to_string()),
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(value["model"], "provider/model-a");
    assert_eq!(serde_json::from_value::<super::ChatMessage>(value).unwrap(), message);
}

#[test]
fn chat_message_processing_timing_is_optional_and_round_trips() {
    let legacy: super::ChatMessage =
        serde_json::from_str(r#"{"role":"Assistant","content":"old"}"#).unwrap();
    assert_eq!(legacy.processing_timing, None);

    let message = super::ChatMessage {
        role: super::ChatRole::Assistant,
        content: "new".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: Some(super::ProcessingTiming {
            started_ms: 100,
            completed_ms: Some(250),
            processing_elapsed_ms: Some(140),
            llm_elapsed_ms: Some(125),
        }),
    };
    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(value["processing_timing"]["started_ms"], 100);
    assert_eq!(value["processing_timing"]["completed_ms"], 250);
    assert_eq!(value["processing_timing"]["processing_elapsed_ms"], 140);
    assert_eq!(value["processing_timing"]["llm_elapsed_ms"], 125);
    assert_eq!(serde_json::from_value::<super::ChatMessage>(value).unwrap(), message);

    let legacy_timing: super::ProcessingTiming =
        serde_json::from_str(r#"{"started_ms":100,"completed_ms":250}"#).unwrap();
    assert_eq!(legacy_timing.processing_elapsed_ms, None);
    assert_eq!(legacy_timing.llm_elapsed_ms, None);
}

#[test]
fn chat_session_defaults_collection_fields() {
    let session: super::ChatSession =
        serde_json::from_str(r#"{"id":"s1","title":"Session"}"#).unwrap();

    assert!(session.messages.is_empty());
    assert!(session.calls.is_empty());
    assert_eq!(session.created_ms, 0);
}

#[test]
fn chat_session_step_round_trips_tool_timings() {
    let legacy: super::ChatSessionStep = serde_json::from_str(
        r#"{"index":1,"started_ms":100,"usage":{"input_tokens":0,"output_tokens":0,"cached_tokens":0}}"#,
    )
    .unwrap();
    assert!(legacy.tool_timings.is_empty());

    let step = super::ChatSessionStep {
        index: 2,
        started_ms: 200,
        finished_ms: Some(400),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: super::TokenUsage::default(),
        cost_usd: None,
        finish_reason: Some("tool_calls".to_string()),
        model: Some("provider/model".to_string()),
        tool_timings: vec![super::ToolTiming {
            call_id: "call-1".to_string(),
            name: "shell".to_string(),
            started_ms: Some(225),
            finished_ms: Some(375),
        }],
    };

    let value = serde_json::to_value(&step).unwrap();
    let reloaded: super::ChatSessionStep = serde_json::from_value(value).unwrap();

    assert_eq!(reloaded.tool_timings, step.tool_timings);
}
