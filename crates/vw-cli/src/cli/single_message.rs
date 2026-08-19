//! CLI 单次消息处理模块
//!
//! 本模块提供 CLI 环境下的单次消息处理能力。与交互式会话不同，单次消息模式
//! 接收一个用户输入，执行完整的代理循环（包括上下文构建、工具调用等），
//! 然后返回最终响应。
//!
//! # 主要功能
//!
//! - **自动记忆存储**: 根据配置自动保存用户消息到记忆系统
//! - **上下文增强**: 从记忆系统中检索相关上下文，丰富用户输入
//! - **工具调用循环**: 执行完整的工具调用迭代，直到代理返回最终响应
//! - **观测性支持**: 记录会话完成事件，支持可观测性追踪
//!
//! # 使用场景
//!
//! - 非交互式 CLI 调用（如脚本集成）
//! - 一次性查询处理
//! - 自动化工作流中的代理调用

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::Config;
use crate::app::agent::memory::MemoryCategory;
use crate::app::agent::providers::ChatMessage;
use crate::app::agent::security::SecurityPolicy;
use anyhow::Result;
use std::sync::Arc;

use super::setup::CliSetup;
use crate::app::agent::agent::loop_::context::build_context;
use crate::app::agent::agent::loop_::core::{
    AUTOSAVE_MIN_MESSAGE_CHARS, autosave_memory_key, run_tool_call_loop,
};

/// 执行单次消息处理循环
///
/// 该函数是 CLI 单次消息模式的核心入口点。它接收用户消息，执行完整的代理处理流程，
/// 包括记忆存储、上下文构建、工具调用迭代，最终返回代理的响应文本。
///
/// # 处理流程
///
/// 1. **记忆存储**: 如果启用了自动保存且消息长度达标，将用户消息存入记忆系统
/// 2. **上下文构建**: 从记忆系统检索相关上下文，增强用户输入
/// 3. **时间戳标记**: 为消息添加当前时间戳，便于追踪
/// 4. **历史初始化**: 创建包含系统提示和用户消息的对话历史
/// 5. **工具调用循环**: 执行代理的完整工具调用迭代
/// 6. **响应输出**: 将最终响应打印到标准输出并返回
/// 7. **事件记录**: 记录会话完成事件
///
/// # 参数
///
/// * `config` - 代理配置引用，包含记忆设置、模型参数等配置项
/// * `setup` - CLI 运行时设置，包含 provider、记忆存储、工具注册表等组件
/// * `message` - 用户输入的消息内容
/// * `temperature` - 模型生成的温度参数，控制响应的随机性
///   - 较低的值（如 0.0-0.3）产生更确定性的输出
///   - 较高的值（如 0.7-1.0）产生更有创意/多变的输出
///
/// # 返回值
///
/// 返回 `Result<String>`，其中：
/// - `Ok(String)` - 代理处理后的最终响应文本
/// - `Err` - 处理过程中发生的任何错误（如 provider 调用失败、工具执行错误等）
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::agent::loop_::cli::setup::CliSetup;
///
/// async fn example() -> anyhow::Result<()> {
///     let config = Config::load()?;
///     let setup = CliSetup::initialize(&config).await?;
///
///     let response = run_single_message(
///         &config,
///         &setup,
///         "请帮我分析这段代码的性能问题".to_string(),
///         0.7,
///     ).await?;
///
///     println!("代理响应: {}", response);
///     Ok(())
/// }
/// ```
///
/// # 注意事项
///
/// - 记忆存储失败不会中断主流程（使用 `let _ =` 忽略错误）
/// - 响应会自动打印到标准输出，同时通过返回值提供
/// - 单次消息模式不启用自动批准，所有敏感工具调用需要显式确认
pub(crate) async fn run_single_message(
    config: &Config,
    setup: &CliSetup,
    message: String,
    temperature: f64,
) -> Result<String> {
    // 记忆自动存储：当启用自动保存且消息足够长时，保存用户消息到记忆系统
    // 这允许代理在后续交互中引用当前对话内容
    // 注意：存储失败被静默忽略，不影响主流程继续执行
    if config.memory.auto_save && message.chars().count() >= AUTOSAVE_MIN_MESSAGE_CHARS {
        let user_key = autosave_memory_key("user_msg");
        let _ = setup.mem.store(&user_key, &message, MemoryCategory::Conversation, None).await;
    }

    // 上下文增强：从记忆系统检索与当前消息相关的历史上下文
    // 相似度阈值由配置中的 min_relevance_score 控制
    let mem_context: String =
        build_context(setup.mem.as_ref(), &message, config.memory.min_relevance_score).await;

    // 时间戳标记：记录消息接收时间，便于日志追踪和调试
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z");

    // 消息格式化：根据是否存在记忆上下文，构建最终的用户消息
    // 如果有相关记忆，将其前置于用户消息；否则仅包含时间戳和原始消息
    let enriched = if mem_context.is_empty() {
        format!("[{now}] {message}")
    } else {
        format!("{mem_context}[{now}] {message}")
    };

    // 初始化对话历史：包含系统提示（定义代理行为）和增强后的用户消息
    let mut history = vec![ChatMessage::system(&setup.system_prompt), ChatMessage::user(&enriched)];

    // 执行工具调用循环：这是代理的核心处理逻辑
    // 会迭代调用模型和工具，直到代理返回最终响应或达到最大迭代次数
    // 参数说明：
    // - provider: 模型提供者
    // - history: 对话历史（可变，会被工具调用迭代更新）
    // - tools_registry: 可用工具注册表
    // - observer: 可观测性记录器
    // - provider_name/model_name: 模型标识
    // - temperature: 生成随机性控制
    // - false: 不启用自动批准模式（单次消息需要显式确认）
    // - approval_manager: 工具审批管理器
    // - channel_name: 通道名称（CLI）
    // - multimodal: 多模态配置
    // - max_iterations: 最大工具调用迭代次数
    // - 后续 None/空参数: 不指定特定约束
    let approval = Some(Arc::new(ApprovalManager::from_config(&config.autonomy)));
    let security = Some(Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    )));

    let response = run_tool_call_loop(
        setup.provider.as_ref(),
        &mut history,
        &setup.tools_registry,
        setup.observer.as_ref(),
        &setup.provider_name,
        &setup.model_name,
        temperature,
        false, // 单次消息模式不启用自动批准
        approval,
        setup.channel_name,
        &config.multimodal,
        vw_config_types::agent::effective_main_agent_config(&config.agents).max_iterations,
        None, // 无特定系统提示覆盖
        None, // 无特定工具约束
        None, // 无特定上下文约束
        security,
        &[], // 无额外消息注入
    )
    .await?;

    // 输出响应到标准输出（CLI 交互）
    println!("{response}");

    // 记录会话完成事件，用于可观测性和指标收集
    setup.observer.record_event(&crate::app::agent::observability::ObserverEvent::TurnComplete);

    Ok(response)
}
