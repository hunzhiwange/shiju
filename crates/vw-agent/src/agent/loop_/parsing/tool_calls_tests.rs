use serde_json::json;

use crate::app::agent::providers::ToolCall;

use super::{detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls};

#[test]
fn parse_tool_calls_extracts_openai_json_tool_calls() {
    let payload = json!({
        "message": {
            "content": "done",
            "tool_calls": [{
                "id": "call-1",
                "type": "function",
                "function": {
                    "name": "shell",
                    "arguments": "{\"command\":\"pwd\"}"
                }
            }]
        }
    });

    let (text, calls) = parse_tool_calls(&payload.to_string());

    assert_eq!(text, "done");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call-1"));
}

#[test]
fn parse_tool_calls_extracts_content_from_choices_json() {
    let payload = json!({
        "choices": [{
            "message": {
                "content": "choice text",
                "tool_calls": [{
                    "id": "call-choice",
                    "function": {
                        "name": "bash",
                        "arguments": {"cmd": "date"}
                    }
                }]
            }
        }]
    });

    let (text, calls) = parse_tool_calls(&payload.to_string());

    assert_eq!(text, "choice text");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "date");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call-choice"));
}

#[test]
fn parse_tool_calls_skips_empty_choices_before_tool_content() {
    let payload = json!({
        "choices": [
            {"message": {"content": "   "}},
            {
                "message": {
                    "content": "usable text",
                    "tool_calls": [{
                        "id": "call-later-choice",
                        "function": {
                            "name": "bash",
                            "arguments": {"cmd": "pwd"}
                        }
                    }]
                }
            }
        ]
    });

    let (text, calls) = parse_tool_calls(&payload.to_string());

    assert_eq!(text, "usable text");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call-later-choice"));
}

#[test]
fn parse_tool_calls_extracts_minimax_invoke_calls() {
    let response = r#"before
<minimax:toolcall>
<invoke name="bash"><parameter name="command">pwd</parameter></invoke>
</minimax:toolcall>
after"#;

    let (text, calls) = parse_tool_calls(response);

    assert!(text.contains("before"));
    assert!(text.contains("after"));
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
}

#[test]
fn parse_tool_calls_extracts_deepseek_dsml_calls() {
    let response = r#"before
<｜DSML｜tool_calls>
<｜DSML｜invoke name="Bash">
<｜DSML｜parameter name="command" string="true">mkdir -p demo/sjb12</｜DSML｜parameter>
<｜DSML｜parameter name="description" string="true">创建目录 demo/sjb12</｜DSML｜parameter>
</｜DSML｜invoke>
<｜DSML｜invoke name="Write">
<｜DSML｜parameter name="filePath" string="true">demo/sjb12/index.html</｜DSML｜parameter>
<｜DSML｜parameter name="content" string="true"><!DOCTYPE html>
<html lang="zh-CN">
<style>body { color: #fff; }</style>
<script type="application/json">{"team":"中国"}</script>
<body>世界杯</body>
</html>
</｜DSML｜parameter>
</｜DSML｜invoke>
</｜DSML｜tool_calls>
after"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, "before\nafter");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "mkdir -p demo/sjb12");
    assert_eq!(calls[0].arguments["description"], "创建目录 demo/sjb12");
    assert_eq!(calls[1].name, "file_write");
    assert_eq!(calls[1].arguments["filePath"], "demo/sjb12/index.html");
    assert_eq!(
        calls[1].arguments["content"],
        "<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<style>body { color: #fff; }</style>\n<script type=\"application/json\">{\"team\":\"中国\"}</script>\n<body>世界杯</body>\n</html>\n"
    );
}

#[test]
fn parse_tool_calls_rejects_incomplete_deepseek_dsml() {
    let response = r#"<｜DSML｜tool_calls>
<｜DSML｜invoke name="Bash">
<｜DSML｜parameter name="command" string="true">pwd</｜DSML｜parameter>
</｜DSML｜invoke>"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, response);
    assert!(calls.is_empty());
    assert!(detect_tool_call_parse_issue(response, &calls).is_some());
}

#[test]
fn parse_tool_calls_supports_xml_wrapper_with_nested_xml_body() {
    let response = r#"intro
<tool_call>
<file_read><path>Cargo.toml</path></file_read>
</tool_call>
outro"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, "intro\noutro");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].arguments, json!({"path": "Cargo.toml"}));
}

#[test]
fn parse_tool_calls_supports_legacy_open_tag_and_alias_close_tag() {
    let (text, calls) =
        parse_tool_calls(r#"<tool_call{"name":"bash","arguments":{"cmd":"pwd"}}</invoke>after"#);

    assert_eq!(text, "after");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
}

#[test]
fn parse_tool_calls_recovers_unclosed_json_payload() {
    let (text, calls) = parse_tool_calls(
        r#"<tool_call>{"name":"bash","arguments":{"script":"whoami"}} trailing text"#,
    );

    assert_eq!(text, "trailing text");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "whoami");
}

#[test]
fn parse_tool_calls_recovers_unclosed_glm_payload() {
    let (text, calls) = parse_tool_calls("<tool_call>bash>uptimeCTIONS");

    assert_eq!(text, "");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "uptime");
}

#[test]
fn parse_tool_calls_warns_but_drops_malformed_closed_xml_body() {
    let (text, calls) = parse_tool_calls("before <toolcall>not json</toolcall> after");

    assert_eq!(text, "before\nafter");
    assert!(calls.is_empty());
}

