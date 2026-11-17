use super::*;
use serde_json::json;

#[test]
fn compact_text_handles_unicode_exact_and_truncated_limits() {
    assert_eq!(compact_text("你好世界", 0), "...");
    assert_eq!(compact_text("你好世界", 4), "你好世界");
    assert_eq!(compact_text("你好世界", 2), "你好...");
}

#[test]
fn configured_rows_apply_defaults_and_preserve_explicit_fields() {
    let rows = configured_mcp_server_rows(
        config_ui::DesktopLocale::SimplifiedChinese,
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
    assert!(
        configured_mcp_server_rows(config_ui::DesktopLocale::SimplifiedChinese, "  ")
            .expect("blank input is empty")
            .is_empty()
    );
    assert!(
        configured_mcp_server_rows(config_ui::DesktopLocale::SimplifiedChinese, "{")
            .is_err()
    );
    assert_eq!(
        configured_mcp_server_rows(config_ui::DesktopLocale::SimplifiedChinese, "{}")
            .expect_err("object root must fail"),
        "MCP 配置必须是数组"
    );
    assert_eq!(
        configured_mcp_server_rows(config_ui::DesktopLocale::SimplifiedChinese, "[null]")
            .expect_err("non-object entry must fail"),
        "mcpServers[0] 必须是对象"
    );
}


use crate::page::{McpMessage, mcp_tab_labels, mcp_tab_button, preview_action_message, empty_preview, capability_preview};
use crate::state::McpSettingsTab;
use vw_api_tools::{ToolExposure, ToolIdentity, ToolSource, ToolSpecDto};
type Message = McpMessage;
fn identity(message: McpMessage) -> McpMessage { message }
// MCP fixture 只填充展示与状态匹配所需字段，连接行为不在 UI 单测中模拟。
fn tool_spec(id: &str) -> ToolSpecDto {
    ToolSpecDto {
        identity: ToolIdentity { namespace: None, name: id.into() },
        source: ToolSource::Builtin,
        exposure: ToolExposure::Direct,
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
    assert!(preview_action_message(identity, false).is_none());
    assert!(matches!(
        preview_action_message(identity, true),
        Some(McpMessage::InspectRefresh)
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
    let _: iced::Element<'static, Message> =
        status_badge(config_ui::DesktopLocale::SimplifiedChinese, Some(&connected), true);
    let _: iced::Element<'static, Message> =
        status_badge(config_ui::DesktopLocale::SimplifiedChinese, None, false);
    let _ = toolbar_button::<Message>("disabled", None);
    let _ = toolbar_button(
        "refresh",
        Some(McpMessage::Refresh),
    );
    let _: iced::Element<'_, Message> = form_field("field", iced::widget::text("control").size(12));
    let _: iced::Element<'static, Message> = mcp_tab_button(identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        "JSON",
        McpSettingsTab::Json,
        McpSettingsTab::Servers,
    );
    let _: iced::Element<'static, Message> = mcp_tab_button(identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        "JSON",
        McpSettingsTab::Json,
        McpSettingsTab::Json,
    );
}


#[test]
fn preview_sections_cover_empty_error_hidden_and_disabled_branches() {
    let mut server = inspected_server("demo", McpConnectionStatusDto::Connected);
    let _: iced::Element<'static, Message> =
        empty_preview(identity, config_ui::DesktopLocale::SimplifiedChinese, true, false);
    let _: iced::Element<'static, Message> =
        empty_preview(identity, config_ui::DesktopLocale::SimplifiedChinese, false, true);
    let _: iced::Element<'static, Message> =
        tools_preview_section(config_ui::DesktopLocale::SimplifiedChinese, &server);

    server.tools_error = Some("tool preview failed".repeat(20));
    let _: iced::Element<'static, Message> =
        tools_preview_section(config_ui::DesktopLocale::SimplifiedChinese, &server);

    server.tools_error = None;
    server.tools = (0..10).map(|idx| tool_spec(&format!("tool-{idx}"))).collect();
    let _: iced::Element<'static, Message> =
        tools_preview_section(config_ui::DesktopLocale::SimplifiedChinese, &server);

    let many_values = (0..8)
        .map(|idx| json!({"name": format!("resource-{idx}"), "uri": format!("file:///{idx}")}))
        .collect::<Vec<_>>();
    let _: iced::Element<'static, Message> = value_preview_section(
        config_ui::DesktopLocale::SimplifiedChinese,
        "资源",
        &[],
        None,
        &["name"],
        &["uri"],
    );
    let _: iced::Element<'static, Message> = value_preview_section(
        config_ui::DesktopLocale::SimplifiedChinese,
        "资源",
        &many_values,
        None,
        &["name", "title"],
        &["uri", "description"],
    );
    let _: iced::Element<'static, Message> = value_preview_section(
        config_ui::DesktopLocale::SimplifiedChinese,
        "资源",
        &many_values,
        Some("resource error"),
        &["name"],
        &["uri"],
    );

    let mut disabled = inspected_server("disabled", McpConnectionStatusDto::Disabled);
    let _: iced::Element<'static, Message> = capability_preview(identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        Some(&disabled),
        true,
        false,
    );
    disabled.connection_status = McpConnectionStatusDto::Error;
    disabled.error = Some("server failed".repeat(12));
    let _: iced::Element<'static, Message> = capability_preview(identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        Some(&disabled),
        true,
        true,
    );
    let _: iced::Element<'static, Message> = capability_preview(identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        None,
        true,
        false,
    );
}
