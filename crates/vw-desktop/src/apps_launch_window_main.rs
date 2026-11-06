//! Explicit native rendering validation, excluded from default tests.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
#[path = "app/components/system_settings_apps/launch_window_tests.rs"]
mod window_tests;
fn main() {
    window_tests::run();
}
