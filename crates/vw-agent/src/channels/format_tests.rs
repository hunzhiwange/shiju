use super::*;
use crate::app::agent::tools::{ToolResult, ToolSpec};
use std::collections::HashSet;

struct NamedTool {
    id: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.id
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.id, self.description(), self.parameters_schema())
    }
}

#[test]
fn strip_tool_call_tags_removes_closed_tool_blocks() {
    let message = "before <tool>{\"name\":\"secret\"}</tool> after";

    assert_eq!(strip_tool_call_tags(message), "before  after");
}

#[test]
fn strip_tool_call_tags_handles_aliases_unclosed_json_and_sentinel() {
    assert_eq!(strip_tool_call_tags("a <toolcall>{\"name\":\"x\"}</toolcall> b"), "a  b");
    assert_eq!(strip_tool_call_tags("a <invoke>{\"name\":\"x\"} trailing"), "a trailing");
    assert_eq!(strip_tool_call_tags("a ฦ{\"name\":\"x\"}ฦ b"), "a  b");
}

#[test]
fn strip_tool_call_tags_preserves_unparseable_unclosed_blocks() {
    let original = "before <tool>not-json";

    assert_eq!(strip_tool_call_tags(original), original);
    assert_eq!(strip_tool_call_tags("before <tool>{not-json"), "before <tool>{not-json");
}

#[test]
fn split_internal_progress_delta_detects_known_prefix() {
    let delta = format!("{}working", crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL);
    let (is_progress, content) = split_internal_progress_delta(&delta);

    assert!(is_progress);
    assert_eq!(content, "working");
}

#[test]
fn split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text() {
    let ws_delta = format!(
        "{}{}payload",
        crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL,
        crate::app::agent::agent::loop_::DRAFT_WS_EVENT_SENTINEL
    );

    assert_eq!(split_internal_progress_delta(&ws_delta), (true, ""));
    assert_eq!(split_internal_progress_delta("plain"), (false, "plain"));
}

#[test]
fn channel_delivery_instructions_are_known_for_chat_channels() {
    assert!(channel_delivery_instructions("telegram").is_some());
    assert!(channel_delivery_instructions("whatsapp").is_some());
    assert!(channel_delivery_instructions("lark").is_some());
    assert!(channel_delivery_instructions("feishu").is_some());
    assert!(channel_delivery_instructions("unknown").is_none());
}

#[test]
fn channel_delivery_instructions_include_channel_specific_requirements() {
    let telegram = channel_delivery_instructions("telegram").expect("telegram instructions");
    let whatsapp = channel_delivery_instructions("whatsapp").expect("whatsapp instructions");
    let lark = channel_delivery_instructions("lark").expect("lark instructions");
    let feishu = channel_delivery_instructions("feishu").expect("feishu instructions");

    assert!(telegram.contains("[VOICE:<path-or-url>]"));
    assert!(telegram.contains("bold headers"));
    assert!(whatsapp.contains("[IMAGE:<absolute-path>]"));
    assert!(whatsapp.contains("absolute filesystem path"));
    for instructions in [lark, feishu] {
        assert!(instructions.contains("Markdown"));
        assert!(instructions.contains("tables"));
        assert!(instructions.contains("code blocks"));
    }
    assert_eq!(lark, feishu);
}

#[test]
fn should_expose_internal_tool_details_respects_positive_and_negative_intents() {
    assert!(should_expose_internal_tool_details("please show tool call json"));
    assert!(should_expose_internal_tool_details("把执行过程列出来"));
    assert!(!should_expose_internal_tool_details("do not show command output"));
    assert!(!should_expose_internal_tool_details("不要显示工具调用"));
    assert!(!should_expose_internal_tool_details("   "));
}

