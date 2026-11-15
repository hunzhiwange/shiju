//! Desktop WebView process control and host-mediated MCP app actions.
pub mod mcp_form;

#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_app_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_app_download;
#[cfg(not(target_arch = "wasm32"))]
pub mod webview_runtime;

#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_app_panel;

pub mod state;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod plugin_compat_t30_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod plugin_compat_t31_tests;

#[cfg(all(test, target_os = "macos"))]
mod plugin_compat_t64_tests;

pub mod work_browser;
