//! Gateway 驱动的终端界面组装与运行时边界。
//!
//! `app` 负责宿主生命周期，`bridge` 转换 Gateway 与界面契约，`state`/`model`
//! 保持可测的纯状态。本模块只导出当前 CLI 入口，不在此处隐式选择其他实现。

use vw_tui::V3Surface;

pub(crate) mod app;
pub(crate) mod bridge;
pub(crate) mod commands;
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

#[cfg(test)]
#[path = "command_tests.rs"]
mod command_tests;

#[cfg(test)]
#[path = "session_restore_tests.rs"]
mod session_restore_tests;

#[cfg(test)]
#[path = "session_command_tests.rs"]
mod session_command_tests;

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;

#[cfg(test)]
#[path = "attachment_tests.rs"]
mod attachment_tests;

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "orchestration_tests.rs"]
mod orchestration_tests;

#[cfg(test)]
#[path = "plan_goal_tests.rs"]
mod plan_goal_tests;

/// 构造独立界面对象，但不将其注册或切换为默认 TUI。
pub(crate) fn build_surface() -> V3Surface {
    V3Surface::new()
}

#[cfg(test)]
mod session_title_tests;
