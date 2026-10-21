//! 会话 UI 数据类型的兼容反序列化与往返测试。
//!
//! 覆盖旧数据默认值、空集合省略、模型字段及处理和工具调用耗时信息。

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
    assert_eq!(legacy.turn_id, None);

    let step = super::ChatSessionStep {
        index: 2,
        turn_id: Some("assistant-message-2".to_string()),
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

    assert_eq!(reloaded.turn_id, step.turn_id);
    assert_eq!(reloaded.tool_timings, step.tool_timings);
}

#[test]
fn session_message_page_serializes_bounded_records() {
    let page = super::SessionMessagePage {
        records: vec![super::SessionMessageRecord {
            seq_no: 7,
            message_id: Some("message-7".to_owned()),
            message: super::ChatMessage {
                role: super::ChatRole::Assistant,
                content: "bounded".to_owned(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        }],
        has_more: true,
    };

    let value = serde_json::to_value(&page).unwrap();
    assert_eq!(value["records"][0]["seq_no"], 7);
    assert_eq!(value["records"][0]["message_id"], "message-7");
    assert_eq!(value["has_more"], true);
    assert_eq!(serde_json::from_value::<super::SessionMessagePage>(value).unwrap(), page);
}

#[test]
fn session_message_delta_round_trips_version_and_truncate_boundary() {
    let delta = super::SessionMessageDelta {
        session_id: "session-1".to_owned(),
        expected_updated_ms: 10,
        updated_ms: 11,
        upserts: Vec::new(),
        truncate_after: Some(4),
    };

    let value = serde_json::to_value(&delta).unwrap();
    assert_eq!(value["expected_updated_ms"], 10);
    assert_eq!(value["truncate_after"], 4);
    assert_eq!(serde_json::from_value::<super::SessionMessageDelta>(value).unwrap(), delta);
}
