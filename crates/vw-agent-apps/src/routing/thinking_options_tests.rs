use super::thinking_options::normalize_messages;
use serde_json::json;

#[test]
fn cc_route_064_thinking_options_expected_behavior() {
    let mut body = json!({"model":"deepseek-chat", "thinking":{"type":"disabled","budget_tokens":500},
        "reasoning_effort":"high", "output_config":{"effort":"max","format":"json"}});
    normalize_messages(&mut body, "api.deepseek.com", "/anthropic/v1/messages");
    assert_eq!(body["thinking"], json!({"type":"disabled"}));
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(body["output_config"], json!({"format":"json"}));
}

#[test]
fn cc_route_064_adaptive_defaults_and_disable_constraints() {
    for (model, kind, expected) in [
        ("claude-sonnet-5", None, "adaptive"),
        ("claude-sonnet-5", Some("disabled"), "disabled"),
        ("claude-fable-5", Some("disabled"), "adaptive"),
        ("claude-opus-4.8", Some("enabled"), "adaptive"),
    ] {
        let mut body = json!({"model":model});
        if let Some(kind) = kind {
            body["thinking"] = json!({"type":kind,"budget_tokens":5000});
        }
        normalize_messages(&mut body, "api.anthropic.com", "/v1/messages");
        assert_eq!(body["thinking"]["type"], expected);
        assert!(body["thinking"].get("budget_tokens").is_none());
        if model == "claude-fable-5" {
            assert_eq!(body["output_config"]["effort"], "low");
        }
    }
}

#[test]
fn cc_route_064_budget_and_target_boundaries() {
    for (max, expected) in [(1000, "disabled"), (4096, "enabled")] {
        let mut body = json!({"model":"claude-sonnet-4-5","max_tokens":max,
            "thinking":{"type":"enabled","budget_tokens":9000}});
        normalize_messages(&mut body, "api.anthropic.com", "/v1/messages");
        assert_eq!(body["thinking"]["type"], expected);
        if max == 4096 {
            assert_eq!(body["thinking"]["budget_tokens"], 2048);
        }
    }
    for (host, path) in [
        ("relay.example", "/v1/messages"),
        ("api.deepseek.com", "/v1/messages"),
        ("api.deepseek.com", "/anthropic/v1/chat/completions"),
    ] {
        let mut body = json!({"model":"deepseek-chat","thinking":{"type":"disabled"},"reasoning_effort":"high"});
        let original = body.clone();
        normalize_messages(&mut body, host, path);
        assert_eq!(body, original);
    }
}
