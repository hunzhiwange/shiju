//! Gateway 驱动的终端界面组装与运行时边界。
//!
//! `app` 负责宿主生命周期，`bridge` 转换 Gateway 与界面契约，`state`/`model`
//! 保持可测的纯状态。本模块只导出当前 CLI 入口，不在此处隐式选择其他实现。

use vw_tui::V3Surface;

pub(crate) mod app;
pub(crate) mod bridge;
pub(crate) mod controller;
pub(crate) mod editor;
pub(crate) mod model;
pub(crate) mod paste;
pub(crate) mod runtime;
pub(crate) mod state;
pub(crate) mod vim;

#[cfg(not(test))]
pub(crate) use app::run_tui;

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;

#[cfg(test)]
#[path = "parity_tests.rs"]
mod parity_tests;

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod interaction_tests;

/// 构造独立界面对象，但不将其注册或切换为默认 TUI。
pub(crate) fn build_surface() -> V3Surface {
    V3Surface::new()
}
