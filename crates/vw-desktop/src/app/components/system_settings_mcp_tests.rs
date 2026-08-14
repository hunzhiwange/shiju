use super::{
    ConfiguredMcpServerRow, McpStatusTone, capability_preview, compact_text,
    configured_mcp_server_rows, configured_server_panel, connection_status_label_and_tone,
    empty_preview, form_field, inspect_for_server, mcp_tab_button, mcp_tab_labels,
    new_server_panel, preview_action_message, runtime_server_panel, status_badge, status_dot_color,
    toolbar_button, tools_preview_section, value_field, value_preview_section, view, view_overlays,
};
use crate::app::Message;
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::{McpNewServerTransport, McpSettingsTab};
use iced::widget::text_editor;
use serde_json::json;
use vw_gateway_client::vw_api_types::mcp::{
    McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto,
};
use vw_gateway_client::vw_api_types::tools::ToolSpecDto;

fn tool_spec(id: &str) -> ToolSpecDto {
    ToolSpecDto {
        id: id.into(),
        display_name: format!("Tool {id}"),
        description: "A very useful MCP tool".to_string(),
        input_schema: json!({"type":"object"}),
        aliases: Vec::new(),
        read_only: true,
        destructive: false,
        concurrency_safe: true,
        requires_user_interaction: false,
        strict: true,
    }
}

fn inspected_server(name: &str, status: McpConnectionStatusDto) -> McpServerInspectDto {
    McpServerInspectDto {
        name: name.to_string(),
        transport: "stdio".to_string(),
        enabled: status != McpConnectionStatusDto::Disabled,
        required: false,
        connection_status: status,
        timeout_ms: Some(30_000),
        allowed_tools: Vec::new(),
        denied_tools: Vec::new(),
        tool_count: Some(0),
        resource_count: Some(0),
        resource_template_count: Some(0),
        prompt_count: Some(0),
        tools: Vec::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
        prompts: Vec::new(),
        tools_error: None,
        resources_error: None,
        resource_templates_error: None,
        prompts_error: None,
        error: None,
    }
}

#[test]
fn compact_text_handles_unicode_exact_and_truncated_limits() {
    assert_eq!(compact_text("你好世界", 0), "...");
    assert_eq!(compact_text("你好世界", 4), "你好世界");
    assert_eq!(compact_text("你好世界", 2), "你好...");
}

#[test]
fn configured_rows_apply_defaults_and_preserve_explicit_fields() {
    let rows = configured_mcp_server_rows(
        r#"[
            {},
            {"name":" custom ","type":" http ","enabled":false,"required":true}
        ]"#,
    )
    .expect("valid server array should parse");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].index, 0);
    assert_eq!(rows[0].name, "未命名");
    assert_eq!(rows[0].transport, "stdio");
    assert!(rows[0].enabled);
    assert!(!rows[0].required);
    assert_eq!(rows[1].name, "custom");
    assert_eq!(rows[1].transport, "http");
    assert!(!rows[1].enabled);
    assert!(rows[1].required);
}

#[test]
fn configured_rows_reject_invalid_json_non_arrays_and_non_objects() {
    assert!(configured_mcp_server_rows("  ").expect("blank input is empty").is_empty());
    assert!(configured_mcp_server_rows("{").is_err());
    assert_eq!(
        configured_mcp_server_rows("{}").expect_err("object root must fail"),
        "MCP 配置必须是数组"
    );
    assert_eq!(
        configured_mcp_server_rows("[null]").expect_err("non-object entry must fail"),
        "mcpServers[0] 必须是对象"
    );
}

#[test]
fn tabs_statuses_and_preview_action_cover_all_static_mappings() {
    assert_eq!(
        mcp_tab_labels(),
        &[
            ("全局服务器", McpSettingsTab::Servers),
            ("运行时 MCP", McpSettingsTab::Runtime),
            ("高级 JSON", McpSettingsTab::Json),
        ]
    );

    for (status, expected) in [
        (McpConnectionStatusDto::Connected, ("已连通", McpStatusTone::Connected)),
        (McpConnectionStatusDto::Error, ("连接错误", McpStatusTone::Error)),
        (McpConnectionStatusDto::Disabled, ("已禁用", McpStatusTone::Disabled)),
    ] {
        let server = inspected_server("demo", status);
        assert_eq!(connection_status_label_and_tone(Some(&server), true), expected);
    }
    assert_eq!(connection_status_label_and_tone(None, true), ("未预览", McpStatusTone::Pending));
    assert_eq!(connection_status_label_and_tone(None, false), ("已禁用", McpStatusTone::Disabled));
    assert!(preview_action_message(false).is_none());
    assert!(matches!(
        preview_action_message(true),
        Some(Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh)))
    ));
}

