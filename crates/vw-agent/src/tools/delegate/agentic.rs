//! Agentic 委派执行路径。
//!
//! 本模块把委派 agent 的 provider/model 配置、能力策略和子循环执行连接起来。
//! 工具能力由共享策略解析，并显式排除再次调用 `delegate`，避免子 agent 递归委派
//! 导致不可控的执行树。

use super::super::traits::ToolResult;
use super::support::NoopObserver;
use super::{DELEGATE_AGENTIC_TIMEOUT_SECS, DelegateTool};
use crate::app::agent::agent::loop_::run_tool_call_loop;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::DelegateAgentConfig;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::tools::current_tool_use_context;
use std::sync::Arc;
use std::time::Duration;

/// 以 agentic 模式执行委派 agent。
///
/// # 参数
///
/// - `tool`: 父级 `DelegateTool`，提供可复用工具、安全策略和多模态配置。
/// - `agent_name`: 当前委派 agent 名称，用于错误和输出标识。
/// - `agent_config`: agent 的 provider/model/能力策略/迭代上限配置。
/// - `provider`: 实际执行模型调用的 provider。
/// - `system_prompt`: 可选系统提示词。
/// - `full_prompt`: 传给子 agent 的完整用户任务。
/// - `temperature`: 模型采样温度。
///
/// # 返回值
///
/// 返回 `ToolResult`，成功时包含 agent 输出，失败时把配置错误、循环错误或超时
/// 转换为工具错误文本。
///
/// # 错误
///
/// 仅在调用底层工具循环发生不可恢复错误传播时返回 `Err`；常见业务失败会被
/// 包装成 `ToolResult { success: false, ... }`。
pub(super) async fn execute_agentic(
    tool: &DelegateTool,
    agent_name: &str,
    agent_config: &DelegateAgentConfig,
    provider: &dyn Provider,
    system_prompt: Option<&str>,
    full_prompt: &str,
    temperature: f64,
) -> anyhow::Result<ToolResult> {
    let trusted_user_selected_tools = current_tool_use_context()
        .map(|context| context.trusted_user_selected_tools().iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let trusted_user_selected_skills = current_tool_use_context()
        .map(|context| context.trusted_user_selected_skills().iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let sub_tools = crate::app::agent::tools::delegated_tools::build_agentic_tools(
        &tool.parent_tools,
        agent_config,
        &trusted_user_selected_tools,
        &trusted_user_selected_skills,
    );

    if sub_tools.is_empty() {
        return Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Agent '{agent_name}' has no executable tools after applying its capability policy ({})",
                agent_config.allowed_tools.join(", ")
            )),
        });
    }

    let mut history = Vec::new();
    if let Some(system_prompt) = system_prompt {
        history.push(ChatMessage::system(system_prompt.to_string()));
    }
    history.push(ChatMessage::user(full_prompt.to_string()));

    let noop_observer = NoopObserver;
    let result = tokio::time::timeout(
        Duration::from_secs(DELEGATE_AGENTIC_TIMEOUT_SECS),
        run_tool_call_loop(
            provider,
            &mut history,
            &sub_tools,
            &noop_observer,
            &agent_config.provider,
            &agent_config.model,
            temperature,
            true,
            Option::<Arc<ApprovalManager>>::None,
            "delegate",
            &tool.multimodal_config,
            agent_config.max_iterations,
            None,
            None,
            Option::<Arc<HookRunner>>::None,
            Some(tool.security.clone()),
            &[],
        ),
    )
    .await;

    match result {
        Ok(Ok(response)) => {
            let rendered =
                if response.trim().is_empty() { "[Empty response]".to_string() } else { response };

            Ok(ToolResult {
                success: true,
                output: format!(
                    "[Agent '{agent_name}' ({provider}/{model}, agentic)]\n{rendered}",
                    provider = agent_config.provider,
                    model = agent_config.model
                ),
                error: None,
            })
        }
        Ok(Err(error)) => Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("Agent '{agent_name}' failed: {error}")),
        }),
        Err(_) => Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Agent '{agent_name}' timed out after {DELEGATE_AGENTIC_TIMEOUT_SECS}s"
            )),
        }),
    }
}
#[cfg(test)]
#[path = "agentic_tests.rs"]
mod agentic_tests;
