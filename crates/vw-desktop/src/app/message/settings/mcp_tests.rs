use super::*;
use crate::app::App;
use crate::app::state::McpSettingsTab;
use serde_json::json;

fn app() -> App {
    App::new().0
}

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
        assert!(error.contains(expected_path), "{error}");
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
        assert!(error.contains(expected_path), "{error}");
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
        assert!(error.contains(expected_path), "{error}");
    }
}

#[test]
fn optional_timeout_accepts_empty_and_positive_values_and_rejects_unsafe_values() {
    assert_eq!(optional_timeout("  ").expect("empty timeout"), None);
    assert_eq!(optional_timeout(" 2.5 ").expect("positive timeout"), Some(json!(2.5)));

    for input in ["0", "-1", "NaN", "inf", "1e999", "not-a-number"] {
        let error = optional_timeout(input).expect_err("invalid timeout must be rejected");
        assert!(error.contains("new MCP server timeout"));
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
    assert!(error.contains("JSON 解析失败"));
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

    assert!(error.contains("mcpServers[2] in global config"));
    assert!(!editor.text().contains("enabled"));
}

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

    assert!(error.contains("mcpServers[1] in global config"));
    assert!(editor.text().contains("local"));
}

#[test]
fn set_runtime_server_enabled_trims_names_and_rejects_invalid_existing_overrides() {
    let overrides = set_runtime_server_enabled(&[], " plugin-mcp ", true).expect("new override");
    assert_eq!(overrides, vec![json!({"name": "plugin-mcp", "enabled": true})]);

    let error = set_runtime_server_enabled(&[json!({"name": "", "enabled": true})], "other", false)
        .expect_err("invalid existing override must be rejected");
    assert!(error.contains("mcpServerOverrides[0] in global config.name"));

    assert!(
        set_runtime_server_enabled(&[], " ", true)
            .expect_err("blank name must be rejected")
            .contains("runtime MCP server name")
    );
}

#[test]
fn loaded_message_populates_servers_and_overrides_or_reports_invalid_servers() {
    let mut app = app();
    app.mcp_settings.loading = true;
    app.mcp_settings.save_error = Some("old".to_string());
    let config = json!({
        "mcpServers": [{"name": "local", "command": "npx"}],
        "mcpServerOverrides": [{"name": "local", "enabled": false}]
    });

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::Loaded(Ok(config))));

    assert!(!app.mcp_settings.loading);
    assert_eq!(app.mcp_settings.server_count, 1);
    assert_eq!(app.mcp_settings.runtime_overrides.len(), 1);
    assert!(app.mcp_settings.editor.text().contains("local"));
    assert_eq!(app.mcp_settings.save_error, None);

    let _ =
        update(&mut app, SettingsMessage::Mcp(McpMessage::Loaded(Ok(json!({"mcpServers": {}})))));
    assert_eq!(app.mcp_settings.server_count, 0);
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|value| { value.contains("mcpServers in global config") })
    );
}

#[test]
fn input_preview_and_format_messages_apply_local_state_transitions() {
    let mut app = app();
    app.mcp_settings.save_error = Some("old".to_string());
    app.mcp_settings.status_message = Some("saved".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::NewServerNameChanged("local".to_string())),
    );
    assert_eq!(app.mcp_settings.new_server.name_input, "local");
    assert_eq!(app.mcp_settings.save_error, None);
    assert_eq!(app.mcp_settings.status_message, None);

    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::PreviewExpandedToggled("server:local".to_string())),
    );
    assert!(app.mcp_settings.expanded_previews.contains("server:local"));
    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::PreviewExpandedToggled("server:local".to_string())),
    );
    assert!(!app.mcp_settings.expanded_previews.contains("server:local"));

    app.mcp_settings.editor = text_editor::Content::with_text("[");
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::Format));
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|value| { value.contains("JSON 解析失败") })
    );
}

#[test]
fn navigation_refresh_and_inspect_messages_update_loading_and_error_state() {
    let mut app = app();
    app.mcp_settings.active_tab = McpSettingsTab::Servers;
    app.mcp_settings.save_error = Some("old save".to_string());
    app.mcp_settings.inspect_error = Some("old inspect".to_string());
    app.mcp_settings.status_message = Some("saved".to_string());

    let _ =
        update(&mut app, SettingsMessage::Mcp(McpMessage::TabSelected(McpSettingsTab::Runtime)));
    assert_eq!(app.mcp_settings.active_tab, McpSettingsTab::Runtime);

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::Refresh));
    assert!(app.mcp_settings.loading);
    assert!(app.mcp_settings.inspect_loading);
    assert_eq!(app.mcp_settings.save_error, None);
    assert_eq!(app.mcp_settings.inspect_error, None);
    assert_eq!(app.mcp_settings.status_message, None);

    app.mcp_settings.inspect_loading = false;
    app.mcp_settings.inspect_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::InspectRefresh));
    assert!(app.mcp_settings.inspect_loading);
    assert_eq!(app.mcp_settings.inspect_error, None);

    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::Inspected(Err("inspect failed".to_string()))),
    );
    assert!(!app.mcp_settings.inspect_loading);
    assert_eq!(app.mcp_settings.inspect_error.as_deref(), Some("inspect failed"));
}

