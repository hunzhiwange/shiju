use serde_json::json;
use vw_agent_apps::usage::tokens::{UsageProtocol, parse_response, parse_stream};
use vw_api_types::usage::{UsageInputSemantics, UsageReasoningSemantics};

#[test]
fn usage17_tokens_protocol_variants_preserve_billable_tokens() {
    for (protocol, body, input, output) in [
        (
            UsageProtocol::Claude,
            json!({"id":"msg_1","usage":{"input_tokens":200,"output_tokens":100,"cache_read_input_tokens":50,"cache_creation_input_tokens":30,"cache_creation":{"ephemeral_1h_input_tokens":10}}}),
            200,
            100,
        ),
        (
            UsageProtocol::OpenAi,
            json!({"id":"chat_1","usage":{"prompt_tokens":1000,"completion_tokens":100,"prompt_cache_hit_tokens":600}}),
            1000,
            100,
        ),
        (
            UsageProtocol::Responses,
            json!({"id":"resp_1","usage":{"input_tokens":1000,"output_tokens":500,"output_tokens_details":{"reasoning_tokens":200}}}),
            1000,
            500,
        ),
        (
            UsageProtocol::Gemini,
            json!({"responseId":"gem_1","usageMetadata":{"promptTokenCount":8383,"totalTokenCount":8547,"candidatesTokenCount":50,"thoughtsTokenCount":114}}),
            8383,
            164,
        ),
    ] {
        let parsed = parse_response(protocol, &body).unwrap();
        assert_eq!(parsed.tokens.input, Some(input));
        assert_eq!(parsed.tokens.output, Some(output));
        assert!(parsed.response_id.is_some());
        assert!(parsed.has_billable_tokens());
        assert_eq!(parse_stream(protocol, &[body]).unwrap(), parsed);
    }
}

#[test]
fn usage17_tokens_cache_zero_fallback_and_creation_preserved() {
    let parsed = parse_response(
        UsageProtocol::OpenAi,
        &json!({"data":{"usage":{
            "prompt_tokens":6650,"completion_tokens":16,"cache_read_input_tokens":0,
            "prompt_tokens_details":{"cached_tokens":0,"cache_write_tokens":200},
            "prompt_cache_hit_tokens":6400
        }}}),
    )
    .unwrap();
    assert_eq!(parsed.tokens.cache_read, Some(6400));
    assert_eq!(parsed.tokens.cache_creation, Some(200));
    assert_eq!(parsed.tokens.input_semantics, UsageInputSemantics::Total);
}

#[test]
fn usage17_tokens_claude_stream_preserves_cache_subset_and_correlation() {
    let parsed = parse_stream(UsageProtocol::Claude, &[
        json!({"type":"message_start","message":{"id":"msg_1","model":"claude","usage":{"input_tokens":200000,"cache_read_input_tokens":180000,"cache_creation_input_tokens":2000,"cache_creation":{"ephemeral_1h_input_tokens":1000}}}}),
        json!({"type":"message_delta","usage":{"input_tokens":80000,"output_tokens":1000,"cache_read_input_tokens":120000,"cache_creation_input_tokens":500,"cache_creation":{"ephemeral_1h_input_tokens":100}}}),
    ]).unwrap();
    assert_eq!(parsed.tokens.input, Some(80000));
    assert_eq!(parsed.tokens.cache_read, Some(120000));
    assert_eq!(parsed.tokens.cache_creation, Some(500));
    assert_eq!(parsed.tokens.cache_creation_1h, Some(100));
    assert_eq!(parsed.tokens.input_semantics, UsageInputSemantics::Fresh);
    assert_eq!(parsed.response_id.as_deref(), Some("msg_1"));
}

#[test]
fn usage17_tokens_responses_stream_does_not_add_reasoning_twice() {
    let parsed = parse_stream(UsageProtocol::Responses, &[
        json!({"type":"response.created","response":{"id":"resp_1","model":"o3"}}),
        json!({"type":"response.completed","response":{"usage":{"input_tokens":1000,"output_tokens":500,"output_tokens_details":{"reasoning_tokens":300}}}}),
    ]).unwrap();
    assert_eq!(parsed.tokens.output, Some(500));
    assert_eq!(parsed.tokens.reasoning, Some(300));
    assert_eq!(parsed.tokens.reasoning_semantics, UsageReasoningSemantics::IncludedInOutput);
    assert_eq!(parsed.response_id.as_deref(), Some("resp_1"));
}

#[test]
fn usage17_tokens_missing_and_synthetic_zero_are_not_billable() {
    assert!(parse_response(UsageProtocol::OpenAi, &json!({"usage":null})).is_none());
    assert!(parse_response(UsageProtocol::OpenAi, &json!({"usage":{}})).is_none());
    let parsed = parse_response(
        UsageProtocol::OpenAi,
        &json!({"usage":{"prompt_tokens":0,"completion_tokens":0}}),
    )
    .unwrap();
    assert!(!parsed.has_billable_tokens());
    assert_eq!(parsed.tokens.cache_read, None);
}
