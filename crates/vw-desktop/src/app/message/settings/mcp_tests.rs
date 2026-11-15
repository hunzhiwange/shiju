// 验证 MCP 服务配置、传输字段与覆盖项的精确校验。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::McpSettingsTab;
use serde_json::json;

fn app() -> App {
    App::new().0
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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
            .is_some_and(|error| { error.to_string().contains("mcpServers[1] in global config") })
    );

    let _ = update(&mut app, SettingsMessage::Mcp(McpMessage::ConfirmDelete(0)));
    assert!(!app.mcp_settings.saving);
    assert!(app.mcp_settings.editor.text().contains("local"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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
