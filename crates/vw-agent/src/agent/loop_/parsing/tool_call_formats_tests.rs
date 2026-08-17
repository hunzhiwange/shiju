use super::*;

#[test]
fn raw_string_argument_hint_trims_only_non_empty_strings() {
    let raw = serde_json::json!("  run me  ");
    let empty = serde_json::json!(" \n\t ");
    let object = serde_json::json!({"command": "ls"});

    assert_eq!(raw_string_argument_hint(Some(&raw)), Some("run me"));
    assert_eq!(raw_string_argument_hint(Some(&empty)), None);
    assert_eq!(raw_string_argument_hint(Some(&object)), None);
    assert_eq!(raw_string_argument_hint(None), None);
}

#[test]
fn curl_command_accepts_http_urls_and_rejects_unsafe_shapes() {
    assert_eq!(
        build_curl_command("http://example.com/a'b"),
        Some(r#"curl -s 'http://example.com/a'\\''b'"#.to_string())
    );
    assert_eq!(build_curl_command("ftp://example.com"), None);
    assert_eq!(build_curl_command("https://example.com/a b"), None);
}

#[test]
fn shell_command_normalization_rejects_json_and_builds_curl() {
    assert_eq!(normalize_shell_command_from_raw("  'pwd'  "), Some("pwd".to_string()));
    assert_eq!(normalize_shell_command_from_raw(r#""ls -la""#), Some("ls -la".to_string()));
    assert_eq!(normalize_shell_command_from_raw("   "), None);
    assert_eq!(normalize_shell_command_from_raw(r#""""#), None);
    assert_eq!(normalize_shell_command_from_raw(r#"{"command":"ls"}"#), None);
    assert_eq!(normalize_shell_command_from_raw(r#"["ls"]"#), None);
    assert_eq!(
        normalize_shell_command_from_raw("https://example.com"),
        Some("curl -s 'https://example.com'".to_string())
    );
    assert_eq!(
        normalize_shell_command_from_raw("https://example.com/has space"),
        Some("https://example.com/has space".to_string())
    );
}

#[test]
fn shell_argument_normalization_prefers_command_then_alias_then_url_then_hint() {
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"command": "  echo ok  "}), None)["command"],
        "  echo ok  "
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"script": "npm test"}), None)["command"],
        "npm test"
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"cmd": "{}", "script": "pwd"}), None)["command"],
        "pwd"
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"url": "https://example.com/a"}), None)["command"],
        "curl -s 'https://example.com/a'"
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"http_url": "https://example.com"}), None)["command"],
        "curl -s 'https://example.com'"
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"note": "x"}), Some("  whoami  "))["command"],
        "whoami"
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!({"url": "{}"}), Some("echo fallback"),)["command"],
        "echo fallback"
    );
}

#[test]
fn shell_argument_normalization_handles_strings_other_values_and_empty_results() {
    assert_eq!(
        normalize_shell_arguments(serde_json::json!("date"), None),
        serde_json::json!({"command": "date"})
    );
    assert_eq!(
        normalize_shell_arguments(serde_json::json!(false), Some("uptime")),
        serde_json::json!({"command": "uptime"})
    );
    assert_eq!(normalize_shell_arguments(serde_json::json!("{}"), None), serde_json::json!({}));
    assert_eq!(normalize_shell_arguments(serde_json::json!(false), None), serde_json::json!({}));
}

#[test]
fn tool_argument_normalization_uses_aliases() {
    let shell = normalize_tool_arguments("bash", serde_json::json!({"cmd": "pwd"}), None);
    assert_eq!(shell["command"], "pwd");

    let grep = normalize_tool_arguments("grep", serde_json::json!({"query": "needle"}), None);
    assert_eq!(grep["pattern"], "needle");

    let glob = normalize_tool_arguments("glob", serde_json::json!("*.rs"), None);
    assert_eq!(glob["pattern"], "*.rs");

    let codesearch = normalize_tool_arguments("codesearch", serde_json::json!({}), Some("Parser"));
    assert_eq!(codesearch["query"], "Parser");

    let passthrough = normalize_tool_arguments("unknown", serde_json::json!({"input": "x"}), None);
    assert_eq!(passthrough, serde_json::json!({"input": "x"}));

    assert_eq!(
        normalize_tool_arguments("grep", serde_json::json!("  "), None),
        serde_json::json!({})
    );
    assert_eq!(
        normalize_tool_arguments(
            "glob",
            serde_json::json!({"pattern": "src/*.rs"}),
            Some("ignored")
        ),
        serde_json::json!({"pattern": "src/*.rs"})
    );
    assert_eq!(
        normalize_tool_arguments("codesearch", serde_json::json!({"input": "Symbol"}), None),
        serde_json::json!({"input": "Symbol", "query": "Symbol"})
    );
    assert_eq!(
        normalize_tool_arguments("codesearch", serde_json::json!(12), None),
        serde_json::json!({})
    );
}

