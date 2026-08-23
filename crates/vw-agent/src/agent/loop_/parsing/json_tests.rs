//! 测试 JSON 工具调用解析对合法载荷、包装格式与无效输入的处理。

use super::*;

#[test]
fn parse_arguments_accepts_string_object_and_missing_values() {
    assert_eq!(
        parse_arguments_value(Some(&serde_json::json!(r#"{"a":1}"#))),
        serde_json::json!({"a": 1})
    );
    assert_eq!(
        parse_arguments_value(Some(&serde_json::json!({"b": 2}))),
        serde_json::json!({"b": 2})
    );
    assert_eq!(parse_arguments_value(None), serde_json::json!({}));
}

#[test]
fn parse_arguments_handles_invalid_strings_and_non_object_json() {
    assert_eq!(parse_arguments_value(Some(&serde_json::json!("{bad json"))), serde_json::json!({}));
    assert_eq!(parse_arguments_value(Some(&serde_json::json!("7"))), serde_json::json!(7));
    assert_eq!(parse_arguments_value(Some(&serde_json::json!(["x"]))), serde_json::json!(["x"]));
}

#[test]
fn parse_tool_call_id_uses_priority_order_and_ignores_blank_ids() {
    let value = serde_json::json!({
        "id": " root-id ",
        "tool_call_id": "tool-id",
        "call_id": "call-id",
        "function": { "id": " function-id " }
    });
    let blank_function = serde_json::json!({
        "id": "root-id",
        "function": { "id": "   " }
    });

    assert_eq!(parse_tool_call_id(&value, value.get("function")).as_deref(), Some("function-id"));
    assert_eq!(parse_tool_call_id(&value, None).as_deref(), Some("root-id"));
    assert_eq!(
        parse_tool_call_id(&blank_function, blank_function.get("function")).as_deref(),
        None
    );
    assert_eq!(
        parse_tool_call_id(&serde_json::json!({"call_id": " call "}), None).as_deref(),
        Some("call")
    );
}

#[test]
fn parse_tool_call_id_uses_tool_call_id_and_rejects_non_strings() {
    assert_eq!(
        parse_tool_call_id(&serde_json::json!({"tool_call_id": " tool "}), None).as_deref(),
        Some("tool")
    );
    assert_eq!(parse_tool_call_id(&serde_json::json!({"id": 7}), None), None);
}

#[test]
fn tool_signature_is_order_independent_and_name_normalized() {
    let (name_a, args_a) =
        tool_call_signature(" Shell ", &serde_json::json!({"b": [{"z": 2, "a": 1}], "a": 1}));
    let (name_b, args_b) =
        tool_call_signature("shell", &serde_json::json!({"a": 1, "b": [{"a": 1, "z": 2}]}));

    assert_eq!(name_a, "shell");
    assert_eq!(name_b, "shell");
    assert_eq!(args_a, args_b);
}

#[test]
fn parses_single_tool_call_values_from_nested_and_flat_shapes() {
    let nested = serde_json::json!({
        "id": "call-nested",
        "function": {
            "name": "bash",
            "arguments": "pwd"
        }
    });
    let flat = serde_json::json!({
        "call_id": "call-flat",
        "name": "grep",
        "parameters": { "query": "needle" }
    });
    let fallback_to_flat = serde_json::json!({
        "name": "codesearch",
        "arguments": "Symbol",
        "function": { "name": "   " }
    });

    let nested_call = parse_tool_call_value(&nested).unwrap();
    let flat_call = parse_tool_call_value(&flat).unwrap();
    let fallback_call = parse_tool_call_value(&fallback_to_flat).unwrap();

    assert_eq!(nested_call.name, "bash");
    assert_eq!(nested_call.tool_call_id.as_deref(), Some("call-nested"));
    assert_eq!(nested_call.arguments, serde_json::json!({"command": "pwd"}));
    assert_eq!(flat_call.name, "grep");
    assert_eq!(flat_call.tool_call_id.as_deref(), Some("call-flat"));
    assert_eq!(flat_call.arguments, serde_json::json!({"query": "needle", "pattern": "needle"}));
    assert_eq!(fallback_call.arguments, serde_json::json!({"query": "Symbol"}));
    assert!(parse_tool_call_value(&serde_json::json!({"arguments": {}})).is_none());
}

#[test]
fn parses_nested_tool_call_parameters_and_function_id() {
    let value = serde_json::json!({
        "id": "root-id",
        "function": {
            "id": "function-id",
            "name": "shell",
            "parameters": { "cmd": "date" }
        }
    });

    let call = parse_tool_call_value(&value).unwrap();

    assert_eq!(call.tool_call_id.as_deref(), Some("function-id"));
    assert_eq!(call.arguments, serde_json::json!({"cmd": "date", "command": "date"}));
}

#[test]
fn parses_nested_tool_calls_from_json_value() {
    let value = serde_json::json!({
        "choices": [{
            "message": {
                "tool_calls": [{
                    "id": "call-1",
                    "function": {"name": "shell", "arguments": "{\"command\":\"pwd\"}"}
                }]
            }
        }]
    });

    let calls = parse_tool_calls_from_json_value(&value);

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call-1"));
    assert_eq!(calls[0].arguments["command"], "pwd");
}

#[test]
fn parses_tool_calls_from_arrays_single_values_and_empty_wrappers() {
    let array = serde_json::json!([
        {"name": "grep", "arguments": "needle"},
        {"name": "", "arguments": {}},
        {"function": {"name": "shell", "parameters": {"cmd": "date"}}}
    ]);
    let empty_wrapper = serde_json::json!({
        "tool_calls": [],
        "message": {
            "name": "codesearch",
            "arguments": {"input": "Parser"}
        }
    });
    let choices = serde_json::json!({
        "choices": [
            {"message": {"name": "grep", "arguments": {"pattern": "a"}}},
            {"message": {"name": "glob", "arguments": {"input": "*.rs"}}}
        ]
    });

    let array_calls = parse_tool_calls_from_json_value(&array);
    let wrapper_calls = parse_tool_calls_from_json_value(&empty_wrapper);
    let choice_calls = parse_tool_calls_from_json_value(&choices);

    assert_eq!(array_calls.len(), 2);
    assert_eq!(array_calls[0].arguments, serde_json::json!({"pattern": "needle"}));
    assert_eq!(array_calls[1].arguments, serde_json::json!({"cmd": "date", "command": "date"}));
    assert_eq!(wrapper_calls.len(), 1);
    assert_eq!(
        wrapper_calls[0].arguments,
        serde_json::json!({"input": "Parser", "query": "Parser"})
    );
    assert_eq!(choice_calls.len(), 2);
    assert_eq!(choice_calls[0].arguments["pattern"], "a");
    assert_eq!(choice_calls[1].arguments["pattern"], "*.rs");
}

#[test]
fn parse_tool_calls_from_json_value_returns_empty_for_non_calls() {
    assert!(parse_tool_calls_from_json_value(&serde_json::json!({"tool_calls": []})).is_empty());
    assert!(
        parse_tool_calls_from_json_value(&serde_json::json!({"choices": [{"delta": {}}]}))
            .is_empty()
    );
    assert!(
        parse_tool_calls_from_json_value(&serde_json::json!({"message": {"name": ""}})).is_empty()
    );
    assert!(parse_tool_calls_from_json_value(&serde_json::json!(true)).is_empty());
}

#[test]
fn extracts_first_json_value_after_close_tags() {
    let input = "</tool_call> text {\"a\":1} tail";
    let stripped = strip_leading_close_tags(input);
    let (value, end) = extract_first_json_value_with_end(stripped).unwrap();

    assert_eq!(value, serde_json::json!({"a": 1}));
    assert!(end <= stripped.len());
    assert_eq!(find_json_end("{\"a\":1} tail"), Some(7));
}

#[test]
fn extracts_first_json_value_skips_invalid_candidates() {
    let (value, end) =
        extract_first_json_value_with_end("bad {not json} then {\"ok\":true} tail").unwrap();

    assert_eq!(value, serde_json::json!({"ok": true}));
    assert_eq!("bad {not json} then {\"ok\":true}".len(), end);
}

#[test]
fn json_value_extraction_handles_arrays_escapes_and_missing_json() {
    let (array, end) =
        extract_first_json_value_with_end("prefix [1, {\"a\": true}] suffix").unwrap();
    assert_eq!(array, serde_json::json!([1, {"a": true}]));
    assert_eq!("prefix [1, {\"a\": true}]".len(), end);

    assert_eq!(strip_leading_close_tags(" </a> </b> {\"ok\":true}"), "{\"ok\":true}");
    assert_eq!(strip_leading_close_tags("</broken"), "");
    assert!(extract_first_json_value_with_end("plain text").is_none());
    let escaped = r#" {"text":"brace } and quote \" ok"} tail"#;
    let escaped_end = find_json_end(escaped).unwrap();
    assert_eq!(&escaped[escaped_end..], " tail");
    assert!(find_json_end("[1,2]").is_none());
    assert!(find_json_end("{\"missing\": true").is_none());
}

#[test]
fn json_value_helpers_handle_trimming_and_nested_objects() {
    assert_eq!(strip_leading_close_tags("  plain"), "plain");
    assert_eq!(find_json_end("  {\"outer\":{\"inner\":\"}\"}} rest"), Some(25));
}
