//! 桌面端原生菜单（macOS 系统菜单栏）。
//!
//! topbar 内嵌菜单保持不变；本模块额外安装与之同源的原生菜单。

mod actions;

#[cfg(target_os = "macos")]
mod macos;

pub use crate::app::components::top_bar::menu_actions::NativeMenuAction;
pub(crate) use actions::{NativeMenuSnapshot, resolve_native_menu_action};

#[cfg(target_os = "macos")]
pub(crate) use macos::{install_or_sync, subscription};

#[cfg(not(target_os = "macos"))]
pub(crate) fn install_or_sync(_snapshot: NativeMenuSnapshot) {}

#[cfg(not(target_os = "macos"))]
pub(crate) fn subscription() -> iced::Subscription<crate::app::Message> {
    iced::Subscription::none()
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
