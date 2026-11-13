//! Explicit isolated archive acceptance; never launches the user's bootstrap tasks.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
#[path = "app/components/archived_chats_window_tests.rs"]
mod acceptance;
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
#[path = "app/components/archived_chats_window/native.rs"]
mod native;
fn main() {
    if std::env::var_os("SHIJU_ARCHIVED_ISOLATED_CHILD").is_none() {
        let home = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .env("SHIJU_ARCHIVED_ISOLATED_CHILD", "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .status()
            .unwrap();
        assert!(status.success(), "isolated acceptance child: {status}");
        return;
    }
    native::run();
}
