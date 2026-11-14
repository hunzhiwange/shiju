//! macOS 原生菜单与核心事件转发；其它平台不提供菜单实现。

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
mod edit_menu;

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
pub use edit_menu::{install_edit_menu, set_current_url};

#[cfg(all(test, not(target_arch = "wasm32"), target_os = "macos"))]
mod edit_menu_tests;
