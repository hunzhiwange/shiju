//! 显式的原生窗口测试目标；同业务源码在 cfg(test) 下编译，无产品开关。
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]

mod app;
mod apps;
mod fonts;
#[path = "app/recovery_window_tests.rs"]
mod recovery_window_tests;

fn main() {
    recovery_window_tests::run();
}