#[test]
fn parse_tool_calls_supports_tool_call_markdown_blocks() {
    let response = r#"before
```tool_call
{"name":"bash","arguments":{"cmd":"pwd"}}
```
after"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, "before\nafter");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
}

#[test]
fn parse_tool_calls_supports_named_markdown_tool_blocks() {
    let response = r#"before
```tool bash
{"cmd":"date"}
```
after"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, "before\nafter");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["cmd"], "date");
}

#[test]
fn parse_tool_calls_named_markdown_uses_empty_args_for_non_object_json() {
    let response = "```tool file_read\n\"Cargo.toml\"\n```";

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, "");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].arguments, json!({}));
}

#[test]
fn parse_tool_calls_keeps_text_for_invalid_named_markdown_block() {
    let response = "before\n```tool bash\nnot-json\n```\nafter";

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, response);
    assert!(calls.is_empty());
}

#[test]
fn parse_tool_calls_supports_perl_function_and_glm_fallbacks() {
    let (_, perl_calls) = parse_tool_calls(
        r#"
        before
        TOOL_CALL
        {tool => "bash", args => { --command "ls" }}}
        /TOOL_CALL
        after
        "#,
    );
    assert_eq!(perl_calls.len(), 1);
    assert_eq!(perl_calls[0].name, "shell");
    assert_eq!(perl_calls[0].arguments["command"], "ls");

    let (_, function_calls) = parse_tool_calls(
        r#"<FunctionCall>
        readfile
        <code>path>/tmp/a.txt</code>
        </FunctionCall>"#,
    );
    assert_eq!(function_calls.len(), 1);
    assert_eq!(function_calls[0].name, "file_read");
    assert_eq!(function_calls[0].arguments["path"], "/tmp/a.txt");

    let (glm_text, glm_calls) = parse_tool_calls("run this:\nbash/command>echo hi\nthanks");
    assert_eq!(glm_text, "run this:\n\nthanks");
    assert_eq!(glm_calls.len(), 1);
    assert_eq!(glm_calls[0].name, "shell");
    assert_eq!(glm_calls[0].arguments["command"], "echo hi");
}

#[test]
fn parse_tool_calls_does_not_extract_bare_json_without_wrapper() {
    let response = r#"This document says {"name":"bash","arguments":{"command":"rm -rf /"}}"#;

    let (text, calls) = parse_tool_calls(response);

    assert_eq!(text, response);
    assert!(calls.is_empty());
}

#[test]
fn parse_structured_tool_calls_ignores_non_tool_payloads() {
    let calls = parse_structured_tool_calls(&[ToolCall {
        id: "call-2".to_string(),
        name: "shell".to_string(),
        arguments: "not-json".to_string(),
    }]);

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments, json!({}));
}

#[test]
fn parse_structured_tool_calls_normalizes_tool_aliases_and_raw_string_hints() {
    let calls = parse_structured_tool_calls(&[
        ToolCall {
            id: "call-shell".to_string(),
            name: "bash".to_string(),
            arguments: "echo hi there".to_string(),
        },
        ToolCall {
            id: "call-grep".to_string(),
            name: "grep".to_string(),
            arguments: r#"{"query":"needle"}"#.to_string(),
        },
    ]);

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "bash");
    assert_eq!(calls[0].arguments["command"], "echo hi there");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call-shell"));
    assert_eq!(calls[1].arguments["pattern"], "needle");
}

#[test]
fn detect_tool_call_parse_issue_reports_unclosed_tag() {
    let issue = detect_tool_call_parse_issue("<tool_call>{\"name\":\"shell\"", &[]);

    assert!(issue.is_some());
}

#[test]
fn detect_tool_call_parse_issue_ignores_empty_plain_and_valid_calls() {
    assert!(detect_tool_call_parse_issue("   ", &[]).is_none());
    assert!(detect_tool_call_parse_issue("plain answer", &[]).is_none());

    let (_, calls) =
        parse_tool_calls(r#"<tool_call>{"name":"bash","arguments":{"cmd":"pwd"}}</tool_call>"#);
    assert!(!calls.is_empty());
    assert!(detect_tool_call_parse_issue("```tool bash\n{}\n```", &calls).is_none());
}

#[test]
fn detect_tool_call_parse_issue_catches_supported_payload_markers() {
    for marker in [
        "```tool web_fetch\nnot-json\n```",
        r#"{"tool_calls": []}"#,
        r#"{"name":"shell","arguments":"bad"}"#,
        ":UIButtonType",
        "✁",
        "TOOL_CALL",
        "<FunctionCall>",
    ] {
        assert!(detect_tool_call_parse_issue(marker, &[]).is_some(), "{marker}");
    }
}

#[test]
fn detect_tool_call_parse_issue_catches_tag_and_fence_aliases() {
    for marker in [
        "<toolcall>",
        "<tool-call>",
        "```tool_call\ninvalid\n```",
        "```toolcall\ninvalid\n```",
        "```tool-call\ninvalid\n```",
        "```tool file_read\ninvalid\n```",
        "```tool shell\ninvalid\n```",
        "```tool memory_store\ninvalid\n```",
        "```tool custom\ninvalid\n```",
    ] {
        assert!(detect_tool_call_parse_issue(marker, &[]).is_some(), "{marker}");
    }
}
