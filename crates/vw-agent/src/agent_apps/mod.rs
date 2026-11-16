//! Scoped native application storage and compatibility service exports.
pub mod mcp;
pub use agent_app_services::{
    auth_accounts, preferences_import, prompts_store, store, usage_settings,
};
#[cfg(target_os = "macos")]
pub use agent_app_services::{shared_snapshot, shared_snapshot_import};
