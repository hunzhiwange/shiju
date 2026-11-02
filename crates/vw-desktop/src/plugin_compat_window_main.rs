//! Explicit native window validation target; no product switch.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
#[path = "app/plugin_compat_window.rs"]
mod plugin_compat_window;
fn main() {
    plugin_compat_window::run();
}
