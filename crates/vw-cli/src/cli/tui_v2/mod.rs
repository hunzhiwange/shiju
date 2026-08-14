//! 新一代 CLI TUI 入口模块。
//!
//! 当前阶段已覆盖 Phase 1 到 Phase 3 的基础骨架：
//! - 为 CLI 建立直连 gateway client 的依赖边界
//! - 落地内部模型层、状态层与 snapshot round-trip
//! - 新建 tui_v2 根入口、controller/renderer 分层与 fullscreen layout slots
//!
//! 该模块是 CLI 交互模式的唯一 TUI 宿主，负责 gateway-first 路径所需的
//! 内部状态、全屏布局、输入处理与运行时事件适配。

/// 顶层事件循环及其 session、stream、command 内部副作用模块。
pub(crate) mod app;
#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
mod clipboard;
#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod clipboard_tests;
pub(crate) mod commands;
pub(crate) mod completion;
#[cfg(test)]
#[path = "completion_tests.rs"]
mod completion_tests;
pub(crate) mod controller;
#[cfg(test)]
#[path = "controller_tests.rs"]
mod controller_tests;
pub(crate) mod editor;
#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;
pub(crate) mod effect;
mod history_search;
#[cfg(test)]
#[path = "history_search_tests.rs"]
mod history_search_tests;
pub(crate) mod input;
mod keybindings;
#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod keybindings_tests;
mod keymap;
mod message_action_controller;
#[cfg(test)]
#[path = "message_action_controller_tests.rs"]
mod message_action_controller_tests;
pub(crate) mod model;
mod notifications;
#[cfg(test)]
#[path = "notifications_tests.rs"]
mod notifications_tests;
mod overlay_controller;
pub(crate) mod paste;
#[cfg(test)]
#[path = "paste_tests.rs"]
mod paste_tests;
pub(crate) mod render;
pub(crate) mod runtime;
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
pub(crate) mod state;
#[cfg(test)]
#[path = "stress_tests.rs"]
mod stress_tests;
pub(crate) mod terminal_capabilities;
mod transcript_controller;
#[cfg(test)]
#[path = "transcript_controller_tests.rs"]
mod transcript_controller_tests;
mod vim;
#[cfg(test)]
#[path = "vim_tests.rs"]
mod vim_tests;

pub(crate) use app::TuiRunMode;
#[cfg(not(test))]
pub(crate) use app::run_tui_v2;

#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;

#[cfg(test)]
mod tests;
