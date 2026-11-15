use super::*;
use serde_json::json;
#[test]
fn validate_mcp_servers_reports_precise_paths_for_invalid_common_fields() {
    let cases = [
        (json!({}), "mcpServers in global config"),
        (json!([null]), "mcpServers[0] in global config"),
        (json!([{"name": " ", "command": "npx"}]), "mcpServers[0] in global config.name"),
        (
            json!([{"name": "local", "command": "npx", "enabled": "yes"}]),
            "mcpServers[0] in global config.enabled",
        ),
        (
            json!([{"name": "local", "command": "npx", "timeout": 0}]),
            "mcpServers[0] in global config.timeout",
        ),
        (
            json!([{"name": "local", "command": "npx", "allowedTools": [""]}]),
            "mcpServers[0] in global config.allowedTools[0]",
        ),
        (
            json!([{"name": "local", "command": "npx", "_meta": []}]),
            "mcpServers[0] in global config._meta",
        ),
        (
            json!([{"name": "remote", "type": "websocket", "url": "https://example.test"}]),
            "mcpServers[0] in global config.type",
        ),
    ];

    for (value, expected_path) in cases {
        let error = validate_mcp_servers(&value).expect_err("invalid server must be rejected");
        assert!(error.to_string().contains(expected_path), "{error}");
    }
}

#[test]
fn validate_mcp_servers_reports_precise_paths_for_transport_specific_fields() {
    let cases = [
        (
            json!([{"name": "local", "type": "stdio", "command": "npx", "args": "tool"}]),
            "mcpServers[0] in global config.args",
        ),
        (
            json!([{"name": "local", "type": "stdio", "command": "npx", "args": [3]}]),
            "mcpServers[0] in global config.args[0]",
        ),
        (
            json!([{"name": "local", "type": "stdio", "command": "npx", "env": [{"name": "TOKEN", "value": ""}]}]),
            "mcpServers[0] in global config.env[0].value",
        ),
        (
            json!([{"name": "remote", "type": "http", "url": "https://example.test", "headers": [null]}]),
            "mcpServers[0] in global config.headers[0]",
        ),
        (
            json!([{"name": "remote", "type": "sse", "url": "https://example.test", "headers": [{"name": "", "value": "x"}]}]),
            "mcpServers[0] in global config.headers[0].name",
        ),
    ];

    for (value, expected_path) in cases {
        let error = validate_mcp_servers(&value).expect_err("invalid server must be rejected");
        assert!(error.to_string().contains(expected_path), "{error}");
    }
}

#[test]
fn validate_mcp_server_overrides_accepts_full_override_and_rejects_invalid_fields() {
    let valid = vec![json!({
        "name": "runtime",
        "enabled": true,
        "required": false,
        "timeout": null,
        "allowedTools": ["search"],
        "deniedTools": ["delete"]
    })];
    validate_mcp_server_overrides(&valid).expect("valid override");

    let cases = [
        (vec![json!(null)], "mcpServerOverrides[0] in global config"),
        (
            vec![json!({"name": "runtime", "required": "yes"})],
            "mcpServerOverrides[0] in global config.required",
        ),
        (
            vec![json!({"name": "runtime", "timeout": -1})],
            "mcpServerOverrides[0] in global config.timeout",
        ),
        (
            vec![json!({"name": "runtime", "deniedTools": [" "]})],
            "mcpServerOverrides[0] in global config.deniedTools[0]",
        ),
    ];

    for (overrides, expected_path) in cases {
        let error = validate_mcp_server_overrides(&overrides)
            .expect_err("invalid override must be rejected");
        assert!(error.to_string().contains(expected_path), "{error}");
    }
}

#[test]
fn optional_timeout_accepts_empty_and_positive_values_and_rejects_unsafe_values() {
    assert_eq!(optional_timeout("  ").expect("empty timeout"), None);
    assert_eq!(optional_timeout(" 2.5 ").expect("positive timeout"), Some(json!(2.5)));

    for input in ["0", "-1", "NaN", "inf", "1e999", "not-a-number"] {
        let error = optional_timeout(input).expect_err("invalid timeout must be rejected");
        assert!(error.to_string().contains("new MCP server timeout"));
    }
}

#[test]
fn editor_value_treats_blank_as_empty_and_rejects_malformed_json() {
    let blank = text_editor::Content::with_text(" \n ");
    let (value, count) = editor_value(&blank).expect("blank editor");
    assert_eq!(value, json!([]));
    assert_eq!(count, 0);

    let malformed = text_editor::Content::with_text("[");
    let error = editor_value(&malformed).expect_err("malformed JSON must be rejected");
    assert!(error.to_string().contains("JSON 解析失败"));
}

