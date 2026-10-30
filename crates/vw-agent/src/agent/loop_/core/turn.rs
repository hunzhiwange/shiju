//! 代理轮次执行模块
//!
//! 本模块提供代理循环的单次轮次执行功能，负责协调消息发送、工具调用和响应处理。
//!
//! ## 主要功能
//!
//! - 执行代理的单次交互轮次
//! - 管理工具调用循环的上下文作用域
//! - 支持通道回复目标和审批上下文的传播
//!
//! ## 模块结构
//!
//! 本模块提供三个核心函数：
//! - [`agent_turn`] - 基础代理轮次执行
//! - [`run_tool_call_loop_with_reply_target`] - 带回复目标的工具循环
//! - [`run_tool_call_loop_with_non_cli_approval_context`] - 带非 CLI 审批上下文的工具循环

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::observability::Observer;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::super::approval::{NonCliApprovalContext, TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT};
use super::TOOL_LOOP_REPLY_TARGET;
use super::tool_loop::{run_tool_call_loop, run_tool_call_loop_with_skills};

#[cfg(test)]
#[path = "turn_tests.rs"]
mod turn_tests;

/// 执行代理循环的单次轮次
///
/// 该函数是代理交互的基础单元，负责：
/// 1. 向 LLM 提供者发送消息历史
/// 2. 解析并执行工具调用
/// 3. 循环执行直到 LLM 产生最终文本响应
///
/// # 参数
///
/// * `provider` - LLM 提供者的 trait 对象，负责与语言模型通信
/// * `history` - 可变引用的消息历史记录，用于维护对话上下文
/// * `tools_registry` - 可用工具的注册表
/// * `observer` - 观察者 trait 对象，用于日志和监控
/// * `provider_name` - 提供者名称标识符
/// * `model` - 使用的模型名称
/// * `temperature` - 生成温度参数，控制响应的随机性（0.0-2.0）
/// * `silent` - 是否抑制标准输出（为 true 时用于通道场景）
/// * `multimodal_config` - 多模态配置，控制图像等非文本输入
/// * `max_iterations` - 最大工具迭代次数，防止无限循环
/// * `hooks` - 当前轮次使用的钩子运行器快照
///
/// # 返回值
///
/// 返回 `Result<String>`，成功时包含 LLM 的最终文本响应，
/// 失败时返回错误信息。
///
/// # 示例
///
/// ```ignore
/// let response = agent_turn(
///     &provider,
///     &mut history,
///     &tools,
///     &observer,
///     "openai",
///     "gpt-4",
///     0.7,
///     false,
///     &multimodal_config,
///     10,
///     None,
/// ).await?;
/// println!("Agent response: {}", response);
/// ```
///
/// # 注意事项
///
/// - 该函数内部调用 `run_tool_call_loop`，传入默认参数
/// - 不包含审批管理器，适用于无需审批的场景
/// - 使用 "channel" 作为默认通道名称
#[allow(clippy::too_many_arguments)]
pub(crate) async fn agent_turn(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    hooks: Option<Arc<HookRunner>>,
    skills: Option<&mut super::skills::CoreSkillSession>,
) -> Result<String> {
    // 调用工具调用循环，使用默认参数（无审批管理器、无取消令牌等）
    run_tool_call_loop_with_skills(
        provider,
        history,
        tools_registry,
        observer,
        provider_name,
        model,
        temperature,
        silent,
        None,      // 无审批管理器
        "channel", // 默认通道名称
        multimodal_config,
        max_iterations,
        None, // 无取消令牌
        None, // 无增量回调
        hooks,
        None, // 无安全策略上下文
        &[],  // 无排除的工具
        skills,
    )
    .await
}

