use crate::chat::{sanitize_gateway_response, log_channel_message};
use tool_types::{Tool, ToolResult};
struct MockScheduleTool;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for MockScheduleTool {
    fn name(&self) -> &str {
        "schedule"
    }

    fn description(&self) -> &str {
        "Mock schedule tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string" }
            }
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

#[test]
fn sanitize_gateway_response_keeps_plain_text() {
    assert_eq!(sanitize_gateway_response("plain response", &[]), "plain response");
}

#[test]
fn sanitize_gateway_response_keeps_empty_and_drops_blank_text() {
    assert_eq!(sanitize_gateway_response("", &[]), "");
    assert_eq!(sanitize_gateway_response("   ", &[]), "");
}

#[test]
fn sanitize_gateway_response_returns_safe_message_when_nonempty_response_is_stripped() {
    let input = r#"ฦ
{"name":"schedule","arguments":{"action":"create"}}
ฦ"#;

    assert_eq!(
        sanitize_gateway_response(input, &[]),
        "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
    );
}

#[test]
fn sanitize_gateway_response_removes_known_isolated_tool_json() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(MockScheduleTool)];
    let input = r#"{"name":"schedule","parameters":{"action":"create"}}
{"result":{"status":"scheduled"}}
Reminder set."#;

    assert_eq!(sanitize_gateway_response(input, &tools), "Reminder set.");
}

#[test]
fn sanitize_gateway_response_truncates_long_text() {
    let input = "x".repeat(20_000);
    let sanitized = sanitize_gateway_response(&input, &[]);

    assert_eq!(sanitized.chars().count(), 16_001);
    assert!(sanitized.ends_with('…'));
    assert!(!sanitized.ends_with("..."));
}

#[test]
fn sanitize_gateway_response_keeps_text_at_limit_without_ellipsis() {
    let input = "界".repeat(16_000);

    let sanitized = sanitize_gateway_response(&input, &[]);

    assert_eq!(sanitized, input);
    assert_eq!(sanitized.chars().count(), 16_000);
}

#[test]
fn sanitize_gateway_response_normalizes_short_trailing_ellipsis() {
    assert_eq!(sanitize_gateway_response("Please wait...", &[]), "Please wait…");
}

#[test]
fn log_channel_message_accepts_long_content() {
    log_channel_message("test-channel", "sender-1", &"内容".repeat(80));
}
