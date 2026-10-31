//! Explicit native window validation using production UI code.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
fn main() {
    app::mcp_form_window::run();
}
