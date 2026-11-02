use super::*;
use serde_json::json;

#[test]
fn validate_mcp_servers_accepts_supported_transports() {
    let value = json!([
        {"name": "local", "type": "stdio", "command": "npx", "args": ["tool"], "env": [{"name": "TOKEN", "value": "x"}]},
        {"name": "http", "type": "http", "url": "https://example.test/mcp", "headers": [{"name": "Authorization", "value": "Bearer x"}]},
        {"name": "sse", "type": "sse", "url": "https://example.test/sse", "allowedTools": ["search"], "timeout": 15}
    ]);

    assert_eq!(validate_mcp_servers(&value).expect("valid"), 3);
}

#[test]
fn validate_mcp_servers_rejects_invalid_shape() {
    let value = json!([{"name": "broken", "type": "http"}]);

    let error = validate_mcp_servers(&value).expect_err("invalid");
    assert!(error.to_string().contains("mcpServers[0] in global config.url"));
}

#[test]
fn set_server_enabled_updates_selected_server_only() {
    let editor = text_editor::Content::with_text(
        r#"[
              {"name": "first", "command": "npx", "enabled": true},
              {"name": "second", "command": "node", "enabled": false}
            ]"#,
    );

    let (value, count) = set_server_enabled(&editor, 1, true).expect("updated");

    assert_eq!(count, 2);
    assert_eq!(value[0]["enabled"], true);
    assert_eq!(value[1]["enabled"], true);
}

#[test]
fn set_runtime_server_enabled_appends_override() {
    let overrides = set_runtime_server_enabled(&[], "plugin-mcp", false).expect("updated");

    assert_eq!(overrides, vec![json!({"name": "plugin-mcp", "enabled": false})]);
}

#[test]
fn set_runtime_server_enabled_updates_existing_override_only() {
    let raw = vec![
        json!({"name": "plugin-mcp", "enabled": false, "required": true}),
        json!({"name": "other", "enabled": false}),
    ];

    let overrides = set_runtime_server_enabled(&raw, "plugin-mcp", true).expect("updated");

    assert_eq!(overrides[0]["enabled"], true);
    assert_eq!(overrides[0]["required"], true);
    assert_eq!(overrides[1]["enabled"], false);
}

#[test]
fn append_new_server_adds_stdio_server() {
    let editor = text_editor::Content::with_text("[]");
    let form = McpNewServerState {
        name_input: "filesystem".to_string(),
        command_input: "npx".to_string(),
        args_input: "-y\n@modelcontextprotocol/server-filesystem\n/tmp".to_string(),
        ..McpNewServerState::default()
    };

    let (value, count) = append_new_server(&editor, &form).expect("appended");

    assert_eq!(count, 1);
    assert_eq!(value[0]["name"], json!("filesystem"));
    assert_eq!(value[0]["type"], json!("stdio"));
    assert_eq!(value[0]["command"], json!("npx"));
    assert_eq!(value[0]["args"], json!(["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]));
    assert_eq!(value[0]["enabled"], json!(true));
    assert_eq!(value[0]["timeout"], json!(30.0));
}

#[test]
fn append_new_server_adds_http_server_without_empty_timeout() {
    let editor = text_editor::Content::with_text(r#"[{"name": "local", "command": "npx"}]"#);
    let form = McpNewServerState {
        name_input: "remote".to_string(),
        transport: McpNewServerTransport::Http,
        url_input: "https://example.test/mcp".to_string(),
        timeout_input: String::new(),
        enabled: false,
        required: true,
        ..McpNewServerState::default()
    };

    let (value, count) = append_new_server(&editor, &form).expect("appended");

    assert_eq!(count, 2);
    assert_eq!(value[1]["name"], json!("remote"));
    assert_eq!(value[1]["type"], json!("http"));
    assert_eq!(value[1]["url"], json!("https://example.test/mcp"));
    assert_eq!(value[1]["enabled"], json!(false));
    assert_eq!(value[1]["required"], json!(true));
    assert!(value[1].get("timeout").is_none());
}

#[test]
fn append_new_server_rejects_missing_stdio_command() {
    let editor = text_editor::Content::with_text("[]");
    let form = McpNewServerState {
        name_input: "broken".to_string(),
        command_input: " ".to_string(),
        ..McpNewServerState::default()
    };

    let error = append_new_server(&editor, &form).expect_err("invalid");

    assert!(error.to_string().contains("new MCP stdio command"));
}
