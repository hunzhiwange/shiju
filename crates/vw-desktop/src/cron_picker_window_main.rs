//! 显式运行日期时间选择器的真实窗口验收。
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
#[path = "app/components/system_settings_cron/date_time_window_tests.rs"]
mod window_tests;
fn main() {
    window_tests::run();
}
