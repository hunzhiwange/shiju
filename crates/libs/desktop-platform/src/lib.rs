//! Explicit native integration entry points; no automatic system registration.
pub mod menu_actions;
pub mod native_menu;
pub mod desktop_notification;
pub mod skill_deeplink;
#[cfg(target_os = "macos")]
pub mod single_instance;
#[cfg(target_os = "macos")]
pub mod deep_links;
#[cfg(target_os = "macos")]
pub mod mcp_link_preview;
#[cfg(target_os = "macos")]
pub mod prompt_link_preview;
#[cfg(target_os = "macos")]
pub mod provider_link_preview;
#[cfg(target_os = "macos")]
pub mod skill_link_preview;
#[cfg(target_os = "windows")]
pub mod native_crash;

#[cfg(target_os = "macos")]
pub mod window;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_lifecycle;