#[test]
fn text_argument_normalization_prefers_primary_aliases_and_hints() {
    assert_eq!(
        normalize_tool_arguments(
            "grep",
            serde_json::json!({"pattern": "kept", "query": "ignored"}),
            Some("hint")
        ),
        serde_json::json!({"pattern": "kept", "query": "ignored"})
    );
    assert_eq!(
        normalize_tool_arguments("glob", serde_json::json!({"other": 1}), Some("*.toml")),
        serde_json::json!({"other": 1, "pattern": "*.toml"})
    );
    assert_eq!(
        normalize_tool_arguments("codesearch", serde_json::json!({"pattern": "Symbol"}), None),
        serde_json::json!({"pattern": "Symbol", "query": "Symbol"})
    );
    assert_eq!(
        normalize_tool_arguments("grep", serde_json::json!(42), Some("needle")),
        serde_json::json!({"pattern": "needle"})
    );
    assert_eq!(
        normalize_tool_arguments("grep", serde_json::json!({"input": "  needle  "}), None),
        serde_json::json!({"input": "  needle  ", "pattern": "needle"})
    );
    assert_eq!(
        normalize_tool_arguments("codesearch", serde_json::json!({"query": "  Symbol  "}), None),
        serde_json::json!({"query": "  Symbol  "})
    );
    assert_eq!(
        normalize_tool_arguments("glob", serde_json::json!({"pattern": "   "}), Some("  *.rs  ")),
        serde_json::json!({"pattern": "*.rs"})
    );
}

#[test]
fn parses_xml_attribute_tool_calls_and_skips_empty_argument_sets() {
    let calls = parse_xml_attribute_tool_calls(
        r#"
        <invoke name="bash"><parameter name="command">pwd</parameter></invoke>
        <invoke name="file_read"><parameter name="path">Cargo.toml</parameter></invoke>
        <invoke name="shell"></invoke>
        "#,
    );

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
    assert_eq!(calls[1].name, "file_read");
    assert_eq!(calls[1].arguments["path"], "Cargo.toml");
    assert!(calls[0].tool_call_id.is_none());
    assert!(parse_xml_attribute_tool_calls("<invoke name=\"shell\"></invoke>").is_empty());

    let empty_value_calls = parse_xml_attribute_tool_calls(
        r#"<invoke name="send_message"><parameter name="body"></parameter></invoke>"#,
    );
    assert_eq!(empty_value_calls.len(), 1);
    assert_eq!(empty_value_calls[0].name, "message_send");
    assert_eq!(empty_value_calls[0].arguments["body"], "");
}

#[test]
fn parses_perl_style_tool_calls_and_ignores_incomplete_blocks() {
    let calls = parse_perl_style_tool_calls(
        r#"
        TOOL_CALL
        {tool => "bash", args => {
          --command "ls -la"
          --description "List files"
        }}}
        /TOOL_CALL
        TOOL_CALL
        {args => { --command "missing tool" }}}
        /TOOL_CALL
        TOOL_CALL
        {tool => "bash", args => {}}}
        /TOOL_CALL
        TOOL_CALL
        {tool => "file_read"}}
        /TOOL_CALL
        TOOL_CALL
        {tool => "file_read", args => {
          --path "/tmp/readme.md"
        }}}
        /TOOL_CALL
        "#,
    );

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "ls -la");
    assert_eq!(calls[0].arguments["description"], "List files");
    assert_eq!(calls[1].name, "file_read");
    assert_eq!(calls[1].arguments["path"], "/tmp/readme.md");
}

#[test]
fn parses_function_call_tool_calls_and_ignores_empty_pairs() {
    let calls = parse_function_call_tool_calls(
        r#"
        <FunctionCall>
        readfile
        <code>
          path>/tmp/a.txt
          ignored
          empty>
          key>value>with-gt
        </code>
        </FunctionCall>
        <FunctionCall>
        bash
        <code> ></code>
        </FunctionCall>
        "#,
    );

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].arguments["path"], "/tmp/a.txt");
    assert_eq!(calls[0].arguments["key"], "value>with-gt");
}

