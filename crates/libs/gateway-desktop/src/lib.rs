//! Desktop gateway routes and native host actions.
#![cfg(not(target_arch = "wasm32"))]
mod desktop;
mod desktop_acp_install;
mod desktop_browser_install;
pub mod desktop_skills;
pub use desktop::router;