#[test]
fn inspect_lookup_and_value_field_handle_present_missing_and_blank_values() {
    let inspect = McpInspectResponse {
        servers: vec![
            inspected_server("alpha", McpConnectionStatusDto::Connected),
            inspected_server("beta", McpConnectionStatusDto::Error),
        ],
    };
    let value = json!({"name":"  ", "title":" Useful title ", "uri":42});

    assert_eq!(
        inspect_for_server(Some(&inspect), "beta").map(|server| server.name.as_str()),
        Some("beta")
    );
    assert!(inspect_for_server(Some(&inspect), "missing").is_none());
    assert!(inspect_for_server(None, "alpha").is_none());
    assert_eq!(value_field(&value, &["name", "title", "uri"]), Some("Useful title".to_string()));
    assert_eq!(value_field(&json!(["not an object"]), &["name"]), None);
}

#[test]
fn status_and_toolbar_elements_build_for_enabled_disabled_and_active_tabs() {
    let theme = iced::Theme::Dark;
    for tone in [
        McpStatusTone::Connected,
        McpStatusTone::Error,
        McpStatusTone::Disabled,
        McpStatusTone::Pending,
    ] {
        let _ = status_dot_color(&theme, tone);
    }

    let connected = inspected_server("demo", McpConnectionStatusDto::Connected);
    let _: iced::Element<'static, Message> = status_badge(Some(&connected), true);
    let _: iced::Element<'static, Message> = status_badge(None, false);
    let _ = toolbar_button("disabled", None);
    let _ = toolbar_button(
        "refresh",
        Some(Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh))),
    );
    let _: iced::Element<'_, Message> = form_field("field", iced::widget::text("control").size(12));
    let _: iced::Element<'static, Message> =
        mcp_tab_button("JSON", McpSettingsTab::Json, McpSettingsTab::Servers);
    let _: iced::Element<'static, Message> =
        mcp_tab_button("JSON", McpSettingsTab::Json, McpSettingsTab::Json);
}

#[test]
fn preview_sections_cover_empty_error_hidden_and_disabled_branches() {
    let mut server = inspected_server("demo", McpConnectionStatusDto::Connected);
    let _: iced::Element<'static, Message> = empty_preview(true, false);
    let _: iced::Element<'static, Message> = empty_preview(false, true);
    let _: iced::Element<'static, Message> = tools_preview_section(&server);

    server.tools_error = Some("tool preview failed".repeat(20));
    let _: iced::Element<'static, Message> = tools_preview_section(&server);

    server.tools_error = None;
    server.tools = (0..10).map(|idx| tool_spec(&format!("tool-{idx}"))).collect();
    let _: iced::Element<'static, Message> = tools_preview_section(&server);

    let many_values = (0..8)
        .map(|idx| json!({"name": format!("resource-{idx}"), "uri": format!("file:///{idx}")}))
        .collect::<Vec<_>>();
    let _: iced::Element<'static, Message> =
        value_preview_section("资源", &[], None, &["name"], &["uri"]);
    let _: iced::Element<'static, Message> = value_preview_section(
        "资源",
        &many_values,
        None,
        &["name", "title"],
        &["uri", "description"],
    );
    let _: iced::Element<'static, Message> =
        value_preview_section("资源", &many_values, Some("resource error"), &["name"], &["uri"]);

    let mut disabled = inspected_server("disabled", McpConnectionStatusDto::Disabled);
    let _: iced::Element<'static, Message> = capability_preview(Some(&disabled), true, false);
    disabled.connection_status = McpConnectionStatusDto::Error;
    disabled.error = Some("server failed".repeat(12));
    let _: iced::Element<'static, Message> = capability_preview(Some(&disabled), true, true);
    let _: iced::Element<'static, Message> = capability_preview(None, true, false);
}

