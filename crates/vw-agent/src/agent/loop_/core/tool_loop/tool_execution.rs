//! 执行已解析的工具调用并把结果追加回对话历史。
//!
//! 本模块位于工具循环中段：负责通道级工具禁用、重复调用去重、进度事件、
//! 顺序/并行调度，以及为下一轮 LLM 构造工具结果消息。

use crate::app::agent::observability::Observer;
use model_types::{ChatMessage, ToolCall};
use crate::app::agent::tools::{
    PendingToolCall, ScheduledToolBatchMode, Tool, ToolUseContext, schedule_tool_calls,
};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use super::super::super::cron::maybe_inject_cron_add_delivery;
use super::super::super::execution::{
    ToolExecutionOutcome, execute_tools_parallel, execute_tools_sequential,
};
use super::super::super::parsing::{ParsedToolCall, tool_call_signature};
use super::super::super::progress::{
    DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL, tool_progress_actions, tool_progress_label,
};
use super::super::super::utils::scrub_credentials;

#[cfg(test)]
#[path = "tool_execution_tests.rs"]
mod tool_execution_tests;

/// 执行本轮所有工具调用，并更新对话历史。
///
/// 参数包含当前历史、解析后的工具调用、工具注册表、观测器、通道上下文、
/// 取消令牌与去重集合。函数返回 `Ok(())` 表示已把助手消息和工具结果写回历史；
/// 工具执行调度或取消失败时返回错误。
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_tool_calls_and_update_history(
    history: &mut Vec<ChatMessage>,
    tool_calls: &[ParsedToolCall],
    assistant_history_content: String,
    native_tool_calls: &[ToolCall],
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    channel_name: &str,
    channel_reply_target: Option<&str>,
    provider_name: &str,
    model: &str,
    turn_id: &str,
    iteration: usize,
    tool_use_context: &Arc<ToolUseContext>,
    on_delta: Option<&Sender<String>>,
    excluded_tools: &[String],
    _bypass_non_cli_approval_for_turn: bool,
    cancellation_token: Option<&CancellationToken>,
    seen_tool_signatures: &mut HashSet<(String, String)>,
    use_native_tools: bool,
) -> Result<()> {
    agent_loop::core::tool_execution::execute_tool_calls_and_update_history(&services(), history, tool_calls, assistant_history_content, native_tool_calls, tools_registry, observer, channel_name, channel_reply_target, provider_name, model, turn_id, iteration, tool_use_context, on_delta, excluded_tools, _bypass_non_cli_approval_for_turn, cancellation_token, seen_tool_signatures, use_native_tools).await
}

#[cfg(not(target_arch = "wasm32"))]
type NativeHistory = crate::session::llm::native_tools::NativeHistoryItem;
#[cfg(target_arch = "wasm32")]
type NativeHistory = ();
fn services() -> agent_loop::core::tool_execution::ToolExecutionServices<crate::tools::context::AgentContextHost, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, NativeHistory> {
    use agent_loop::core::tool_execution::{ToolExecutionServices, CallView, OutcomeView, ToolBatch};
    ToolExecutionServices {
        call_view: |call| CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() },
        pending_view: |call| CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() },
        pending_call: |name, arguments, tool_call_id| PendingToolCall { name, arguments, tool_call_id },
        schedule: |calls, tools| schedule_tool_calls(calls, tools).into_iter().map(|batch| ToolBatch { parallel: batch.mode == ScheduledToolBatchMode::Parallel, calls: batch.calls }).collect(),
        outcome_view: |outcome| OutcomeView { tool_name: &outcome.tool_name, output: &outcome.output, success: outcome.success, error_reason: outcome.error_reason.as_deref(), result_dto: outcome.result_dto.as_ref(), duration: outcome.duration },
        take_output: |outcome| outcome.output,
        failure: |tool_name, output| ToolExecutionOutcome { tool_name, error_reason: Some(output.clone()), output, success: false, result_dto: None, duration: Duration::ZERO },
        execute_parallel: |calls, tools, observer, context, token| Box::pin(execute_tools_parallel(calls, tools, observer, context, token)),
        execute_sequential: |calls, tools, observer, context, token| Box::pin(execute_tools_sequential(calls, tools, observer, context, token)),
        inject_cron_delivery: maybe_inject_cron_add_delivery,
        call_signature: tool_call_signature, scrub_credentials,
        progress_actions: tool_progress_actions, progress_label: tool_progress_label,
        progress_sentinel: DRAFT_PROGRESS_SENTINEL, ws_event_sentinel: DRAFT_WS_EVENT_SENTINEL,
        #[cfg(not(target_arch = "wasm32"))]
        serialize_native: |item| Ok(serde_json::to_string(item)?),
        #[cfg(target_arch = "wasm32")]
        serialize_native: |_| anyhow::bail!("native tool search requires a non-wasm target"),
        #[cfg(not(target_arch = "wasm32"))]
        native_search_history: |discovery, call| {
            let typed_call = crate::session::llm::ToolCall {
                kind: crate::session::llm::ToolCallKind::Search,
                namespace: call.namespace.clone(),
                id: call.id.clone(),
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            };
            let aisdk::core::Message::ToolSearchOutput(output) =
                discovery.native_output(&typed_call)?
            else {
                anyhow::bail!("expected dedicated native search output");
            };
            use crate::session::llm::native_tools::NativeHistoryItem;
            let output = NativeHistoryItem::ToolSearchOutput(output);
            let call = NativeHistoryItem::ToolSearchCall(aisdk::core::ToolSearchCall {
                id: None, call_id: Some(call.id.clone()), status: Some("completed".into()),
                execution: aisdk::core::ToolSearchExecution::Client,
                arguments: serde_json::from_str(&call.arguments)?, encrypted_content: None,
            });
            Ok((output, call))
        },
    }
}

#[cfg(test)]
fn immediate_failure(name: String, id: Option<String>, output: String) -> (String, Option<String>, ToolExecutionOutcome) {
    agent_loop::core::tool_execution::immediate_failure(&services(), name, id, output)
}
#[cfg(test)]
async fn send_structured_tool_result_progress(tx: &Sender<String>, outcome: &ToolExecutionOutcome, secs: u64) {
    let services = services();
    let view = (services.outcome_view)(outcome);
    agent_loop::core::tool_execution::send_structured_tool_result_progress(tx, &view, secs, &services).await
}
