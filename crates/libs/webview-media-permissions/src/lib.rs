//! Native WebView media permission adapters.

#![cfg(not(any(target_arch = "wasm32", coverage)))]

#[cfg(all(unix, not(target_os = "macos")))]
mod gtk;
#[cfg(all(unix, not(target_os = "macos")))]
pub use gtk::install;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::with_permission_handler;