#[test]
fn extract_tool_context_summary_collects_unique_tool_names_after_start_index() {
    let history = vec![
        ChatMessage::assistant(r#"<tool_call>{"name":"ignored"}</tool_call>"#),
        ChatMessage::assistant(r#"<toolcall>{"name":"shell"}</toolcall>"#),
        ChatMessage::assistant(
            r#"{"tool_calls":[{"function":{"name":"file_read","arguments":{}}},{"name":"shell","arguments":{}}]}"#,
        ),
        ChatMessage::user(r#"<tool_result name="grep">ok</tool_result>"#),
    ];

    assert_eq!(extract_tool_context_summary(&history, 1), "[Used tools: shell, file_read, grep]");
    assert_eq!(extract_tool_context_summary(&history, history.len()), "");
}

#[test]
fn tool_json_payload_detection_requires_known_tool_and_arguments() {
    let known = HashSet::from(["shell".to_string()]);

    assert!(is_tool_call_payload(
        &serde_json::json!({"name": "shell", "arguments": {"cmd": "ls"}}),
        &known
    ));
    assert!(is_tool_call_payload(
        &serde_json::json!({"function": {"name": "shell", "parameters": {}}}),
        &known
    ));
    assert!(!is_tool_call_payload(
        &serde_json::json!({"name": "unknown", "arguments": {}}),
        &known
    ));
    assert!(!is_tool_call_payload(&serde_json::json!({"name": "shell"}), &known));
}

#[test]
fn tool_result_payload_requires_prior_tool_call_and_result_shape() {
    let object = serde_json::json!({"result": "ok", "tool_call_id": "1"})
        .as_object()
        .cloned()
        .expect("object");

    assert!(is_tool_result_payload(&object, true));
    assert!(!is_tool_result_payload(&object, false));

    let noisy =
        serde_json::json!({"result": "ok", "extra": true}).as_object().cloned().expect("object");
    assert!(!is_tool_result_payload(&noisy, true));
}

#[test]
fn sanitize_tool_json_value_strips_calls_and_keeps_message_content() {
    let known = HashSet::from(["shell".to_string()]);

    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!({"name": "shell", "arguments": {"cmd": "ls"}}),
            &known,
            false
        ),
        Some((String::new(), true))
    );
    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!({
                "content": "visible",
                "tool_calls": [{"function": {"name": "shell", "arguments": {}}}]
            }),
            &known,
            false
        ),
        Some(("visible".to_string(), true))
    );
    assert_eq!(
        sanitize_tool_json_value(&serde_json::json!({"result": "done"}), &known, true),
        Some((String::new(), false))
    );
}

#[test]
fn isolated_tool_json_artifacts_are_removed_without_touching_inline_json() {
    let known = HashSet::from(["shell".to_string()]);
    let message = concat!(
        "before\n",
        "{\"name\":\"shell\",\"arguments\":{\"cmd\":\"ls\"}}\n",
        "{\"result\":\"ok\"}\n",
        "inline {\"name\":\"shell\",\"arguments\":{}} stays"
    );

    let cleaned = strip_isolated_tool_json_artifacts(message, &known);

    assert_eq!(cleaned, "before\n\ninline {\"name\":\"shell\",\"arguments\":{}} stays");
}

#[test]
fn sanitize_channel_response_removes_known_tool_json_and_tags() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(NamedTool { id: "shell" })];
    let response = concat!(
        "hello\n",
        "<tool>{\"name\":\"shell\",\"arguments\":{}}</tool>\n",
        "{\"name\":\"shell\",\"arguments\":{\"cmd\":\"pwd\"}}\n",
        "done"
    );

    assert_eq!(sanitize_channel_response(response, &tools), "hello\n\ndone");
}

#[test]
fn line_isolated_json_segment_detects_whitespace_boundaries() {
    let message = "prefix\n  {\"a\":1}  \nsuffix";
    let start = message.find('{').expect("json start");
    let end = start + "{\"a\":1}".len();

    assert!(is_line_isolated_json_segment(message, start, end));

    let inline = "prefix {\"a\":1} suffix";
    let start = inline.find('{').expect("json start");
    let end = start + "{\"a\":1}".len();
    assert!(!is_line_isolated_json_segment(inline, start, end));
}

#[test]
fn strip_tool_call_tags_removes_all_closed_tag_variants() {
    let cases = [
        ("<function_calls>{\"name\":\"x\"}</function_calls>", "before  after"),
        ("<function_call>{\"name\":\"x\"}</function_call>", "before  after"),
        ("<tool_call>{\"name\":\"x\"}</tool_call>", "before  after"),
        ("<tool-call>{\"name\":\"x\"}</tool-call>", "before  after"),
    ];

    for (tagged, expected) in cases {
        assert_eq!(strip_tool_call_tags(&format!("before {tagged} after")), expected);
    }
}