#[test]
fn template_panel_and_form_messages_update_local_new_server_state() {
    let mut app = app();
    app.mcp_settings.new_server_expanded = false;
    app.mcp_settings.save_error = Some("old".to_string());
    app.mcp_settings.status_message = Some("saved".to_string());

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::NewServerPanelToggled));
    assert!(app.mcp_settings.new_server_expanded);

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::InsertTemplate));
    assert_eq!(app.mcp_settings.server_count, 1);
    assert!(app.mcp_settings.editor.text().contains("local-demo"));
    assert_eq!(app.mcp_settings.save_error, None);
    assert_eq!(app.mcp_settings.status_message, None);

    let messages = [
        McpMessage::NewServerTransportChanged(McpNewServerTransport::Http),
        McpMessage::NewServerCommandChanged("npx".to_string()),
        McpMessage::NewServerArgsChanged("-y, tool".to_string()),
        McpMessage::NewServerUrlChanged("https://example.test/mcp".to_string()),
        McpMessage::NewServerTimeoutChanged("12".to_string()),
        McpMessage::NewServerEnabledToggled(false),
        McpMessage::NewServerRequiredToggled(true),
    ];
    for message in messages {
        app.mcp_settings.save_error = Some("stale".to_string());
        let _ = update(&mut app, SettingsMessage::Mcp(message));
        assert_eq!(app.mcp_settings.save_error, None);
    }

    assert_eq!(app.mcp_settings.new_server.transport, McpNewServerTransport::Http);
    assert_eq!(app.mcp_settings.new_server.command_input, "npx");
    assert_eq!(app.mcp_settings.new_server.args_input, "-y, tool");
    assert_eq!(app.mcp_settings.new_server.url_input, "https://example.test/mcp");
    assert_eq!(app.mcp_settings.new_server.timeout_input, "12");
    assert!(!app.mcp_settings.new_server.enabled);
    assert!(app.mcp_settings.new_server.required);
}

#[test]
fn save_and_toggle_messages_set_saving_or_report_validation_errors() {
    let mut app = app();
    app.mcp_settings.editor =
        text_editor::Content::with_text(r#"[{"name":"local","command":"npx"}]"#);

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEnabledToggled(0, false)));
    assert!(app.mcp_settings.saving);
    assert_eq!(app.mcp_settings.server_count, 1);
    assert!(app.mcp_settings.editor.text().contains(r#""enabled": false"#));

    app.mcp_settings.saving = false;
    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::RuntimeServerEnabledToggled(
            " runtime ".to_string(),
            true,
        )),
    );
    assert!(app.mcp_settings.saving);
    assert_eq!(
        app.mcp_settings.runtime_overrides,
        vec![json!({"name": "runtime", "enabled": true})]
    );

    app.mcp_settings.saving = false;
    app.mcp_settings.editor = text_editor::Content::with_text("[");
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::Save));
    assert!(!app.mcp_settings.saving);
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|value| { value.contains("JSON 解析失败") })
    );

    let _ = update(
        &mut app,
        SettingsMessage::Mcp(McpMessage::RuntimeServerEnabledToggled(" ".to_string(), false)),
    );
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|value| { value.contains("runtime MCP server name") })
    );
}

#[test]
fn add_server_message_persists_valid_form_and_keeps_invalid_form_for_correction() {
    let mut app = app();
    app.mcp_settings.editor = text_editor::Content::with_text("[]");
    app.mcp_settings.new_server = McpNewServerState {
        name_input: "remote".to_string(),
        transport: McpNewServerTransport::Http,
        url_input: "https://example.test/mcp".to_string(),
        timeout_input: String::new(),
        enabled: false,
        required: true,
        ..McpNewServerState::default()
    };

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::AddServer));
    assert!(app.mcp_settings.saving);
    assert_eq!(app.mcp_settings.server_count, 1);
    assert!(app.mcp_settings.editor.text().contains(r#""name": "remote""#));
    assert_eq!(app.mcp_settings.new_server.name_input, "");
    assert_eq!(app.mcp_settings.save_error, None);

    app.mcp_settings.saving = false;
    app.mcp_settings.new_server = McpNewServerState {
        name_input: "broken".to_string(),
        transport: McpNewServerTransport::Http,
        url_input: " ".to_string(),
        ..McpNewServerState::default()
    };

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::AddServer));
    assert!(!app.mcp_settings.saving);
    assert_eq!(app.mcp_settings.new_server.name_input, "broken");
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|value| { value.contains("new MCP server url") })
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

