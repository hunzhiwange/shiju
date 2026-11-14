//! WebView binary mode dispatch.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(any(target_arch = "wasm32", coverage)))]
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--mcp-app-stdin") {
        if let Err(error) = webview_mcp_runtime::run() {
            eprintln!("MCP app failed: {error}");
        }
        return;
    }
    webview_browser_runtime::run(args);
}

#[cfg(any(target_arch = "wasm32", coverage))]
fn main() {}

#[cfg(all(test, coverage))]
#[path = "main_tests.rs"]
mod main_tests;
