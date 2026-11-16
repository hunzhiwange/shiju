use crate::app::{App, Message};
use desktop_navigation::components::top_bar::gateway as domain;
fn with_state<R>(app: &App, render: impl FnOnce(&domain::State<'_>) -> R) -> R {
 let mcp_text = app.mcp_settings.editor.text();
 let state = domain::State {
 desktop_locale: app.desktop_locale, gateway_client_settings: &app.gateway_client_settings,
 mcp: domain::McpState { loading: app.mcp_settings.loading, saving: app.mcp_settings.saving, save_error: &app.mcp_settings.save_error, status_message: &app.mcp_settings.status_message },
 mcp_text: &mcp_text,
 plugins: domain::PluginState { plugin_loading: app.skills_settings.plugin_loading, plugin_catalog: &app.skills_settings.plugin_catalog },
 active_menu: super::mapping::local_menu(app.navigation_panels.active_menu), top_bar_gateway_tab: app.navigation_panels.top_bar_gateway_tab,
 #[cfg(not(target_arch = "wasm32"))] lsp_disabled: app.lsp_disabled,
 #[cfg(not(target_arch = "wasm32"))] lsp_server_keys: app.navigation_tabs.preview_tabs.iter().filter_map(|tab| tab.lsp_server_key).collect(),
 #[cfg(not(target_arch = "wasm32"))] lsp_progress: app.lsp_progress.iter().map(|(key, entries)| (key.as_str(), domain::ServerProgress {
 working: !entries.is_empty(), progress: entries.values().next().map(|progress| domain::LspProgress { title: &progress.title, message: progress.message.as_deref(), percentage: progress.percentage }),
 })).collect(),
 #[cfg(not(target_arch = "wasm32"))] lsp_status: crate::app::i18n::editor::lsp_status(app),
 };
 render(&state)
}
pub(super) fn gateway_services_module(app: &App) -> iced::Element<'_, Message> { with_state(app, domain::gateway_services_module).map(super::mapping::map_message) }
#[cfg(test)]
#[path = "gateway_tests.rs"] mod gateway_tests;
#[cfg(test)] fn lsp_tab_content(app: &App) -> Vec<iced::Element<'static, Message>> { with_state(app, domain::lsp_tab_content).into_iter().map(|row| row.map(super::mapping::map_message)).collect() }
#[cfg(test)] fn manage_plugins_message() -> Message { super::mapping::map_message(domain::manage_plugins_message()) }
