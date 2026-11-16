//! 从宿主迁入的 LSP 展示用例，直接使用导航领域输入。
#![cfg(not(target_arch = "wasm32"))]
use crate::components::top_bar::gateway::*;
fn state<'a>(
    settings: &'a desktop_settings_network::state::GatewayClientSettingsState,
    plugins: &'a Vec<desktop_settings_skills::types::PluginCatalogItem>,
) -> State<'a> {
    State {
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        gateway_client_settings: settings,
        mcp: McpState { loading: false, saving: false, save_error: &None, status_message: &None },
        mcp_text: "",
        plugins: PluginState { plugin_loading: false, plugin_catalog: plugins },
        active_menu: None,
        top_bar_gateway_tab: Default::default(),
        lsp_disabled: false,
        lsp_server_keys: Vec::new(),
        lsp_progress: Default::default(),
        lsp_status: None,
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_disabled_state_first() {
    let settings = Default::default();
    let plugins = Vec::new();
    let mut app = state(&settings, &plugins);
    app.lsp_disabled = true;
    app.lsp_status = Some("LSP ready".to_string());
    app.lsp_server_keys.push("rust-analyzer");

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_empty_status_when_no_servers_exist() {
    let settings = Default::default();
    let plugins = Vec::new();
    let mut app = state(&settings, &plugins);
    app.lsp_status = Some("LSP 尚未启动".to_string());

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_deduplicates_keys_and_appends_status() {
    let settings = Default::default();
    let plugins = Vec::new();
    let mut app = state(&settings, &plugins);
    app.lsp_status = Some("LSP 已连接".to_string());
    app.lsp_server_keys.push("rust-analyzer");
    app.lsp_server_keys.push("rust-analyzer");
    app.lsp_progress.insert("rust-analyzer", ServerProgress { working: false, progress: None });

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_formats_progress_detail() {
    let settings = Default::default();
    let plugins = Vec::new();
    let mut app = state(&settings, &plugins);
    app.lsp_progress.insert(
        "rust-analyzer",
        ServerProgress {
            working: true,
            progress: Some(LspProgress {
                title: "indexing",
                message: Some("workspace"),
                percentage: Some(42),
            }),
        },
    );

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}
