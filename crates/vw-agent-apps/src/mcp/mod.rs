//! Business rules for the managed MCP catalog.

pub mod catalog;
pub mod presets;
pub mod validation;

#[cfg(test)]
mod presets_tests;
#[cfg(test)]
mod validation_tests;

#[cfg(target_os = "macos")]
pub mod native_file;
#[cfg(all(test, target_os = "macos"))]
mod native_file_tests;

#[cfg(target_os = "macos")]
pub mod adapters;

#[cfg(target_os = "macos")]
pub mod import;

#[cfg(target_os = "macos")]
pub mod distribution;

use vw_api_types::{
    agent_apps::AppId,
    mcp_management::{McpAppCapability, McpSupport},
};

/// Native adapter transport support for the gateway's execution platform.
pub fn native_capability(app_id: AppId) -> McpAppCapability {
    use AppId::*;
    use McpSupport::{NotVerified, Supported, Unsupported};

    let (stdio, streamable_http, sse) = if !cfg!(target_os = "macos") {
        (NotVerified, NotVerified, NotVerified)
    } else {
        match app_id {
            ClaudeDesktop | OpenClaw => (Unsupported, Unsupported, Unsupported),
            Agy | Codex | Dsh | GrokBuild | Pi => (Supported, Supported, Unsupported),
            OpenCode => (Supported, Unsupported, Supported),
            Claude | Gemini | Hermes | Kimi | Mcode | Omp => (Supported, Supported, Supported),
        }
    };
    McpAppCapability { app_id, stdio, streamable_http, sse }
}