#[test]
fn strip_tool_call_tags_handles_block_boundaries_and_extra_blank_lines() {
    let message = "intro\n   <tool>{\"name\":\"x\"}</tool>\n   next\n\n\n\nend";

    assert_eq!(strip_tool_call_tags(message), "intro\nnext\n\nend");
}

#[test]
fn strip_tool_call_tags_keeps_empty_line_after_removed_block() {
    let message = "intro\n<tool>{\"name\":\"x\"}</tool>\n   ";

    assert_eq!(strip_tool_call_tags(message), "intro");
}

#[test]
fn strip_tool_call_tags_handles_unclosed_json_arrays_and_leading_close_tags() {
    assert_eq!(strip_tool_call_tags("a <invoke>[{\"name\":\"x\"}] </tool> b"), "a b");
    assert_eq!(strip_tool_call_tags("a <tool>noise {\"name\":\"x\"}</broken"), "a");
}

#[test]
fn strip_tool_call_tags_uses_earliest_tool_marker() {
    assert_eq!(
        strip_tool_call_tags("a ฦ{\"name\":\"first\"}ฦ <tool>{\"name\":\"second\"}</tool> b"),
        "a   b"
    );
    assert_eq!(
        strip_tool_call_tags("a <tool>{\"name\":\"first\"}</tool> ฦ{\"name\":\"second\"}ฦ b"),
        "a   b"
    );
}

#[test]
fn strip_tool_call_tags_preserves_messages_without_tags() {
    assert_eq!(strip_tool_call_tags("  plain message  "), "plain message");
}

#[test]
fn should_expose_internal_tool_details_covers_verb_and_no_match_paths() {
    assert!(should_expose_internal_tool_details("please display the internal step details"));
    assert!(should_expose_internal_tool_details("把命令贴出"));
    assert!(!should_expose_internal_tool_details("show me the summary"));
    assert!(!should_expose_internal_tool_details("command"));
}

#[test]
fn should_expose_internal_tool_details_handles_english_verb_and_negation_hints() {
    assert!(should_expose_internal_tool_details("please share the command output"));
    assert!(!should_expose_internal_tool_details("do not show function call details"));
    assert!(!should_expose_internal_tool_details("skip commands in the reply"));
}

