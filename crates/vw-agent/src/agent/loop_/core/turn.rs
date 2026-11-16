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
use model_types::{ChatMessage, Provider};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::super::approval::NonCliApprovalContext;

#[cfg(test)]
#[path = "turn_tests.rs"]
mod turn_tests;

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
    agent_loop::core::turn::agent_turn::<super::skills::NativeCoreSkillHost, _, _>(
        &super::tool_loop::services(), provider, history, tools_registry, observer,
        provider_name, model, temperature, silent, multimodal_config,
        max_iterations, hooks, skills,
    ).await
}

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
    agent_loop::core::turn::run_tool_call_loop_with_reply_target::<super::skills::NativeCoreSkillHost, _, _>(
        &super::tool_loop::services(), provider, history, tools_registry, observer,
        provider_name, model, temperature, silent, approval, channel_name, reply_target,
        multimodal_config, max_iterations, cancellation_token, on_delta, hooks, security,
        excluded_tools,
    ).await
}

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
    agent_loop::core::turn::run_tool_call_loop_with_non_cli_approval_context::<super::skills::NativeCoreSkillHost, _, _>(
        &super::tool_loop::services(), provider, history, tools_registry, observer,
        provider_name, model, temperature, silent, approval, channel_name, non_cli_approval_context,
        multimodal_config, max_iterations, cancellation_token, on_delta, hooks, security,
        excluded_tools,
    ).await
}
