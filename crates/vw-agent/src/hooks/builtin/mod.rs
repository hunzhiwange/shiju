//! # 内置钩子模块
//!
//! 本模块提供 VibeWindow 运行时预置的钩子实现集合。
//!
//! ## 模块职责
//!
//! - 聚合并导出所有内置钩子类型
//! - 作为 `hooks::builtin` 命名空间的统一入口点
//! - 便于外部模块通过单一路径访问内置钩子
//!
//! ## 可用钩子
//!
//! | 钩子名称 | 用途 | 优先级 |
//! |---------|------|--------|
//! | `CommandLoggerHook` | 工具调用审计日志 | -50 |
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::hooks::builtin::CommandLoggerHook;
//!
//! let logger = CommandLoggerHook::new();
//! ```
//!
//! ## 扩展指南
//!
//! 添加新的内置钩子时：
//! 1. 在本目录下创建新的子模块（推荐使用目录结构以支持测试分离）
//! 2. 在本文件中添加 `pub mod` 声明
//! 3. 在本文件中添加 `pub use` 重新导出

/// 命令日志记录钩子模块
///
/// 提供工具调用审计功能的钩子实现
pub mod command_logger;

/// 重新导出 [`CommandLoggerHook`] 以简化外部访问路径
///
/// 外部模块可直接使用 `hooks::builtin::CommandLoggerHook`
/// 而无需显式引用 `hooks::builtin::command_logger::CommandLoggerHook`
pub use command_logger::{
    CommandAuditEntry, CommandAuditStore, CommandLoggerHook, DEFAULT_COMMAND_AUDIT_CAPACITY,
};
