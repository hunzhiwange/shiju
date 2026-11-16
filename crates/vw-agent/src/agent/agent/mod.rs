//! Agent 核心模块
//!
//! 本模块提供 VibeWindow 代理的核心实现，包括：
//! - 代理的构建和配置
//! - 对话历史管理
//! - 工具调用循环
//! - 记忆系统与上下文加载
//! - 研究阶段的自动触发和执行
//! - 查询分类与模型路由

mod builder;
mod core;
mod run;

#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
#[cfg(test)]
#[path = "turn_tests.rs"]
mod turn_tests;

pub use builder::AgentBuilder;
pub use core::Agent;
#[cfg(not(target_arch = "wasm32"))]
pub use run::run;

/// 单元测试模块
///
/// 测试代码位于同目录的 `tests.rs` 文件中。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