#[test]
fn canonicalizes_known_tool_aliases_and_preserves_unknown_names() {
    for alias in ["shell", "bash", "sh", "exec", "command", "cmd", "browser_open"] {
        assert_eq!(canonicalize_tool_name(alias), "shell");
    }
    assert_eq!(canonicalize_tool_name("sendmessage"), "message_send");
    assert_eq!(canonicalize_tool_name("read_file"), "file_read");
    assert_eq!(canonicalize_tool_name("editfile"), "file_edit");
    assert_eq!(canonicalize_tool_name("writefile"), "file_write");
    assert_eq!(canonicalize_tool_name("list_files"), "file_list");
    assert_eq!(canonicalize_tool_name("memrecall"), "memory_recall");
    assert_eq!(canonicalize_tool_name("memstore"), "memory_store");
    assert_eq!(canonicalize_tool_name("memforget"), "memory_forget");
    assert_eq!(canonicalize_tool_name("wget"), "http_request");
    assert_eq!(canonicalize_tool_name("BASH"), "shell");
    assert_eq!(canonicalize_tool_name("ReadFile"), "file_read");
    assert_eq!(canonicalize_tool_name("CustomTool"), "CustomTool");
}

#[test]
fn parses_glm_style_lines_for_param_json_url_and_invalid_cases() {
    let calls = parse_glm_style_tool_calls(
        r#"
        bash/command>echo hi
        bash/url>https://example.com
        http/url>https://api.example.com/v1
        file_read/{"path":"/tmp/a.txt"}
        file_read/{not-json}
        file_read/not-json
        file_read/path>https://example.com/asset
        bash/command>https://valid.example.com
        invalid-tool/path>ignored
        bash/url>https://bad host
        bash/command>https://bad host
        https://plain.example.com
        "#,
    );

    assert_eq!(calls.len(), 8);
    assert_eq!(calls[0].0, "shell");
    assert_eq!(calls[0].1["command"], "echo hi");
    assert_eq!(calls[1].1["command"], "curl -s 'https://example.com'");
    assert_eq!(calls[2].0, "http_request");
    assert_eq!(
        calls[2].1,
        serde_json::json!({"url": "https://api.example.com/v1", "method": "GET"})
    );
    assert_eq!(calls[3].0, "file_read");
    assert_eq!(calls[3].1, serde_json::json!({"path": "/tmp/a.txt"}));
    assert_eq!(calls[4].1, serde_json::json!({"path": "https://example.com/asset"}));
    assert_eq!(calls[5].1["command"], "curl -s 'https://valid.example.com'");
    assert_eq!(calls[6].1["command"], "https://bad host");
    assert_eq!(calls[7].1["command"], "curl -s 'https://plain.example.com'");
    assert_eq!(calls[0].2.as_deref(), Some("bash/command>echo hi"));
}

#[test]
fn glm_style_lines_handle_http_aliases_empty_lines_and_bad_tool_names() {
    let calls = parse_glm_style_tool_calls(
        r#"

        curl/url>https://example.com/api
        browser_open/url>https://example.com/page
        web-search/query>ignored
        no_slash_here
        http/url>https://bad host
        "#,
    );

    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, "http_request");
    assert_eq!(calls[0].1, serde_json::json!({"url": "https://example.com/api", "method": "GET"}));
    assert_eq!(calls[1].0, "shell");
    assert_eq!(calls[1].1["command"], "curl -s 'https://example.com/page'");
    assert_eq!(calls[2].0, "http_request");
    assert_eq!(calls[2].1, serde_json::json!({"url": "https://bad host", "method": "GET"}));
}

#[test]
fn default_param_matches_known_tool_families() {
    assert_eq!(default_param_for_tool("bash"), "command");
    assert_eq!(default_param_for_tool("file_read"), "path");
    assert_eq!(default_param_for_tool("grep"), "pattern");
    assert_eq!(default_param_for_tool("codesearch"), "query");
    assert_eq!(default_param_for_tool("memory_forget"), "query");
    assert_eq!(default_param_for_tool("memory_store"), "content");
    assert_eq!(default_param_for_tool("WebSearch"), "query");
    assert_eq!(default_param_for_tool("BrowserOpen"), "url");
    assert_eq!(default_param_for_tool("unknown"), "input");
}

#[test]
fn glm_shortened_body_rejects_empty_and_invalid_tool_names() {
    assert!(parse_glm_shortened_body("").is_none());
    assert!(parse_glm_shortened_body("not a call").is_none());
    assert!(parse_glm_shortened_body("bad-tool>value").is_none());
    assert!(parse_glm_shortened_body(">value").is_none());
}

