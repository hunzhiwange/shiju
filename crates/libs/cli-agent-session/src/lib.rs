//! CLI（命令行界面）交互模块
//!
//! 本模块提供了 VibeWindow 代理的命令行界面交互功能，负责处理用户输入、
//! 渲染输出、管理会话状态以及协调代理的运行循环。
//!
//! # 模块结构
//!
//! - `interactive` - 交互式会话管理，处理多轮对话
//! - `interactive_input` - 交互式输入处理，包括用户输入的读取和解析
//! - `processor` - 消息处理器，处理代理的核心逻辑
//! - `run` - 主运行入口，启动代理的执行循环
//! - `session` - 会话管理，维护对话上下文和状态
//! - `setup` - 初始化设置，配置 CLI 环境
//! - `single_message` - 单消息模式，处理一次性输入输出
//! - `stats` - 统计信息收集和展示
//!
//! # 导出项
//!
//! - `run` - 公开的运行入口函数，用于启动 CLI 代理

pub(crate) mod interactive;
pub(crate) mod interactive_input;
#[cfg(test)]
#[path = "interactive_input_tests.rs"]
mod interactive_input_tests;
pub(crate) mod processor;
pub(crate) mod run;
#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
pub(crate) mod session;
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
pub(crate) mod setup;
#[cfg(test)]
#[path = "setup_tests.rs"]
mod setup_tests;
pub(crate) mod single_message;
#[cfg(test)]
#[path = "single_message_tests.rs"]
mod single_message_tests;
pub(crate) mod stats;
#[cfg(test)]
#[path = "stats_tests.rs"]
mod stats_tests;

/// 导出主运行入口函数
///
/// 这是 CLI 代理的公开入口点，启动代理的执行循环并处理用户交互。
/// 外部代码应通过此函数启动代理的 CLI 模式。
pub use run::run;