/// 运行带有通道回复目标上下文的工具循环
///
/// 该函数用于通道运行时，自动填充计划提醒的投递路由信息。
/// 通过 `TOOL_LOOP_REPLY_TARGET` 作用域将回复目标传递给工具循环。
///
/// # 参数
///
/// * `provider` - LLM 提供者的 trait 对象
/// * `history` - 可变引用的消息历史记录
/// * `tools_registry` - 可用工具的注册表
/// * `observer` - 观察者 trait 对象
/// * `provider_name` - 提供者名称标识符
/// * `model` - 使用的模型名称
/// * `temperature` - 生成温度参数
/// * `silent` - 是否抑制标准输出
/// * `approval` - 可选的审批管理器引用
/// * `channel_name` - 通道名称标识符
/// * `reply_target` - 可选的回复目标地址，用于后续消息投递
/// * `multimodal_config` - 多模态配置
/// * `max_iterations` - 最大工具迭代次数
/// * `cancellation_token` - 可选的取消令牌，用于提前终止执行
/// * `on_delta` - 可选的增量响应发送器，用于流式输出
/// * `hooks` - 可选的钩子执行器引用
/// * `excluded_tools` - 排除的工具名称列表
///
/// # 返回值
///
/// 返回 `Result<String>`，包含最终响应或错误信息。
///
/// # 示例
///
/// ```ignore
/// let response = run_tool_call_loop_with_reply_target(
///     &provider,
///     &mut history,
///     &tools,
///     &observer,
///     "openai",
///     "gpt-4",
///     0.7,
///     true,
///     Some(&approval_manager),
///     "telegram",
///     Some("chat_12345"),
///     &multimodal_config,
///     10,
///     Some(cancellation_token),
///     Some(delta_sender),
///     Some(&hook_runner),
///     &["dangerous_tool".to_string()],
/// ).await?;
/// ```
///
/// # 实现细节
///
/// 该函数使用 `TOOL_LOOP_REPLY_TARGET` 的 `scope` 方法创建一个作用域，
/// 在作用域内执行工具循环，确保回复目标在整个调用链中可用。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_tool_call_loop_with_reply_target(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    reply_target: Option<&str>,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    // 使用 reply_target 作用域包装工具调用循环
    // 这确保在工具执行期间可以访问回复目标信息
    TOOL_LOOP_REPLY_TARGET
        .scope(
            // 将 &str 转换为 String 以满足作用域要求
            reply_target.map(str::to_string),
            run_tool_call_loop(
                provider,
                history,
                tools_registry,
                observer,
                provider_name,
                model,
                temperature,
                silent,
                approval,
                channel_name,
                multimodal_config,
                max_iterations,
                cancellation_token,
                on_delta,
                hooks,
                security,
                excluded_tools,
            ),
        )
        .await
}

/// 运行带有非 CLI 审批上下文的工具循环
///
/// 该函数为任务提供可选的非 CLI 审批上下文作用域，用于需要审批但不在
/// 命令行交互环境中的场景（如通道触发的工具调用）。
///
/// # 参数
///
/// * `provider` - LLM 提供者的 trait 对象
/// * `history` - 可变引用的消息历史记录
/// * `tools_registry` - 可用工具的注册表
/// * `observer` - 观察者 trait 对象
/// * `provider_name` - 提供者名称标识符
/// * `model` - 使用的模型名称
/// * `temperature` - 生成温度参数
/// * `silent` - 是否抑制标准输出
/// * `approval` - 可选的审批管理器引用
/// * `channel_name` - 通道名称标识符
/// * `non_cli_approval_context` - 可选的非 CLI 审批上下文，包含审批所需信息
/// * `multimodal_config` - 多模态配置
/// * `max_iterations` - 最大工具迭代次数
/// * `cancellation_token` - 可选的取消令牌
/// * `on_delta` - 可选的增量响应发送器
/// * `hooks` - 可选的钩子执行器引用
/// * `excluded_tools` - 排除的工具名称列表
///
/// # 返回值
///
/// 返回 `Result<String>`，包含最终响应或错误信息。
///
/// # 示例
///
/// ```ignore
/// let approval_ctx = NonCliApprovalContext {
///     reply_target: Some("chat_12345".to_string()),
///     // 其他审批相关字段...
/// };
///
/// let response = run_tool_call_loop_with_non_cli_approval_context(
///     &provider,
///     &mut history,
///     &tools,
///     &observer,
///     "openai",
///     "gpt-4",
///     0.7,
///     false,
///     Some(&approval_manager),
///     "discord",
///     Some(approval_ctx),
///     &multimodal_config,
///     10,
///     None,
///     None,
///     None,
///     &[],
/// ).await?;
/// ```
///
/// # 实现细节
///
/// 该函数同时设置两个作用域：
/// 1. `TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT` - 非 CLI 审批上下文
/// 2. `TOOL_LOOP_REPLY_TARGET` - 从审批上下文中提取的回复目标
///
/// 这种嵌套作用域确保了审批机制和消息路由在整个工具调用链中可用。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_tool_call_loop_with_non_cli_approval_context(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    non_cli_approval_context: Option<NonCliApprovalContext>,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    // 从审批上下文中提取回复目标
    // 如果存在审批上下文，则使用其中的 reply_target；否则为 None
    let reply_target = non_cli_approval_context.as_ref().map(|ctx| ctx.reply_target.clone());

    // 嵌套使用两个作用域：
    // 外层：非 CLI 审批上下文作用域
    // 内层：回复目标作用域
    // 这确保了在工具执行期间可以同时访问审批信息和回复目标
    TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT
        .scope(
            non_cli_approval_context,
            TOOL_LOOP_REPLY_TARGET.scope(
                reply_target,
                run_tool_call_loop(
                    provider,
                    history,
                    tools_registry,
                    observer,
                    provider_name,
                    model,
                    temperature,
                    silent,
                    approval,
                    channel_name,
                    multimodal_config,
                    max_iterations,
                    cancellation_token,
                    on_delta,
                    hooks,
                    security,
                    excluded_tools,
                ),
            ),
        )
        .await
}
