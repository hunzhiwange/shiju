use super::*;
use iced::widget::Space;
#[test]
fn mcp_server_summaries_read_editor_config() {
    let summaries = mcp_server_summaries(
        r#"[
          {"name":"docs", "type":"http", "url":"https://example.test/mcp", "enabled":false},
          {"name":"local", "command":"node", "required":true}
        ]"#,
    )
    .expect("valid MCP config should parse");

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].name, "docs");
    assert_eq!(summaries[0].transport, "http");
    assert!(!summaries[0].enabled);
    assert_eq!(summaries[1].transport, "stdio");
    assert!(summaries[1].required);
}

#[test]
fn mcp_server_summaries_handle_empty_invalid_and_defaulted_servers() {
    assert_eq!(mcp_server_summaries("  ").expect("empty config"), Vec::new());
    assert!(mcp_server_summaries("{not-json").expect_err("invalid json").contains("解析失败"));
    assert_eq!(mcp_server_summaries("{}").expect_err("non-array"), "MCP 配置必须是 server 数组");

    let summaries = mcp_server_summaries(
        r#"[
          42,
          {"name":"  ", "type":"  "},
          {"name":"local", "enabled":true}
        ]"#,
    )
    .expect("mixed config should parse with placeholders");

    assert_eq!(summaries[0].name, "server 1");
    assert_eq!(summaries[0].transport, "invalid");
    assert_eq!(summaries[1].name, "未命名");
    assert_eq!(summaries[1].transport, "stdio");
    assert!(summaries[2].enabled);
    assert!(!summaries[2].required);
}

#[test]
fn compact_gateway_label_truncates_by_characters() {
    assert_eq!(compact_gateway_label("网关服务状态", 2), "网关...");
    assert_eq!(compact_gateway_label("ok", 8), "ok");
}

#[test]
fn basic_gateway_widgets_can_be_built_for_all_states() {
    let _: Element<'static, Message> = status_dot(true);
    let _: Element<'static, Message> = status_dot(false);
    let _: Element<'static, Message> = gateway_services_button(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::English),
        true,
        true,
    );
    let _: Element<'static, Message> = gateway_services_button(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::English),
        false,
        false,
    );
    let _: Element<'static, Message> = tab_label("网关", TopBarGatewayTab::Gateway, true);
    let _: Element<'static, Message> = tab_label("插件", TopBarGatewayTab::Plugins, false);
    let _: Element<'static, Message> = gateway_row(
        status_dot(true),
        "可点击行".to_string(),
        Some("✓"),
        Some(Message::AddGateway),
    );
    let _: Element<'static, Message> = gateway_row_keep_open(
        status_dot(true),
        "保持打开".to_string(),
        Some("刷新"),
        Some(Message::PluginsRefresh),
    );
    let _: Element<'static, Message> = gateway_menu_item_btn_keep_open(
        "保持打开按钮",
        Some(Message::PluginsRefresh),
    );
    let _: Element<'static, Message> =
        gateway_row(status_dot(false), "只读行".to_string(), None, None);
    let content: Element<'static, Message> = Space::new().height(Length::Fixed(1.0)).into();
    let _: Element<'static, Message> = gateway_menu_container(content);
}

#[test]
fn gateway_press_message_can_keep_menu_open() {
    let direct = gateway_press_message(Message::PluginsRefresh, false);
    let wrapped = gateway_press_message(Message::PluginsRefresh, true);

    assert!(matches!(direct, Message::PluginsRefresh));
    assert!(matches!(
        wrapped,
        Message::MenuAction(_)
    ));
}
