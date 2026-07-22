use super::{
    active_gateway_healthy, compact_gateway_label, gateway_menu_container,
    gateway_menu_item_btn_keep_open, gateway_press_message, gateway_row, gateway_row_keep_open,
    gateway_server_healthy, gateway_services_button, gateway_services_module, gateway_tab_content,
    lsp_tab_content, mcp_server_summaries, mcp_tab_content, plugin_tab_content, status_dot,
    tab_label,
};
use crate::app::components::editor::Editor;
use crate::app::message::settings::{GatewayClientMessage, SettingsMessage};
use crate::app::message::view::MenuType;
use crate::app::state::{GatewayClientServerDraft, PluginCatalogItem, TopBarGatewayTab};
use crate::app::{App, LspProgress, Message, PreviewTab, message};
use iced::widget::Space;
use iced::{Element, Length};
use std::collections::HashMap;

fn test_app() -> App {
    App::new().0
}

fn server(id: &str, name: &str, host: &str, port: u16) -> GatewayClientServerDraft {
    GatewayClientServerDraft {
        id: id.to_string(),
        name: name.to_string(),
        host: host.to_string(),
        port,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        skey: String::new(),
    }
}

fn mark_server_health(app: &mut App, server: &GatewayClientServerDraft, healthy: bool) {
    let key = crate::app::message::gateway_health::server_health_key(server)
        .expect("test server should produce a health key");
    app.gateway_client_settings.health.insert(key, healthy);
}

fn preview_tab_with_lsp(server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: "/tmp/main.rs".to_string(),
        title: "main.rs".to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new("", "rust"),
        scroll_id: iced::widget::Id::unique(),
        lsp_server_key: server_key,
        lsp_uri: None,
        lsp_language_id: None,
    }
}

#[test]
fn gateway_health_defaults_to_false_without_health_entry() {
    let app = test_app();
    let server = server("local", "Local", "127.0.0.1", 42617);

    assert!(!gateway_server_healthy(&app, &server));
    assert!(!active_gateway_healthy(&app));
}

#[test]
fn gateway_health_uses_matching_server_health_key() {
    let mut app = test_app();
    let local = server("local", "Local", "127.0.0.1", 42617);
    let remote = server("remote", "Remote", "https://gateway.example.test", 443);
    app.gateway_client_settings.servers = vec![local.clone(), remote.clone()];
    app.gateway_client_settings.selected_server_id = remote.id.clone();
    mark_server_health(&mut app, &local, false);
    mark_server_health(&mut app, &remote, true);

    assert!(!gateway_server_healthy(&app, &local));
    assert!(gateway_server_healthy(&app, &remote));
    assert!(active_gateway_healthy(&app));
}

#[test]
fn active_gateway_health_is_false_when_selected_server_is_missing() {
    let mut app = test_app();
    let local = server("local", "Local", "127.0.0.1", 42617);
    app.gateway_client_settings.servers = vec![local.clone()];
    app.gateway_client_settings.selected_server_id = "missing".to_string();
    mark_server_health(&mut app, &local, true);

    assert!(!active_gateway_healthy(&app));
}

#[test]
fn gateway_tab_content_includes_servers_and_management_actions() {
    let mut app = test_app();
    let local = server("local", "Local", "127.0.0.1", 42617);
    let remote = server("remote", "Remote", "10.0.0.2", 8080);
    app.gateway_client_settings.servers = vec![local.clone(), remote.clone()];
    app.gateway_client_settings.selected_server_id = local.id.clone();
    mark_server_health(&mut app, &local, true);
    mark_server_health(&mut app, &remote, false);

    let rows = gateway_tab_content(&app);

    assert_eq!(rows.len(), 4);
}

#[test]
fn plugin_tab_content_includes_toggle_and_management_actions() {
    let mut app = test_app();
    app.skills_settings.plugin_catalog.push(PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Demo plugin".to_string(),
        enabled: true,
        component_count: 4,
        source_path: Some("/tmp/demo-plugin".to_string()),
    });

    let rows = plugin_tab_content(&app);

    assert_eq!(rows.len(), 3);
}

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
fn mcp_tab_content_includes_servers_and_management_actions() {
    let mut app = test_app();
    app.mcp_settings.editor = iced::widget::text_editor::Content::with_text(
        r#"[
          {"name":"docs", "type":"http", "url":"https://example.test/mcp"},
          {"name":"local", "command":"node", "enabled":false}
        ]"#,
    );
    app.mcp_settings.server_count = 2;

    let rows = mcp_tab_content(&app);

    assert_eq!(rows.len(), 4);
}