#[test]
fn glm_shortened_body_parses_function_and_attribute_styles() {
    let function = parse_glm_shortened_body(r#"bash(command="pwd", description="where")"#).unwrap();
    assert_eq!(function.name, "shell");
    assert_eq!(function.arguments["command"], "pwd");
    assert_eq!(function.arguments["description"], "where");

    let attrs = parse_glm_shortened_body(r#"file_read path="/tmp/a.txt" />"#).unwrap();
    assert_eq!(attrs.name, "file_read");
    assert_eq!(attrs.arguments["path"], "/tmp/a.txt");

    let fallback = parse_glm_shortened_body(r#"bash command="unterminated"#).unwrap();
    assert_eq!(fallback.name, "shell");
    assert_eq!(fallback.arguments["command"], r#"command="unterminated"#);

    assert!(parse_glm_shortened_body(r#"bash(command="pwd""#).is_none());
}

#[test]
fn glm_shortened_body_parses_function_single_value_and_punctuated_attrs() {
    let function = parse_glm_shortened_body("memory_recall(search for this)").unwrap();
    assert_eq!(function.name, "memory_recall");
    assert_eq!(function.arguments, serde_json::json!({"query": "search for this"}));

    let attrs = parse_glm_shortened_body(r#"custom_tool first="1"; second="2" />"#).unwrap();
    assert_eq!(attrs.name, "custom_tool");
    assert_eq!(attrs.arguments["first"], "1");
    assert_eq!(attrs.arguments["second"], "2");

    assert!(parse_glm_shortened_body("custom_tool>").is_none());
}

#[test]
fn glm_shortened_body_parses_yaml_multiline_values_and_booleans() {
    let call = parse_glm_shortened_body(
        r#"
        file_write>
        path: /tmp/out.txt

        overwrite: true
        dry_run: no
        content: hello
        empty:
        : ignored
        ignored
        "#,
    )
    .unwrap();

    assert_eq!(call.name, "file_write");
    assert_eq!(call.arguments["path"], "/tmp/out.txt");
    assert_eq!(call.arguments["overwrite"], true);
    assert_eq!(call.arguments["dry_run"], false);
    assert_eq!(call.arguments["content"], "hello");

    let fallback = parse_glm_shortened_body(
        r#"
        custom_tool>
        ignored
        also ignored
        "#,
    )
    .unwrap();
    assert_eq!(fallback.arguments["input"], "ignored\n        also ignored");
}

#[test]
fn glm_shortened_body_parses_single_value_for_shell_http_and_default_params() {
    let shell = parse_glm_shortened_body("bash>https://example.com").unwrap();
    assert_eq!(shell.name, "shell");
    assert_eq!(shell.arguments["command"], "curl -s 'https://example.com'");

    let http = parse_glm_shortened_body("fetch>https://api.example.com").unwrap();
    assert_eq!(http.name, "http_request");
    assert_eq!(
        http.arguments,
        serde_json::json!({"url": "https://api.example.com", "method": "GET"})
    );

    let memory = parse_glm_shortened_body("memory_store>remember this").unwrap();
    assert_eq!(memory.arguments, serde_json::json!({"content": "remember this"}));

    let unknown = parse_glm_shortened_body("custom_tool>raw").unwrap();
    assert_eq!(unknown.name, "custom_tool");
    assert_eq!(unknown.arguments, serde_json::json!({"input": "raw"}));

    let self_closing = parse_glm_shortened_body("grep>needle/>").unwrap();
    assert_eq!(self_closing.name, "grep");
    assert_eq!(self_closing.arguments, serde_json::json!({"pattern": "needle"}));

    let unsafe_shell_url = parse_glm_shortened_body("bash>https://bad host").unwrap();
    assert_eq!(unsafe_shell_url.arguments["command"], "https://bad host");

    assert!(parse_glm_shortened_body("bash>").is_none());
}

#[test]
fn shell_argument_normalization_handles_remaining_aliases_and_invalid_urls() {
    for (alias, expected) in [
        ("shell_command", "echo shell"),
        ("command_line", "echo line"),
        ("bash", "echo bash"),
        ("sh", "echo sh"),
        ("input", "echo input"),
    ] {
        let arguments = serde_json::json!({ alias: expected });

        assert_eq!(normalize_shell_arguments(arguments, None)["command"], expected);
    }

    assert_eq!(
        normalize_shell_arguments(
            serde_json::json!({"command": "   ", "url": "https://bad host"}),
            None,
        )["command"],
        "https://bad host"
    );
}

#[test]
fn glm_style_lines_keep_json_payloads_and_unknown_tool_names() {
    let calls = parse_glm_style_tool_calls(
        r#"
        custom_tool/input>value
        bash/{"cmd":"pwd"}
        "#,
    );

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "custom_tool");
    assert_eq!(calls[0].1, serde_json::json!({"input": "value"}));
    assert_eq!(calls[1].0, "shell");
    assert_eq!(calls[1].1, serde_json::json!({"cmd": "pwd"}));
}

#[test]
fn glm_shortened_body_handles_attribute_parser_edges() {
    let partial =
        parse_glm_shortened_body(r#"custom_tool first="1" second="unterminated"#).unwrap();
    assert_eq!(partial.name, "custom_tool");
    assert_eq!(partial.arguments, serde_json::json!({"first": "1"}));

    let empty_key = parse_glm_shortened_body(r#"custom_tool ="x" />"#).unwrap();
    assert_eq!(empty_key.arguments, serde_json::json!({"input": r#"="x""#}));
}
