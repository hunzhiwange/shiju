//! VibeWindow-owned boundary for the isolated v3 TUI.
//! 隔离后的 v3 TUI 由 VibeWindow 自有的边界模块统一承载。
//!
//! 动作、展示事件与根表面在此分层导出，避免 UI 直接依赖运行时协议。

pub mod action;
pub mod event;
pub mod surface;

#[cfg(test)]
// 表面契约测试独立存放，业务模块保持单一职责。
#[path = "surface_tests.rs"]
mod surface_tests;
