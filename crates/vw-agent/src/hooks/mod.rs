//! # Hooks 模块
//!
//! 本模块提供了 VibeWindow 代理系统的生命周期钩子（Hooks）机制。
//!
//! ## 模块概述
//!
//! 钩子系统允许在代理执行的关键生命周期节点插入自定义行为，例如：
//! - 消息处理前后
//! - 工具调用前后
//! - 提供者调用前后
//! - 会话生命周期事件
//!
//! ## 主要组件
//!
//! - [`HookHandler`]：钩子处理器 trait，定义钩子的执行接口
//! - [`HookResult`]：钩子执行结果，表示钩子执行的状态和输出
//! - [`HookRunner`]：钩子运行器，负责管理和执行已注册的钩子
//!
//! ## 设计目标
//!
//! - **可扩展性**：支持内置钩子和第三方插件钩子
//! - **解耦性**：钩子与核心代理逻辑分离，互不影响
//! - **可观测性**：钩子可用于日志、指标收集和调试
//!
//! ## 使用示例
//!
//! ```ignore
//! use vibewindow::app::agent::hooks::{HookHandler, HookResult};
//!
//! // 实现自定义钩子处理器
//! struct MyHook;
//!
//! impl HookHandler for MyHook {
//!     async fn handle(&self, context: &HookContext) -> HookResult {
//!         // 自定义钩子逻辑
//!         HookResult::Continue
//!     }
//! }
//! ```

/// 内置钩子模块
///
/// 包含 VibeWindow 预定义的钩子实现，提供开箱即用的功能：
/// - 日志记录钩子
/// - 性能指标收集钩子
/// - 安全审计钩子
pub mod builtin;

/// 钩子运行器模块（内部）
///
/// 包含 [`HookRunner`] 的实现，负责：
/// - 管理已注册的钩子列表
/// - 按优先级顺序执行钩子
/// - 处理钩子执行错误和超时
mod runner;

/// Shared hook runtime assembly and replacement support.
mod runtime;

/// 钩子 trait 定义模块（内部）
///
/// 定义钩子系统的核心接口：
/// - [`HookHandler`]：钩子处理器 trait
/// - [`HookResult`]：钩子执行结果枚举
mod traits;

/// 重导出钩子运行器
///
/// [`HookRunner`] 是管理钩子执行的主要入口点，外部代码通过它来：
/// - 注册新的钩子处理器
/// - 触发钩子执行
/// - 查询已注册的钩子状态
pub use runner::HookRunner;
pub use runtime::{HookRuntime, HookRuntimeCandidate, build_hook_runtime};

/// 重导出钩子处理器 trait 和执行结果
///
/// 注意：[`HookHandler`] 和 [`HookResult`] 是 crate 公开钩子 API 的一部分。
/// 它们可能在内部看起来未被使用，但为了外部集成和未来的插件开发者，
/// 这里有意将其重导出为公开接口。
///
/// ## 公开 API 契约
///
/// 这些类型是公开 API 的稳定部分，应当保持向后兼容性：
/// - 第三方插件可以实现 `HookHandler` 来扩展代理行为
/// - `HookResult` 的变体名称和语义应保持稳定
#[allow(unused_imports)]
pub use traits::{HookHandler, HookResult};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
