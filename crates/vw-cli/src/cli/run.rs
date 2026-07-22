//! CLI 运行时入口模块
//!
//! 本模块提供 CLI 模式下代理执行的主入口点，负责协调单次消息处理与交互式会话两种运行模式。
//!
//! # 主要功能
//!
//! - **单次消息模式**：处理单条用户消息并返回响应，适用于脚本化调用或一次性查询
//! - **交互式模式**：启动持续的用户交互会话，支持多轮对话
//!
//! # 架构位置
//!
//! ```text
//! cli/run.rs (本模块)
//!     ├── cli/setup.rs      → 初始化 Provider、Model、Observer 等
//!     ├── cli/single_message.rs → 单次消息处理
//!     └── cli/interactive.rs    → 交互式会话
//! ```

#[cfg(not(test))]
use super::interactive::run_interactive;
#[cfg(test)]
use super::run_tests::{run_interactive, run_single_message, setup_cli};
#[cfg(not(test))]
use super::setup::setup_cli;
#[cfg(not(test))]
use super::single_message::run_single_message;
use crate::app::agent::config::Config;
use crate::app::agent::observability::ObserverEvent;
use anyhow::Result;
use std::time::Instant;

/// 执行 CLI 模式的代理运行时
///
/// 根据输入参数选择单次消息处理或交互式会话模式，并负责整个执行周期的
/// 资源初始化、运行时监控和终止事件记录。
///
/// # 参数
///
/// - `config`: 代理的完整配置对象，包含 Provider、Memory、Security 等子系统配置
/// - `message`: 可选的单次消息内容。若提供，则执行单次消息模式；否则进入交互式模式
/// - `provider_override`: 可选的 Provider 覆盖名称，用于临时替换配置中的默认 Provider
/// - `model_override`: 可选的模型名称覆盖，用于临时替换配置中的默认模型
/// - `temperature`: 生成温度参数，控制模型输出的随机性（0.0-2.0，值越高越随机）
/// - `_peripheral_overrides`: 外围设备覆盖配置（当前未使用，保留用于未来扩展）
/// - `interactive`: 是否强制启用交互式模式标志
///
/// # 返回值
///
/// 返回 `Result<String>`，其中：
/// - `Ok(String)`: 执行成功，包含最终输出文本
/// - `Err`: 执行过程中发生错误
///
/// # 执行流程
///
/// 1. 调用 [`setup_cli`] 初始化运行时环境（Provider、Model、Observer）
/// 2. 记录开始时间用于性能监控
/// 3. 根据是否有消息输入选择执行模式：
///    - 有消息 → 调用 [`run_single_message`] 处理单次请求
///    - 无消息 → 调用 [`run_interactive`] 启动交互式会话
/// 4. 记录 [`ObserverEvent::AgentEnd`] 事件用于可观测性
/// 5. 返回最终输出
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::agent::loop_::cli::run::run;
///
/// async fn example() -> anyhow::Result<()> {
///     let config = Config::load("config.toml")?;
///
///     // 单次消息模式
///     let response = run(
///         config.clone(),
///         Some("你好，请介绍一下自己".to_string()),
///         None,  // 使用默认 Provider
///         None,  // 使用默认模型
///         0.7,   // 温度
///         vec![],
///         false,
///     ).await?;
///     println!("Response: {}", response);
///
///     Ok(())
/// }
/// ```
pub async fn run(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
    _peripheral_overrides: Vec<String>,
    interactive: bool,
) -> Result<String> {
    // 初始化 CLI 运行时环境
    // 包括：Provider 实例化、模型选择、Observer 配置等
    let setup =
        setup_cli(&config, interactive, provider_override.as_deref(), model_override.as_deref())?;

    // 记录执行开始时间，用于后续计算总耗时
    let start = Instant::now();

    // 初始化最终输出字符串
    // 将在单次消息或交互式会话中被填充
    let mut final_output = String::new();

    // 根据是否提供消息内容选择执行模式
    if let Some(msg) = message {
        // 单次消息模式：处理单条消息并直接返回响应
        let response = run_single_message(&config, &setup, msg, temperature).await?;
        final_output = response;
    } else {
        // 交互式模式：启动持续的用户交互会话
        // 输出将通过回调或引用参数累积到 final_output
        run_interactive(&config, &setup, &mut final_output).await?;
    }

    // 计算总执行耗时
    let duration = start.elapsed();

    // 记录代理执行终止事件
    // 此事件将被 Observer 捕获并转发到配置的可观测性后端
    // tokens_used 和 cost_usd 当前未在 CLI 模式下追踪
    setup.observer.record_event(&ObserverEvent::AgentEnd {
        provider: setup.provider_name.to_string(),
        model: setup.model_name.to_string(),
        duration,
        tokens_used: None,
        cost_usd: None,
    });

    Ok(final_output)
}
