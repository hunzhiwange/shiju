use super::*;

#[test]
fn imports_codex_toml_and_cursor_or_claude_json_as_disabled_drafts() {
    let mut state = State::default();
    let codex = r#"
[mcp_servers.docs]
url = "https://example.com/mcp"
enabled = true
startup_timeout_sec = 15
[mcp_servers.docs.http_headers]
"X-Client" = "demo"
"#;
    let json = r#"{"mcpServers":{
        "local":{"command":"example","args":["serve"],"env":{"LANG":"en"}},
        "docs":{"type":"sse","url":"https://example.net/mcp"}
    }}"#;

    assert_eq!(stage_import(&mut state, codex).unwrap(), (1, 0));
    assert_eq!(stage_import(&mut state, json).unwrap(), (1, 1));
    assert_eq!(state.active_tab, McpSettingsTab::Json);
    let (servers, count) = editor_value(&state.editor).unwrap();
    assert_eq!(count, 2);
    assert_eq!(
        servers,
        json!([
            {
                "name": "docs", "type": "http", "url": "https://example.com/mcp",
                "enabled": false, "timeout": 15,
                "headers": [{"name": "X-Client", "value": "demo"}]
            },
            {
                "name": "local", "type": "stdio", "command": "example",
                "args": ["serve"], "enabled": false,
                "env": [{"name": "LANG", "value": "en"}]
            }
        ]),
    );
}

#[test]
fn backup_round_trip_keeps_server_fields_and_invalid_import_keeps_editor() {
    let mut original = State::default();
    let source = r#"[{"name":"native","type":"stdio","command":"example",
        "enabled":true,"required":false,"allowedTools":["read"],"_meta":{"label":"demo"}}]"#;
    stage_import(&mut original, source).unwrap();

    let mut restored = State::default();
    let backup = backup(&original).unwrap();
    assert_eq!(stage_import(&mut restored, &backup).unwrap(), (1, 0));
    assert_eq!(restored.editor.text(), original.editor.text());

    let unsupported = r#"{"mcpServers":{"other":{
        "url":"https://example.com/mcp","bearer_token_env_var":"MCP_TOKEN"
    }}}"#;
    assert!(stage_import(&mut restored, unsupported).is_err());
    assert_eq!(restored.editor.text(), original.editor.text());
}
