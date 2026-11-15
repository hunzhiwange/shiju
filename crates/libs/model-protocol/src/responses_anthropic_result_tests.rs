use super::{responses::ChatTools, responses_anthropic_result::convert};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"id":"msg_fixture","type":"message","role":"assistant","model":"upstream",
        "content":[{"type":"text","text":"before"},
            {"type":"tool_use","id":"call_fixture","name":"lookup","input":{"n":1}},
            {"type":"text","text":"after"}],"stop_reason":"tool_use",
        "usage":{"input_tokens":4,"output_tokens":2,"cache_read_input_tokens":3,
            "cache_creation_input_tokens":1}})
}

#[test]
fn cc_route_058_responses_anthropic_result_expected_behavior() {
    let out = convert(&fixture(), "published", "resp_fixture", &ChatTools::default()).unwrap();
    assert_eq!(out["status"], "completed");
    assert_eq!(out["model"], "published");
    assert_eq!(out["output"][0]["content"][0]["text"], "before");
    assert_eq!(out["output"][1]["call_id"], "call_fixture");
    assert_eq!(out["output"][1]["arguments"], "{\"n\":1}");
    assert_eq!(out["output"][2]["content"][0]["text"], "after");
    assert_eq!(out["usage"]["input_tokens"], 8);
    assert_eq!(out["usage"]["total_tokens"], 10);
    assert_eq!(out["usage"]["input_tokens_details"]["cached_tokens"], 3);
    assert_eq!(out["usage"]["input_tokens_details"]["cache_write_tokens"], 1);
}

#[test]
fn cc_route_058_incomplete_and_refusal_preserve_terminal_state() {
    for (stop, reason) in [
        ("max_tokens", "max_output_tokens"),
        ("model_context_window_exceeded", "max_output_tokens"),
        ("refusal", "content_filter"),
    ] {
        let mut body = fixture();
        body["stop_reason"] = stop.into();
        let out = convert(&body, "published", "resp_fixture", &ChatTools::default()).unwrap();
        assert_eq!(out["status"], "incomplete");
        assert_eq!(out["incomplete_details"]["reason"], reason);
        assert_eq!(out["output"][1]["status"], "incomplete");
        if stop == "refusal" {
            assert_eq!(out["output"][0]["content"][0]["refusal"], "before");
        }
    }
}

#[test]
fn cc_route_058_invalid_upstream_is_rejected() {
    for stop in [Value::Null, json!("pause_turn"), json!("unknown")] {
        let mut body = fixture();
        body["stop_reason"] = stop;
        assert!(convert(&body, "published", "resp_fixture", &ChatTools::default()).is_err());
    }
    let mut body = fixture();
    body["usage"]["input_tokens"] = u64::MAX.into();
    assert!(convert(&body, "published", "resp_fixture", &ChatTools::default()).is_err());
    let mut body = fixture();
    body["error"] = json!({"type":"api_error"});
    assert!(convert(&body, "published", "resp_fixture", &ChatTools::default()).is_err());
}