#[test]
fn extract_tool_context_summary_handles_tag_variants_and_malformed_results() {
    let history = vec![
        ChatMessage::assistant(r#"<tool_call>{"name":"alpha"}</tool_call>"#),
        ChatMessage::assistant(r#"<tool-call>{"name":"beta"}</tool-call>"#),
        ChatMessage::assistant(r#"<invoke>{"name":"gamma"}</invoke>"#),
        ChatMessage::assistant(r#"<tool_call>{"name":" "}</tool_call>"#),
        ChatMessage::assistant(r#"<tool_call>not-json</tool_call>"#),
        ChatMessage::user(r#"<tool_result name="" />"#),
        ChatMessage::user(r#"<tool_result name="unterminated"#),
        ChatMessage::system("ignored"),
    ];

    assert_eq!(extract_tool_context_summary(&history, 0), "[Used tools: alpha, beta, gamma]");
}

#[test]
fn extract_tool_context_summary_ignores_native_json_without_calls() {
    let history = vec![
        ChatMessage::assistant(r#"{"tool_calls":[]}"#),
        ChatMessage::assistant(r#"{"tool_calls":{}}"#),
        ChatMessage::assistant("not-json"),
    ];

    assert_eq!(extract_tool_context_summary(&history, 0), "");
}

#[test]
fn extract_tool_context_summary_deduplicates_native_and_result_names() {
    let history = vec![
        ChatMessage::assistant(
            r#"{"tool_calls":[{"name":"shell","arguments":{}},{"function":{"name":"shell","arguments":{}}},{"function":{"name":"grep","arguments":{}}}]}"#,
        ),
        ChatMessage::user(
            r#"<tool_result name="shell">ok</tool_result><tool_result name="grep">done</tool_result>"#,
        ),
    ];

    assert_eq!(extract_tool_context_summary(&history, 0), "[Used tools: shell, grep]");
}

#[test]
fn is_tool_call_payload_accepts_fallback_names_and_rejects_bad_shapes() {
    let known = HashSet::from(["shell".to_string()]);

    assert!(is_tool_call_payload(
        &serde_json::json!({"name": " SHELL ", "function": {}, "arguments": {}}),
        &known
    ));
    assert!(!is_tool_call_payload(&serde_json::json!("shell"), &known));
    assert!(!is_tool_call_payload(&serde_json::json!({"name": " ", "arguments": {}}), &known));
    assert!(!is_tool_call_payload(&serde_json::json!({"function": {"name": "shell"}}), &known));
}

#[test]
fn is_tool_result_payload_rejects_objects_without_result() {
    let object = serde_json::json!({"tool_call_id": "1"}).as_object().cloned().expect("object");

    assert!(!is_tool_result_payload(&object, true));
}

#[test]
fn sanitize_tool_json_value_handles_arrays_and_non_objects() {
    let known = HashSet::from(["shell".to_string()]);

    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!([{"name": "shell", "arguments": {}}]),
            &known,
            false
        ),
        Some((String::new(), true))
    );
    assert_eq!(sanitize_tool_json_value(&serde_json::json!([]), &known, false), None);
    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!([{"name": "unknown", "arguments": {}}]),
            &known,
            false
        ),
        None
    );
    assert_eq!(sanitize_tool_json_value(&serde_json::json!("text"), &known, false), None);
}

#[test]
fn sanitize_tool_json_value_handles_tool_call_messages_without_content() {
    let known = HashSet::from(["shell".to_string()]);

    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!({"tool_calls": [{"name": "shell", "arguments": {}}]}),
            &known,
            false
        ),
        Some((String::new(), true))
    );
    assert_eq!(
        sanitize_tool_json_value(
            &serde_json::json!({"tool_calls": [{"name": "unknown", "arguments": {}}]}),
            &known,
            false
        ),
        None
    );
}

#[test]
fn strip_isolated_tool_json_artifacts_replaces_message_objects_and_normalizes_crlf() {
    let known = HashSet::from(["shell".to_string()]);
    let message = concat!(
        "before\r\n",
        "{\"content\":\" visible \",\"tool_calls\":[{\"name\":\"shell\",\"arguments\":{}}]}\r\n",
        "after"
    );

    assert_eq!(strip_isolated_tool_json_artifacts(message, &known), "before\nvisible\nafter");
}

#[test]
fn strip_isolated_tool_json_artifacts_preserves_invalid_and_unrelated_json() {
    let known = HashSet::from(["shell".to_string()]);
    let message = "before\n{not json}\n{\"result\":\"early\"}\n{\"a\":1}\nafter";

    assert_eq!(strip_isolated_tool_json_artifacts(message, &known), message);
}

#[test]
fn strip_isolated_tool_json_artifacts_removes_tool_arrays_and_following_results() {
    let known = HashSet::from(["shell".to_string()]);
    let message = concat!(
        "before\n",
        "[{\"name\":\"shell\",\"arguments\":{}}]\n",
        "{\"result\":\"ok\",\"tool\":\"shell\"}\n",
        "after"
    );

    assert_eq!(strip_isolated_tool_json_artifacts(message, &known), "before\n\nafter");
}

#[test]
fn strip_isolated_tool_json_artifacts_keeps_early_results_until_a_tool_call_is_seen() {
    let known = HashSet::from(["shell".to_string()]);
    let message = concat!(
        "before\n",
        "{\"result\":\"early\"}\n",
        "{\"name\":\"shell\",\"arguments\":{}}\n",
        "{\"result\":\"late\"}\n",
        "after"
    );

    assert_eq!(
        strip_isolated_tool_json_artifacts(message, &known),
        "before\n{\"result\":\"early\"}\n\nafter"
    );
}

#[test]
fn strip_isolated_tool_json_artifacts_preserves_plain_text_without_json() {
    let known = HashSet::new();

    assert_eq!(strip_isolated_tool_json_artifacts("  no json  ", &known), "no json");
}

#[test]
fn sanitize_channel_response_preserves_clean_response_without_tools() {
    let tools = Vec::<Box<dyn Tool>>::new();

    assert_eq!(sanitize_channel_response("  hello  ", &tools), "hello");
}

#[test]
fn sanitize_channel_response_matches_tool_names_case_insensitively() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(NamedTool { id: "Shell" })];
    let response = concat!(
        "hello\n",
        "{\"name\":\"shell\",\"arguments\":{\"cmd\":\"pwd\"}}\n",
        "{\"result\":\"done\"}\n",
        "done"
    );

    assert_eq!(sanitize_channel_response(response, &tools), "hello\n\ndone");
}
