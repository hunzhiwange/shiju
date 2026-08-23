//! 汇总原先独立的集成测试模块，由顶层单一测试二进制统一加载。
//! 平台限定仍保留在各模块声明处，避免在不支持的系统编译 PTY 场景。

// Aggregates all former standalone integration tests as modules.
// 将原先独立的集成测试汇总为模块。
#[cfg(unix)]
mod focus_palette;
mod resize_reflow;
mod status_indicator;
mod vt100_history;
mod vt100_live_commit;