#[test]
fn server_panels_build_configured_runtime_and_new_server_states() {
    let (mut app, _) = crate::app::App::new();
    let mut inspect = inspected_server("demo", McpConnectionStatusDto::Connected);
    inspect.required = true;
    inspect.tool_count = Some(3);
    inspect.resource_count = Some(2);
    inspect.prompt_count = Some(1);
    inspect.resources = vec![json!({"name":"Resource","uri":"file:///tmp"})];
    inspect.resource_templates = vec![json!({"uriTemplate":"file:///{path}"})];
    inspect.prompts = vec![json!({"name":"Prompt","description":"Use it"})];

    let row = ConfiguredMcpServerRow {
        index: 0,
        name: "demo".to_string(),
        transport: "stdio".to_string(),
        enabled: true,
        required: true,
    };
    let _: iced::Element<'_, Message> = configured_server_panel(
        row.clone(),
        Some(&inspect),
        "global:demo".to_string(),
        true,
        true,
        true,
        false,
    );
    let _: iced::Element<'_, Message> =
        configured_server_panel(row, None, "global:demo".to_string(), false, false, false, true);
    let _: iced::Element<'_, Message> =
        runtime_server_panel(&inspect, "runtime:demo".to_string(), true, true, true, false);

    app.mcp_settings.new_server_expanded = false;
    let _: iced::Element<'_, Message> = new_server_panel(&app.mcp_settings, true);
    app.mcp_settings.new_server_expanded = true;
    for transport in
        [McpNewServerTransport::Stdio, McpNewServerTransport::Http, McpNewServerTransport::Sse]
    {
        app.mcp_settings.new_server.transport = transport;
        let _: iced::Element<'_, Message> = new_server_panel(&app.mcp_settings, false);
    }
}

#[test]
fn view_builds_servers_runtime_and_json_tabs_with_state_variants() {
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.editor = text_editor::Content::with_text(
        r#"[{"name":"demo","type":"stdio","enabled":true,"required":true}]"#,
    );
    app.mcp_settings.new_server_expanded = true;
    app.mcp_settings.status_message = Some("saved".to_string());
    app.mcp_settings.save_error = Some("save failed".to_string());
    app.mcp_settings.inspect_error = Some("inspect failed".to_string());
    app.mcp_settings.expanded_previews.insert("global:demo".to_string());
    app.mcp_settings.inspect = Some(McpInspectResponse {
        servers: vec![inspected_server("demo", McpConnectionStatusDto::Connected)],
    });

    for transport in
        [McpNewServerTransport::Stdio, McpNewServerTransport::Http, McpNewServerTransport::Sse]
    {
        app.mcp_settings.active_tab = McpSettingsTab::Servers;
        app.mcp_settings.new_server.transport = transport;
        assert!(!view(&app).as_widget().children().is_empty());
    }

    app.mcp_settings.active_tab = McpSettingsTab::Runtime;
    app.mcp_settings.expanded_previews.insert("runtime:demo".to_string());
    assert!(!view(&app).as_widget().children().is_empty());

    app.mcp_settings.active_tab = McpSettingsTab::Json;
    app.mcp_settings.loading = true;
    app.mcp_settings.inspect_loading = true;
    app.mcp_settings.saving = true;
    assert!(!view(&app).as_widget().children().is_empty());
}

#[test]
fn delete_confirmation_uses_shared_modal_overlay_and_only_shows_on_servers_tab() {
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.editor =
        text_editor::Content::with_text(r#"[{"name":"demo","type":"stdio","command":"npx"}]"#);
    app.mcp_settings.delete_pending = Some(0);

    let base: iced::Element<'_, Message> = iced::widget::text("base").into();
    let overlay = view_overlays(&app, base);
    assert_eq!(overlay.as_widget().children().len(), 2);
    drop(overlay);

    app.mcp_settings.active_tab = McpSettingsTab::Json;
    let base: iced::Element<'_, Message> = iced::widget::text("base").into();
    let unchanged = view_overlays(&app, base);
    assert!(unchanged.as_widget().children().is_empty());
}
