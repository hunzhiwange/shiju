//! 桌面端原生菜单（macOS 系统菜单栏）。
//!
//! topbar 内嵌菜单保持不变；本模块额外安装与之同源的原生菜单。

#[cfg(any(target_os = "macos", test))]
mod actions;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use crate::app::components::top_bar::menu_actions::NativeMenuAction;
#[cfg(target_os = "macos")]
pub(crate) use actions::{NativeMenuSnapshot, resolve_native_menu_action};

#[cfg(target_os = "macos")]
pub(crate) use macos::{install_or_sync, subscription};

#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