#[test]
fn new_server_value_trims_sse_fields_and_requires_remote_url() {
    let form = McpNewServerState {
        name_input: " events ".to_string(),
        transport: McpNewServerTransport::Sse,
        url_input: " https://example.test/sse ".to_string(),
        timeout_input: " 5 ".to_string(),
        enabled: false,
        required: true,
        ..McpNewServerState::default()
    };

    let value = new_server_value(&form).expect("valid SSE server");
    assert_eq!(
        value,
        json!({
            "name": "events",
            "type": "sse",
            "enabled": false,
            "required": true,
            "timeout": 5.0,
            "url": "https://example.test/sse"
        })
    );

    let missing_url = McpNewServerState {
        name_input: "events".to_string(),
        transport: McpNewServerTransport::Sse,
        url_input: " ".to_string(),
        ..McpNewServerState::default()
    };
    assert!(
        new_server_value(&missing_url)
            .expect_err("missing URL must be rejected")
            .to_string()
            .contains("new MCP server url")
    );
}

#[test]
fn loaded_helpers_default_missing_or_wrong_typed_sections() {
    let config = json!({"mcpServers": "wrong", "mcpServerOverrides": {}});

    assert_eq!(loaded_mcp_servers(config.clone()), json!("wrong"));
    assert!(loaded_mcp_server_overrides(&config).is_empty());
    assert_eq!(loaded_mcp_servers(json!({})), json!([]));
}

#[test]
fn set_server_enabled_rejects_missing_index_without_changing_source() {
    let editor = text_editor::Content::with_text(r#"[{"name":"local","command":"npx"}]"#);

    let error = set_server_enabled(&editor, 2, false).expect_err("missing server index");

    assert!(error.to_string().contains("mcpServers[2] in global config"));
    assert!(!editor.text().contains("enabled"));
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn delete_server_removes_selected_server_and_recounts() {
    let editor = text_editor::Content::with_text(
        r#"[
            {"name":"first","command":"npx"},
            {"name":"second","command":"node"},
            {"name":"third","command":"bun"}
        ]"#,
    );

    let (value, count) = delete_server(&editor, 1).expect("selected server should be deleted");

    assert_eq!(count, 2);
    assert_eq!(value[0]["name"], "first");
    assert_eq!(value[1]["name"], "third");
}

#[test]
fn delete_server_rejects_missing_index_without_changing_source() {
    let editor = text_editor::Content::with_text(r#"[{"name":"local","command":"npx"}]"#);

    let error = delete_server(&editor, 1).expect_err("missing server index");

    assert!(error.to_string().contains("mcpServers[1] in global config"));
    assert!(editor.text().contains("local"));
}

#[test]
fn set_runtime_server_enabled_trims_names_and_rejects_invalid_existing_overrides() {
    let overrides = set_runtime_server_enabled(&[], " plugin-mcp ", true).expect("new override");
    assert_eq!(overrides, vec![json!({"name": "plugin-mcp", "enabled": true})]);

    let error = set_runtime_server_enabled(&[json!({"name": "", "enabled": true})], "other", false)
        .expect_err("invalid existing override must be rejected");
    assert!(error.to_string().contains("mcpServerOverrides[0] in global config.name"));

    assert!(
        set_runtime_server_enabled(&[], " ", true)
            .expect_err("blank name must be rejected")
            .to_string()
            .contains("runtime MCP server name")
    );
}

#[test]
fn server_edit_state_populates_form_from_existing_server() {
    let editor = text_editor::Content::with_text(
        r#"[{
            "name":"remote",
            "type":"http",
            "url":"https://example.test/mcp",
            "timeout":12,
            "enabled":false,
            "required":true
        }]"#,
    );

    let state = server_edit_state(&editor, 0).expect("existing server should populate edit form");

    assert_eq!(state.name_input, "remote");
    assert_eq!(state.transport, McpNewServerTransport::Http);
    assert_eq!(state.url_input, "https://example.test/mcp");
    assert_eq!(state.timeout_input, "12");
    assert!(!state.enabled);
    assert!(state.required);
}

#[test]
fn update_server_preserves_advanced_fields_for_unchanged_transport() {
    let editor = text_editor::Content::with_text(
        r#"[{
            "name":"remote",
            "type":"http",
            "url":"https://old.test/mcp",
            "headers":[{"name":"Authorization","value":"Bearer secret"}],
            "allowedTools":["read"]
        }]"#,
    );
    let input = McpNewServerState {
        name_input: "renamed".to_string(),
        transport: McpNewServerTransport::Http,
        url_input: "https://new.test/mcp".to_string(),
        timeout_input: String::new(),
        ..McpNewServerState::default()
    };

    let (value, count) = update_server(&editor, 0, &input).expect("server edit should be valid");
    let server = &value[0];

    assert_eq!(count, 1);
    assert_eq!(server["name"], "renamed");
    assert_eq!(server["url"], "https://new.test/mcp");
    assert_eq!(server["headers"], json!([{"name":"Authorization","value":"Bearer secret"}]));
    assert_eq!(server["allowedTools"], json!(["read"]));
    assert!(server.get("timeout").is_none());
}
