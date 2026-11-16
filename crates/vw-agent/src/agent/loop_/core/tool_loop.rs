//! # 工具调用循环核心模块
//!
//! 本模块保留工具循环的对外入口，并将 LLM 轮次处理、进度输出、
//! 工具执行和完成声明检测拆分到 `tool_loop/` 子模块，降低单文件职责密度。

#[cfg(test)]
mod collaboration;
#[cfg(test)]
mod detection;
mod llm_round;
mod tool_execution;

#[cfg(test)]
#[path = "tool_loop/discovery_tests.rs"]
mod discovery_tests;

#[cfg(test)]
#[path = "tool_loop_tests.rs"]
mod tool_loop_tests;

#[cfg(test)]
pub(crate) use detection::looks_like_unverified_action_completion_without_tool_call;

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::observability::Observer;
use model_types::{ChatMessage, Provider};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
#[cfg(test)]
use crate::app::agent::tools::ToolUseContext;
use crate::tools::tool_search::ToolDiscoverySession;
use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::super::approval::TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT;
use super::super::utils::scrub_credentials;
use super::TOOL_LOOP_REPLY_TARGET;
#[cfg(test)]
use super::constants::EMPTY_FINAL_RESPONSE_RETRY_PROMPT;
#[cfg(test)]
use super::errors::ToolLoopCancelled;

/// 执行代理工具调用循环的核心函数。
#[allow(clippy::too_many_arguments)]
pub async fn run_tool_call_loop(
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
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    run_tool_call_loop_with_skills(
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
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_tool_call_loop_with_skills(
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
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
    skills: Option<&mut super::skills::CoreSkillSession>,
) -> Result<String> {
    agent_loop::core::tool_loop::run_tool_call_loop_with_skills::<super::skills::NativeCoreSkillHost, _, _>(&services(), provider, history, tools_registry, observer, provider_name, model, temperature, silent, approval, channel_name, multimodal_config, max_iterations, cancellation_token, on_delta, hooks, security, excluded_tools, skills).await
}

fn services() -> agent_loop::core::tool_loop::LoopServices<crate::tools::context::AgentContextHost, super::super::parsing::ParsedToolCall> {
    use agent_loop::core::tool_loop::LoopServices;
    use super::super::progress::{DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL};
    use crate::session::child_agent::coordinator::global_child_agent_coordinator;
    LoopServices {
        wire: agent_loop::core::output::ProgressWire { clear: DRAFT_CLEAR_SENTINEL, progress: DRAFT_PROGRESS_SENTINEL },
        non_cli_approval_context: || TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT.try_with(Clone::clone).ok().flatten(),
        reply_target: || TOOL_LOOP_REPLY_TARGET.try_with(Clone::clone).ok().flatten(),
        new_discovery: ToolDiscoverySession::new,
        rebase_discovery: |discovery, specs| discovery.rebase(specs),
        exposed_specs: |discovery| discovery.exposed_specs(),
        refresh_catalog: |session, workspace, catalog| Box::pin(async move {
            crate::plugins::mcp_session_connections(session, workspace).refresh_tool_catalog(catalog).await
        }),
        new_extractor: || Box::new(crate::multimodal::ocr::SessionImageTextExtractor::new()),
        exposed_instructions: super::super::instructions::build_exposed_tool_instructions,
        scrub_credentials,
        llm_round: |r| Box::pin(llm_round::run_llm_round(r.provider, r.history, r.observer, r.provider_name, r.model, r.temperature, r.channel_name, r.multimodal_config, r.cancellation_token, r.hooks, r.tool_specs, r.use_native_tools, r.turn_id, r.iteration)),
        execute_tools: |r| Box::pin(tool_execution::execute_tool_calls_and_update_history(r.history, r.tool_calls, r.assistant_history_content, r.native_tool_calls, r.tools_registry, r.observer, r.channel_name, r.channel_reply_target, r.provider_name, r.model, r.turn_id, r.iteration, r.tool_use_context, r.on_delta, r.excluded_tools, r.bypass_non_cli_approval_for_turn, r.cancellation_token, r.seen_tool_signatures, r.use_native_tools)),
        stop_hook: |context, interrupt, active, message| Box::pin(crate::plugins::hook_stop::emit(context, interrupt, active, message)),
        cancel_descendants: |session, children| global_child_agent_coordinator().cancel_descendants(session, children),
        descendant_snapshots: |session, children| global_child_agent_coordinator().descendant_snapshots(session, children),
        wait_for_descendants: |session, children, timeout| Box::pin(async move { global_child_agent_coordinator().wait_for_descendants(session, children, timeout).await }),
        deliver_model_events: |context, events| global_child_agent_coordinator().deliver_model_events(context, events),
    }
}