#[test]
fn mcp_tab_content_handles_loading_saving_errors_status_and_parse_empty_states() {
    let mut app = test_app();
    app.mcp_settings.loading = true;
    app.mcp_settings.saving = true;
    app.mcp_settings.save_error = Some("very long gateway mcp error message".to_string());
    app.mcp_settings.status_message = Some("MCP 已同步".to_string());
    app.mcp_settings.editor = iced::widget::text_editor::Content::with_text("{not-json");

    assert_eq!(mcp_tab_content(&app).len(), 6);

    app.mcp_settings.loading = false;
    app.mcp_settings.saving = false;
    app.mcp_settings.save_error = None;
    app.mcp_settings.status_message = None;
    assert_eq!(mcp_tab_content(&app).len(), 3);

    app.mcp_settings.editor = iced::widget::text_editor::Content::with_text(" ");
    assert_eq!(mcp_tab_content(&app).len(), 3);
}

#[test]
fn gateway_press_message_can_keep_menu_open() {
    let direct = gateway_press_message(Message::Settings(SettingsMessage::PluginsRefresh), false);
    let wrapped = gateway_press_message(Message::Settings(SettingsMessage::PluginsRefresh), true);

    assert!(matches!(direct, Message::Settings(SettingsMessage::PluginsRefresh)));
    assert!(matches!(
        wrapped,
        Message::View(crate::app::message::view::ViewMessage::MenuAction(_))
    ));
}

#[test]
fn compact_gateway_label_truncates_by_characters() {
    assert_eq!(compact_gateway_label("网关服务状态", 2), "网关...");
    assert_eq!(compact_gateway_label("ok", 8), "ok");
}

#[test]
fn gateway_and_plugin_tabs_cover_empty_and_loading_states() {
    let mut app = test_app();
    app.gateway_client_settings.servers.clear();
    assert_eq!(gateway_tab_content(&app).len(), 2);

    app.skills_settings.plugin_loading = true;
    assert_eq!(plugin_tab_content(&app).len(), 3);

    app.skills_settings.plugin_loading = false;
    assert_eq!(plugin_tab_content(&app).len(), 3);
}

#[test]
fn basic_gateway_widgets_can_be_built_for_all_states() {
    let _: Element<'static, Message> = status_dot(true);
    let _: Element<'static, Message> = status_dot(false);
    let _: Element<'static, Message> = gateway_services_button(true, true);
    let _: Element<'static, Message> = gateway_services_button(false, false);
    let _: Element<'static, Message> = tab_label("网关", TopBarGatewayTab::Gateway, true);
    let _: Element<'static, Message> = tab_label("插件", TopBarGatewayTab::Plugins, false);
    let _: Element<'static, Message> = gateway_row(
        status_dot(true),
        "可点击行".to_string(),
        Some("✓"),
        Some(Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer))),
    );
    let _: Element<'static, Message> = gateway_row_keep_open(
        status_dot(true),
        "保持打开".to_string(),
        Some("刷新"),
        Some(Message::Settings(SettingsMessage::PluginsRefresh)),
    );
    let _: Element<'static, Message> = gateway_menu_item_btn_keep_open(
        "保持打开按钮",
        Some(Message::Settings(SettingsMessage::PluginsRefresh)),
    );
    let _: Element<'static, Message> =
        gateway_row(status_dot(false), "只读行".to_string(), None, None);
    let content: Element<'static, Message> = Space::new().height(Length::Fixed(1.0)).into();
    let _: Element<'static, Message> = gateway_menu_container(content);
}

#[test]
fn gateway_services_module_builds_each_tab() {
    let mut app = test_app();
    app.active_menu = Some(MenuType::GatewayServices);

    for tab in [
        TopBarGatewayTab::Gateway,
        TopBarGatewayTab::Mcp,
        TopBarGatewayTab::Lsp,
        TopBarGatewayTab::Plugins,
    ] {
        app.top_bar_gateway_tab = tab;
        let _: Element<'_, Message> = gateway_services_module(&app);
    }

    app.active_menu = None;
    let _: Element<'_, Message> = gateway_services_module(&app);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_disabled_state_first() {
    let mut app = test_app();
    app.lsp_disabled = true;
    app.lsp_status = Some("LSP ready".to_string());
    app.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_empty_status_when_no_servers_exist() {
    let mut app = test_app();
    app.lsp_status = Some("LSP 尚未启动".to_string());

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_deduplicates_keys_and_appends_status() {
    let mut app = test_app();
    app.lsp_status = Some("LSP 已连接".to_string());
    app.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));
    app.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));
    app.lsp_progress.insert("rust-analyzer".to_string(), HashMap::new());

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_formats_progress_detail() {
    let mut app = test_app();
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        HashMap::from([(
            "token-1".to_string(),
            LspProgress {
                title: "indexing".to_string(),
                message: Some("workspace".to_string()),
                percentage: Some(42),
            },
        )]),
    );

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[test]
fn gateway_tests_keep_public_view_messages_reachable() {
    let toggle = Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::GatewayServices)));
    let close = Message::View(message::ViewMessage::ToggleMenu(None));

    assert!(matches!(toggle, Message::View(_)));
    assert!(matches!(close, Message::View(_)));
}
