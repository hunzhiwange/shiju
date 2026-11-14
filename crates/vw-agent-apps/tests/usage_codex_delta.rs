use serde_json::{Value, json};
use vw_agent_apps::usage::codex_delta::{CodexDeltaParser, normalize_codex_model, parse_reader};
use vw_api_types::usage::{UsageInputSemantics, UsageReasoningSemantics};

fn snapshot(input: u64, output: u64) -> Value {
    json!({"type":"event_msg", "payload":{"type":"token_count", "info":{
        "total_token_usage":{"input_tokens":input,"output_tokens":output}
    }}})
}

#[test]
fn usage17_codex_delta_cumulative_snapshot_produces_only_new_delta() {
    let mut parser = CodexDeltaParser::new();
    let first = parser.observe(&snapshot(100, 20)).unwrap();
    assert_eq!(first.usage.tokens.input, Some(100));
    assert!(parser.observe(&snapshot(100, 20)).is_none());
    let second = parser.observe(&snapshot(130, 25)).unwrap();
    assert_eq!(second.usage.tokens.input, Some(30));
    assert_eq!(second.usage.tokens.output, Some(5));
}

#[test]
fn usage17_codex_delta_regression_keeps_high_water() {
    let mut parser = CodexDeltaParser::new();
    parser.observe(&snapshot(100, 20)).unwrap();
    assert!(parser.observe(&snapshot(60, 10)).is_none());
    let delta = parser.observe(&snapshot(110, 22)).unwrap();
    assert_eq!(delta.usage.tokens.input, Some(10));
    assert_eq!(delta.usage.tokens.output, Some(2));
}

#[test]
fn usage17_codex_delta_exact_last_preserves_subset_semantics() {
    let mut parser = CodexDeltaParser::new();
    parser.observe(&snapshot(1000, 100)).unwrap();
    let mut event = snapshot(50, 5);
    event["payload"]["info"]["last_token_usage"] = json!({
        "input_tokens":30,"cached_input_tokens":20,"cache_write_input_tokens":15,
        "output_tokens":10,"reasoning_output_tokens":6
    });
    let delta = parser.observe(&event).unwrap();
    let tokens = delta.usage.tokens;
    assert_eq!(tokens.input, Some(30));
    assert_eq!(tokens.cache_read, Some(20));
    assert_eq!(tokens.cache_creation, Some(10));
    assert_eq!(tokens.output, Some(10));
    assert_eq!(tokens.reasoning, Some(6));
    assert_eq!(tokens.input_semantics, UsageInputSemantics::Total);
    assert_eq!(tokens.reasoning_semantics, UsageReasoningSemantics::IncludedInOutput);
    assert!(parser.observe(&event).is_none());
}

#[test]
fn usage17_codex_delta_interleaved_lane_replay_is_ignored() {
    let mut parser = CodexDeltaParser::new();
    let mut a = snapshot(100, 20);
    a["payload"]["rate_limits"] = json!({"limit_id":"a"});
    a["payload"]["info"]["last_token_usage"] = json!({"input_tokens":5});
    let mut b = snapshot(80, 10);
    b["payload"]["rate_limits"] = json!({"limit_id":"b"});
    b["payload"]["info"]["last_token_usage"] = json!({"input_tokens":7});
    assert_eq!(parser.observe(&a).unwrap().usage.tokens.input, Some(5));
    assert_eq!(parser.observe(&b).unwrap().usage.tokens.input, Some(7));
    assert!(parser.observe(&a).is_none());
}

#[test]
fn usage17_codex_delta_last_only_equal_requests_are_distinct() {
    let mut parser = CodexDeltaParser::new();
    let event = json!({"type":"event_msg","payload":{"type":"token_count","info":{
        "last_token_usage":{"input_tokens":5,"output_tokens":2}
    }}});
    assert!(parser.observe(&event).is_some());
    assert!(parser.observe(&event).is_some());
}

#[test]
fn usage17_codex_delta_reader_preserves_unknown_model_and_skips_zero() {
    let lines = ["not json".to_owned(), snapshot(0, 0).to_string(), snapshot(10, 2).to_string()];
    let deltas = parse_reader(std::io::Cursor::new(lines.join("\n"))).unwrap();
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].line_number, 3);
    assert_eq!(deltas[0].request_model, None);
    assert_eq!(deltas[0].pricing_model, None);
    assert_eq!(deltas[0].usage.tokens.cache_read, None);
}

#[test]
fn usage17_codex_delta_model_changes_preserve_cumulative_baseline() {
    let mut parser = CodexDeltaParser::new();
    parser.observe(&snapshot(10, 2)).unwrap();
    parser.observe(&json!({"type":"event_msg","payload":{
        "type":"thread_settings_applied","thread_settings":{"model":"OpenAI/GPT-5.4-2026-03-05"}
    }}));
    let delta = parser.observe(&snapshot(15, 3)).unwrap();
    assert_eq!(delta.request_model.as_deref(), Some("OpenAI/GPT-5.4-2026-03-05"));
    assert_eq!(delta.pricing_model.as_deref(), Some("gpt-5.4"));
    assert_eq!(delta.usage.tokens.input, Some(5));
    assert_eq!(normalize_codex_model("azure/GPT-5.4-20260305"), "gpt-5.4");
    assert_eq!(normalize_codex_model("中文模型"), "中文模型");
}