#[test]
fn edit_messages_open_save_and_cancel_single_server_form() {
    let mut app = app();
    app.mcp_settings.editor = text_editor::Content::with_text(
        r#"[{"name":"local","type":"stdio","command":"npx","env":[{"name":"A","value":"B"}]}]"#,
    );

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEditRequested(0)));
    assert_eq!(app.mcp_settings.editing_server_index, Some(0));
    assert!(app.mcp_settings.new_server_expanded);
    assert_eq!(app.mcp_settings.new_server.name_input, "local");

    app.mcp_settings.new_server.name_input = "renamed".to_string();
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::SaveServerEdit));
    assert!(app.mcp_settings.saving);
    assert_eq!(app.mcp_settings.editing_server_index, None);
    assert!(app.mcp_settings.editor.text().contains(r#""name": "renamed""#));
    assert!(app.mcp_settings.editor.text().contains(r#""env""#));

    app.mcp_settings.saving = false;
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEditRequested(0)));
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ServerEditCanceled));
    assert_eq!(app.mcp_settings.editing_server_index, None);
    assert_eq!(app.mcp_settings.new_server.name_input, "");
}

#[test]
fn delete_confirmation_requires_matching_target_and_adjusts_edit_index() {
    let mut app = app();
    app.mcp_settings.editor = text_editor::Content::with_text(
        r#"[
            {"name":"first","command":"npx"},
            {"name":"second","command":"node"},
            {"name":"third","command":"bun"}
        ]"#,
    );
    app.mcp_settings.editing_server_index = Some(2);

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::RequestDelete(1)));
    assert_eq!(app.mcp_settings.delete_pending, Some(1));

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ConfirmDelete(0)));
    assert_eq!(app.mcp_settings.delete_pending, None);
    assert!(app.mcp_settings.editor.text().contains("second"));

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::RequestDelete(1)));
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ConfirmDelete(1)));

    assert!(app.mcp_settings.saving);
    assert_eq!(app.mcp_settings.server_count, 2);
    assert!(!app.mcp_settings.editor.text().contains("second"));
    assert_eq!(app.mcp_settings.editing_server_index, Some(1));
}

#[test]
fn deleting_edited_server_clears_stale_form_state_and_cancel_keeps_editor() {
    let mut app = app();
    app.mcp_settings.editor = text_editor::Content::with_text(
        r#"[{"name":"first","command":"npx"},{"name":"second","command":"node"}]"#,
    );
    app.mcp_settings.editing_server_index = Some(1);
    app.mcp_settings.new_server_expanded = true;
    app.mcp_settings.new_server.name_input = "second".to_string();

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::RequestDelete(1)));
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::CancelDelete));
    assert_eq!(app.mcp_settings.delete_pending, None);
    assert!(app.mcp_settings.editor.text().contains("second"));

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::RequestDelete(1)));
    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ConfirmDelete(1)));

    assert_eq!(app.mcp_settings.editing_server_index, None);
    assert!(!app.mcp_settings.new_server_expanded);
    assert_eq!(app.mcp_settings.new_server.name_input, "");
}

#[test]
fn delete_request_rejects_invalid_index_and_confirm_without_pending_is_noop() {
    let mut app = app();
    app.mcp_settings.editor =
        text_editor::Content::with_text(r#"[{"name":"local","command":"npx"}]"#);

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::RequestDelete(1)));
    assert_eq!(app.mcp_settings.delete_pending, None);
    assert!(
        app.mcp_settings
            .save_error
            .as_deref()
            .is_some_and(|error| { error.contains("mcpServers[1] in global config") })
    );

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ConfirmDelete(0)));
    assert!(!app.mcp_settings.saving);
    assert!(app.mcp_settings.editor.text().contains("local"));
}

#[test]
fn saved_message_distinguishes_success_refresh_from_failure() {
    let mut app = app();
    app.mcp_settings.saving = true;
    app.mcp_settings.inspect_error = Some("old inspect".to_string());

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::Saved(Ok(()))));
    assert!(!app.mcp_settings.saving);
    assert!(app.mcp_settings.loading && app.mcp_settings.inspect_loading);
    assert_eq!(app.mcp_settings.inspect_error, None);
    assert_eq!(app.mcp_settings.status_message.as_deref(), Some("MCP 配置已保存"));

    app.mcp_settings.saving = true;
    app.mcp_settings.loading = false;
    let _ =
        update(&mut app, SettingsMessage::Mcp(McpMessage::Saved(Err("save failed".to_string()))));
    assert!(!app.mcp_settings.saving);
    assert!(!app.mcp_settings.loading);
    assert_eq!(app.mcp_settings.save_error.as_deref(), Some("save failed"));
}
