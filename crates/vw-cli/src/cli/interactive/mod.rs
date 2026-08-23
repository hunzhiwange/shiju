//! # CLI 交互式模式模块
//!
//! 本模块提供了 VibeWindow 代理的命令行交互式运行时环境。
//! 它实现了完整的 REPL（Read-Eval-Print Loop）交互循环，
//! 允许用户通过终端界面与代理进行实时对话和命令交互。
//!
//! ## 模块架构
//!
//! 交互式模式由以下几个核心子模块组成：
//!
//! - **`tui_v2`**: gateway-first 终端宿主，处理输入、渲染和运行时事件
//!
//! ## 主要功能
//!
//! - 实时对话：支持与 AI 代理的流式对话交互
//! - 命令系统：内置命令（如 `/help`）用于控制会话
//! - 会话管理：维护会话历史、上下文和状态
//! - TUI 渲染：基于 crossterm 的终端用户界面渲染
//! - 事件响应：键盘快捷键、鼠标滚动、窗口自适应
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::config::Config;
//! use crate::app::agent::agent::loop_::cli::setup::CliSetup;
//! use crate::app::agent::agent::loop_::cli::interactive::run_interactive;
//!
//! async fn start_interactive_session(config: &Config, setup: &CliSetup) -> anyhow::Result<()> {
//!     let mut final_output = String::new();
//!     run_interactive(config, setup, &mut final_output).await?;
//!     println!("会话输出: {}", final_output);
//!     Ok(())
//! }
//! ```
//!
//! ## 线程安全
//!
//! 本模块的交互循环在单个异步任务中运行，不涉及跨线程共享状态。
//! 所有状态变更都在事件循环的主控制流中顺序执行。

use crate::app::agent::config::Config;
use anyhow::Result;

use super::setup::CliSetup;
#[cfg(test)]
use self::tests::run_tui_v3;
#[cfg(not(test))]
use super::tui_v3::run_tui_v3;

/// 启动交互式命令行会话
///
/// 初始化并运行 CLI 交互式会话的主入口函数。该函数会接管终端控制，
/// 进入事件驱动的交互循环，直到用户显式退出或发生错误。
///
/// # 参数
///
/// - `config`: 代理配置引用，包含工作空间路径、模型设置等运行时配置
/// - `setup`: CLI 设置引用，包含 provider 名称、模型名称等环境信息
/// - `final_output`: 可变字符串引用，用于收集会话期间的最终输出内容，
///   在会话结束后可供调用方使用（如保存到文件或返回给上层）
///
/// # 返回值
///
/// - `Ok(())`: 会话正常结束（用户执行 `/exit` 或 `Ctrl+C`/`Ctrl+D`）
/// - `Err(e)`: 会话因错误而终止（如终端初始化失败、IO 错误等）
///
/// # 行为说明
///
/// 1. **终端接管**: 函数会初始化 TUI 并进入原始模式（raw mode）
/// 2. **事件循环**: 持续监听并处理键盘、鼠标和终端事件
/// 3. **状态管理**: 维护会话历史、输入缓冲区、滚动位置等状态
/// 4. **流式渲染**: 实时更新终端显示，展示对话记录和代理响应
/// 5. **资源清理**: 函数退出时自动恢复终端到正常模式
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::agent::loop_::cli::setup::CliSetup;
///
/// async fn run_cli_session(config: &Config) {
///     let setup = CliSetup::from_config(config);
///     let mut output = String::new();
///
///     match run_interactive(config, &setup, &mut output).await {
///         Ok(()) => println!("会话正常结束"),
///         Err(e) => eprintln!("会话错误: {}", e),
///     }
/// }
/// ```
///
/// # 注意事项
///
/// - 该函数会阻塞当前异步任务，直到会话结束
/// - 确保在调用前已正确配置日志重定向（避免日志污染 TUI）
/// - `final_output` 会在会话过程中被追加内容，而非覆盖
pub(crate) async fn run_interactive(
    config: &Config,
    setup: &CliSetup,
    _final_output: &mut String,
) -> Result<()> {
    run_tui_v3(config, setup)
}

#[cfg(test)]
mod tests;
