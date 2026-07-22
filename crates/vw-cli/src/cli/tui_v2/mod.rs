//! 新一代 CLI TUI 入口模块。
//!
//! 当前阶段已覆盖 Phase 1 到 Phase 3 的基础骨架：
//! - 为 CLI 建立直连 gateway client 的依赖边界
//! - 落地内部模型层、状态层与 snapshot round-trip
//! - 新建 tui_v2 根入口、controller/renderer 分层与 fullscreen layout slots
//!
//! 该模块是 CLI 交互模式的唯一 TUI 宿主，负责 gateway-first 路径所需的
//! 内部状态、全屏布局、输入处理与运行时事件适配。

pub(crate) mod app;
#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
pub(crate) mod controller;
#[cfg(test)]
#[path = "controller_tests.rs"]
mod controller_tests;
pub(crate) mod input;
pub(crate) mod model;
pub(crate) mod render;
pub(crate) mod runtime;
pub(crate) mod state;

pub(crate) use app::TuiRunMode;
#[cfg(not(test))]
pub(crate) use app::run_tui_v2;

#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;

#[cfg(test)]
mod tests;
