//! Native MCP configuration adapters and host dispatch.
pub mod mcp;
pub use mcp::*;

#[cfg(target_os = "macos")]
use app_auth::auth;

#[cfg(target_os = "macos")]
mod providers {
    pub use app_auth::providers::{dsh, kimi, live};
    pub use app_config_paths as paths;
    pub use app_provider_native::providers::{hermes, opencode};
    #[cfg(test)]
    pub use app_providers::providers::grok;
}
