use crate::stream::*;
use session_ui_types as ui_models;

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
fn split_stream_model_ref_uses_model_id_when_provider_is_missing() {
    let model_ref = split_stream_model_ref(Some("claude-sonnet-4"));

    assert_eq!(model_ref.provider_id, "");
    assert_eq!(model_ref.model_id, "claude-sonnet-4");
}
