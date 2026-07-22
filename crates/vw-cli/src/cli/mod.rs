//! CLI（命令行界面）交互模块
//!
//! 本模块提供了 VibeWindow 代理的命令行界面交互功能，负责处理用户输入、
//! 渲染输出、管理会话状态以及协调代理的运行循环。
//!
//! # 模块结构
//!
//! - `interactive` - 交互式会话管理，处理多轮对话
//! - `interactive_input` - 交互式输入处理，包括用户输入的读取和解析
//! - `logo` - Logo 文本生成，用于显示品牌标识
//! - `processor` - 消息处理器，处理代理的核心逻辑
//! - `render` - 渲染工具，用于格式化和显示输出
//! - `run` - 主运行入口，启动代理的执行循环
//! - `session` - 会话管理，维护对话上下文和状态
//! - `setup` - 初始化设置，配置 CLI 环境
//! - `single_message` - 单消息模式，处理一次性输入输出
//! - `stats` - 统计信息收集和展示
//! - `stdio` - 标准输入输出处理
//! - `transcript` - 对话记录，保存和加载会话历史
//! - `tui_v2` - gateway-first 终端用户界面（Terminal User Interface）
//! - `tui_utils` - TUI 工具函数和辅助方法
//!
//! # 导出项
//!
//! - `run` - 公开的运行入口函数，用于启动 CLI 代理
//! - `logo_text_lines` - Logo 文本行，用于显示欢迎信息
//! - `render_execution_indicator` - 执行指示器渲染，显示代理正在工作

pub(crate) mod interactive;
pub(crate) mod interactive_input;
#[cfg(test)]
#[path = "interactive_input_tests.rs"]
mod interactive_input_tests;
pub(crate) mod logo;
#[cfg(test)]
#[path = "logo_tests.rs"]
mod logo_tests;
pub(crate) mod processor;
pub(crate) mod render;
#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
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
pub(crate) mod stdio;
#[cfg(test)]
#[path = "stdio_tests.rs"]
mod stdio_tests;
pub(crate) mod theme;
#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
pub(crate) mod transcript;
pub(crate) mod tui_utils;
#[cfg(test)]
#[path = "tui_utils_tests.rs"]
mod tui_utils_tests;
pub(crate) mod tui_v2;

/// 导出主运行入口函数
///
/// 这是 CLI 代理的公开入口点，启动代理的执行循环并处理用户交互。
/// 外部代码应通过此函数启动代理的 CLI 模式。
pub use run::run;
