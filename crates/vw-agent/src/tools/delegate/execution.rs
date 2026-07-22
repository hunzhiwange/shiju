//! 委派工具的执行入口。
//!
//! 本模块负责校验委派参数、执行安全策略、创建模型 provider，并根据代理配置
//! 选择普通聊天或 agentic 执行路径。协调追踪只记录执行状态，不影响主结果。

use super::super::traits::ToolResult;
use super::{DELEGATE_TIMEOUT_SECS, DelegateTool};
use crate::app::agent::providers::{Provider, ProviderRuntimeOptions};
use crate::app::agent::security::policy::ToolOperation;
use std::time::Duration;

pub(super) async fn execute(
    tool: &DelegateTool,
    args: serde_json::Value,
) -> anyhow::Result<ToolResult> {
    execute_with_provider_factory(tool, args, &|provider, credential, options| {
        crate::app::agent::providers::create_provider_with_options(provider, credential, options)
    })
    .await
}

type ProviderFactory<'a> = dyn Fn(&str, Option<&str>, &ProviderRuntimeOptions) -> anyhow::Result<Box<dyn Provider>>
    + Send
    + Sync
    + 'a;

async fn execute_with_provider_factory(
    tool: &DelegateTool,
    args: serde_json::Value,
    create_provider: &ProviderFactory<'_>,
) -> anyhow::Result<ToolResult> {
    let called_via_agent_tool =
        args.get("_via_agent_tool").and_then(|value| value.as_bool()).unwrap_or(false);
    let agent_name = args
        .get("agent")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .ok_or_else(|| anyhow::anyhow!("Missing 'agent' parameter"))?;
    if agent_name.is_empty() {
        return Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some("'agent' parameter must not be empty".into()),
        });
    }

    let prompt = args
        .get("prompt")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;
    if prompt.is_empty() {
        return Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some("'prompt' parameter must not be empty".into()),
        });
    }

    let context = args.get("context").and_then(|value| value.as_str()).map(str::trim).unwrap_or("");
    let agent_config = match tool.agents.get(agent_name) {
        Some(config) => config,
        None => {
            let available: Vec<&str> = tool.agents.keys().map(|name| name.as_str()).collect();
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Unknown agent '{agent_name}'. Available agents: {}",
                    if available.is_empty() {
                        "(none configured)".to_string()
                    } else {
                        available.join(", ")
                    }
                )),
            });
        }
    };

    if tool.depth >= agent_config.max_depth {
        return Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Delegation depth limit reached ({depth}/{max}). \
                 Cannot delegate further to prevent infinite loops.",
                depth = tool.depth,
                max = agent_config.max_depth
            )),
        });
    }

    // 直接调用 delegate 属于主动行为；由 AgentTool 内部转发时，上层已经完成了
    // 操作权限判断，避免重复拦截同一次委派。
    if !called_via_agent_tool
        && let Err(error) = tool.security.enforce_tool_operation(ToolOperation::Act, "delegate")
    {
        return Ok(ToolResult { success: false, output: String::new(), error: Some(error) });
    }

    let coordination_trace =
        tool.start_coordination_trace(agent_name, prompt, context, agent_config);

    // 优先使用子代理显式凭据，缺省时才回退到工具级凭据，避免无意覆盖代理配置。
    let provider_credential_owned =
        agent_config.api_key.clone().or_else(|| tool.fallback_credential.clone());
    #[allow(clippy::option_as_ref_deref)]
    let provider_credential = provider_credential_owned.as_ref().map(String::as_str);

    let provider: Box<dyn Provider> = match create_provider(
        &agent_config.provider,
        provider_credential,
        &tool.provider_runtime_options,
    ) {
        Ok(provider) => provider,
        Err(error) => {
            let error_message = format!(
                "Failed to create provider '{}' for agent '{agent_name}': {error}",
                agent_config.provider
            );
            tool.finish_coordination_trace(agent_name, &coordination_trace, false, &error_message);
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error_message),
            });
        }
    };

    let full_prompt = if context.is_empty() {
        prompt.to_string()
    } else {
        format!("[Context]\n{context}\n\n[Task]\n{prompt}")
    };
    let temperature = agent_config.temperature.unwrap_or(0.7);
    let merged_system_prompt =
        tool.merged_system_prompt(agent_name, agent_config.system_prompt.as_deref());

    if agent_config.agentic {
        let result = tool
            .execute_agentic(
                agent_name,
                agent_config,
                &*provider,
                merged_system_prompt.as_deref(),
                &full_prompt,
                temperature,
            )
            .await?;

        let summary = if result.success {
            result.output.as_str()
        } else {
            result.error.as_deref().unwrap_or("delegate agentic execution failed")
        };
        tool.finish_coordination_trace(agent_name, &coordination_trace, result.success, summary);
        return Ok(result);
    }

    // 非 agentic 路径没有内部迭代边界，因此在外层包一层超时，防止 provider
    // 长时间无响应占住工具执行槽。
    let result = tokio::time::timeout(
        Duration::from_secs(DELEGATE_TIMEOUT_SECS),
        provider.chat_with_system(
            merged_system_prompt.as_deref(),
            &full_prompt,
            &agent_config.model,
            temperature,
        ),
    )
    .await;

    let result = match result {
        Ok(inner) => inner,
        Err(_) => {
            let timeout_message =
                format!("Agent '{agent_name}' timed out after {DELEGATE_TIMEOUT_SECS}s");
            tool.finish_coordination_trace(
                agent_name,
                &coordination_trace,
                false,
                &timeout_message,
            );
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(timeout_message),
            });
        }
    };

    match result {
        Ok(response) => {
            let mut rendered = response;
            if rendered.trim().is_empty() {
                rendered = "[Empty response]".to_string();
            }

            let output = format!(
                "[Agent '{agent_name}' ({provider}/{model})]\n{rendered}",
                provider = agent_config.provider,
                model = agent_config.model
            );
            tool.finish_coordination_trace(agent_name, &coordination_trace, true, &output);
            Ok(ToolResult { success: true, output, error: None })
        }
        Err(error) => {
            let failure_message = format!("Agent '{agent_name}' failed: {error}");
            tool.finish_coordination_trace(
                agent_name,
                &coordination_trace,
                false,
                &failure_message,
            );
            Ok(ToolResult { success: false, output: String::new(), error: Some(failure_message) })
        }
    }
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;
