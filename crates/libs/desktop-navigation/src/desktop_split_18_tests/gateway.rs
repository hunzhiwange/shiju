use crate::components::top_bar::{Menu as MenuType, Message, gateway::*, types::TopBarGatewayTab};
use desktop_settings_network::state::{GatewayClientServerDraft, GatewayClientSettingsState};
use desktop_settings_skills::types::PluginCatalogItem;
use iced::Element;
#[derive(Default)]
struct Fixture {
    gateway_client_settings: GatewayClientSettingsState,
    plugins: Vec<PluginCatalogItem>,
    plugin_loading: bool,
    loading: bool,
    saving: bool,
    save_error: Option<String>,
    status_message: Option<String>,
    mcp_text: String,
    active_menu: Option<MenuType>,
    top_bar_gateway_tab: TopBarGatewayTab,
}
impl Fixture {
    fn state(&self) -> State<'_> {
        State {
            desktop_locale: Default::default(),
            gateway_client_settings: &self.gateway_client_settings,
            mcp: McpState {
                loading: self.loading,
                saving: self.saving,
                save_error: &self.save_error,
                status_message: &self.status_message,
            },
            mcp_text: &self.mcp_text,
            plugins: PluginState {
                plugin_loading: self.plugin_loading,
                plugin_catalog: &self.plugins,
            },
            active_menu: self.active_menu,
            top_bar_gateway_tab: self.top_bar_gateway_tab,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_disabled: false,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_server_keys: vec![],
            #[cfg(not(target_arch = "wasm32"))]
            lsp_progress: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_status: None,
        }
    }
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

fn mark_server_health(app: &mut Fixture, server: &GatewayClientServerDraft, healthy: bool) {
    let key = desktop_settings_network::server_health_key(server)
        .expect("test server should produce a health key");
    app.gateway_client_settings.health.insert(key, healthy);
}

#[test]
fn gateway_health_defaults_to_false_without_health_entry() {
    let app = Fixture::default();
    let server = server("local", "Local", "127.0.0.1", 42617);

    assert!(!gateway_server_healthy(&app.state(), &server));
    assert!(!active_gateway_healthy(&app.state()));
}

#[test]
fn gateway_health_uses_matching_server_health_key() {
    let mut app = Fixture::default();
    let local = server("local", "Local", "127.0.0.1", 42617);
    let remote = server("remote", "Remote", "https://gateway.example.test", 443);
    app.gateway_client_settings.servers = vec![local.clone(), remote.clone()];
    app.gateway_client_settings.selected_server_id = remote.id.clone();
    mark_server_health(&mut app, &local, false);
    mark_server_health(&mut app, &remote, true);

    assert!(!gateway_server_healthy(&app.state(), &local));
    assert!(gateway_server_healthy(&app.state(), &remote));
    assert!(active_gateway_healthy(&app.state()));
}

#[test]
fn active_gateway_health_is_false_when_selected_server_is_missing() {
    let mut app = Fixture::default();
    let local = server("local", "Local", "127.0.0.1", 42617);
    app.gateway_client_settings.servers = vec![local.clone()];
    app.gateway_client_settings.selected_server_id = "missing".to_string();
    mark_server_health(&mut app, &local, true);

    assert!(!active_gateway_healthy(&app.state()));
}

#[test]
fn gateway_tab_content_includes_servers_and_management_actions() {
    let mut app = Fixture::default();
    let local = server("local", "Local", "127.0.0.1", 42617);
    let remote = server("remote", "Remote", "10.0.0.2", 8080);
    app.gateway_client_settings.servers = vec![local.clone(), remote.clone()];
    app.gateway_client_settings.selected_server_id = local.id.clone();
    mark_server_health(&mut app, &local, true);
    mark_server_health(&mut app, &remote, false);

    let rows = gateway_tab_content(&app.state());

    assert_eq!(rows.len(), 4);
}

#[test]
fn plugin_tab_content_includes_toggle_and_management_actions() {
    let mut app = Fixture::default();
    app.plugins.push(PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Demo plugin".to_string(),
        enabled: true,
        component_count: 4,
        source_path: Some("/tmp/demo-plugin".to_string()),
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    });

    let rows = plugin_tab_content(&app.state());

    assert_eq!(rows.len(), 3);
}

#[test]
fn mcp_tab_content_includes_servers_and_management_actions() {
    let mut app = Fixture::default();
    app.mcp_text = String::from(
        r#"[
          {"name":"docs", "type":"http", "url":"https://example.test/mcp"},
          {"name":"local", "command":"node", "enabled":false}
        ]"#,
    );

    let rows = mcp_tab_content(&app.state());

    assert_eq!(rows.len(), 4);
}

#[test]
fn mcp_tab_content_handles_loading_saving_errors_status_and_parse_empty_states() {
    let mut app = Fixture::default();
    app.loading = true;
    app.saving = true;
    app.save_error = Some("very long gateway mcp error message".to_string());
    app.status_message = Some("MCP 已同步".to_string());
    app.mcp_text = String::from("{not-json");

    assert_eq!(mcp_tab_content(&app.state()).len(), 6);

    app.loading = false;
    app.saving = false;
    app.save_error = None;
    app.status_message = None;
    assert_eq!(mcp_tab_content(&app.state()).len(), 3);

    app.mcp_text = String::from(" ");
    assert_eq!(mcp_tab_content(&app.state()).len(), 3);
}

#[test]
fn gateway_and_plugin_tabs_cover_empty_and_loading_states() {
    let mut app = Fixture::default();
    app.gateway_client_settings.servers.clear();
    assert_eq!(gateway_tab_content(&app.state()).len(), 2);

    app.plugin_loading = true;
    assert_eq!(plugin_tab_content(&app.state()).len(), 3);

    app.plugin_loading = false;
    assert_eq!(plugin_tab_content(&app.state()).len(), 3);
}

#[test]
fn gateway_services_module_builds_each_tab() {
    let mut app = Fixture::default();
    app.active_menu = Some(MenuType::GatewayServices);

    for tab in [
        TopBarGatewayTab::Gateway,
        TopBarGatewayTab::Mcp,
        TopBarGatewayTab::Lsp,
        TopBarGatewayTab::Plugins,
    ] {
        app.top_bar_gateway_tab = tab;
        let _: Element<'_, Message> = gateway_services_module(&app.state());
    }

    app.active_menu = None;
    let _: Element<'_, Message> = gateway_services_module(&app.state());
}
