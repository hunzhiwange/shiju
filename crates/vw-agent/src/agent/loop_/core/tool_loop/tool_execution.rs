//! 执行已解析的工具调用并把结果追加回对话历史。
//!
//! 本模块位于工具循环中段：负责通道级工具禁用、重复调用去重、进度事件、
//! 顺序/并行调度，以及为下一轮 LLM 构造工具结果消息。

use crate::app::agent::observability::{Observer, runtime_trace};
use crate::app::agent::providers::{ChatMessage, ToolCall};
use crate::app::agent::tools::{
    PendingToolCall, ScheduledToolBatchMode, Tool, ToolResultHistoryEntry, ToolUseContext,
    build_tool_result_history_messages, schedule_tool_calls,
};
use anyhow::Result;
use serde_json::json;
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
    // 每轮工具执行都携带当前历史快照，工具可以基于一致视图读取上下文，而不会
    // 观察到本轮尚未写回的部分结果。
    let iteration_tool_use_context = Arc::new(
        tool_use_context
            .as_ref()
            .clone()
            .with_iteration(iteration)
            .with_messages_view(history.clone()),
    );
    let mut ordered_results: Vec<Option<(String, Option<String>, ToolExecutionOutcome)>> =
        (0..tool_calls.len()).map(|_| None).collect();
    let mut executable_indices = Vec::new();
    let mut executable_calls: Vec<PendingToolCall> = Vec::new();

    for (idx, call) in tool_calls.iter().enumerate() {
        let tool_name = call.name.clone();
        let mut tool_args = call.arguments.clone();

        maybe_inject_cron_add_delivery(
            &tool_name,
            &mut tool_args,
            channel_name,
            channel_reply_target,
        );

        if excluded_tools.iter().any(|excluded| excluded == &tool_name) {
            // 通道策略是默认拒绝边界：被排除的工具不能进入调度层，错误结果仍写回
            // 历史，让模型知道该能力在当前通道不可用。
            let blocked = format!("Tool '{tool_name}' is not available in this channel.");
            runtime_trace::record_event(
                "tool_call_result",
                Some(channel_name),
                Some(provider_name),
                Some(model),
                Some(turn_id),
                Some(false),
                Some(&blocked),
                serde_json::json!({
                    "iteration": iteration + 1,
                    "tool": tool_name.clone(),
                    "arguments": scrub_credentials(&tool_args.to_string()),
                    "blocked_by_channel_policy": true,
                }),
            );
            ordered_results[idx] = Some(immediate_failure(
                tool_name.clone(),
                history_tool_call_id(
                    call.tool_call_id.as_deref(),
                    use_native_tools,
                    iteration,
                    idx,
                ),
                blocked,
            ));
            continue;
        }

        let mut signature = tool_call_signature(&tool_name, &tool_args);
        if let Some(tool_call_id) =
            call.tool_call_id.as_deref().filter(|id| !id.starts_with("fallback_"))
        {
            signature.1 = format!("{}#{}", signature.1, tool_call_id);
        }
        if !seen_tool_signatures.insert(signature) {
            // 同一 turn 内重复的工具调用通常是模型重发，直接跳过可避免重复副作用。
            let duplicate = format!(
                "Skipped duplicate tool call '{tool_name}' with identical arguments in this turn."
            );
            runtime_trace::record_event(
                "tool_call_result",
                Some(channel_name),
                Some(provider_name),
                Some(model),
                Some(turn_id),
                Some(false),
                Some(&duplicate),
                serde_json::json!({
                    "iteration": iteration + 1,
                    "tool": tool_name.clone(),
                    "arguments": scrub_credentials(&tool_args.to_string()),
                    "deduplicated": true,
                }),
            );
            ordered_results[idx] = Some(immediate_failure(
                tool_name,
                history_tool_call_id(
                    call.tool_call_id.as_deref(),
                    use_native_tools,
                    iteration,
                    idx,
                ),
                duplicate,
            ));
            continue;
        }

        runtime_trace::record_event(
            "tool_call_start",
            Some(channel_name),
            Some(provider_name),
            Some(model),
            Some(turn_id),
            None,
            None,
            serde_json::json!({
                "iteration": iteration + 1,
                "tool": tool_name.clone(),
                "arguments": scrub_credentials(&tool_args.to_string()),
            }),
        );

        if let Some(tx) = on_delta {
            let (start_action, _) = tool_progress_actions(&tool_name);
            let label = tool_progress_label(&tool_name, &tool_args);
            let progress = format!("⏳ {} · {}\n", start_action, label);
            let _ = tx.send(format!("{DRAFT_PROGRESS_SENTINEL}{progress}")).await;
        }

        executable_indices.push(idx);
        executable_calls.push(PendingToolCall {
            name: tool_name,
            arguments: tool_args,
            tool_call_id: call.tool_call_id.clone(),
        });
    }

    let mut executed_outcomes = Vec::with_capacity(executable_calls.len());
    for batch in schedule_tool_calls(&executable_calls, tools_registry) {
        // 调度器根据工具声明决定并行或顺序执行，既保留可并发工具的效率，也避免
        // 对有副作用或顺序要求的工具打乱执行次序。
        let mut batch_outcomes = match batch.mode {
            ScheduledToolBatchMode::Parallel => {
                execute_tools_parallel(
                    &batch.calls,
                    tools_registry,
                    observer,
                    iteration_tool_use_context.clone(),
                    cancellation_token,
                )
                .await?
            }
            ScheduledToolBatchMode::Sequential => {
                execute_tools_sequential(
                    &batch.calls,
                    tools_registry,
                    observer,
                    iteration_tool_use_context.clone(),
                    cancellation_token,
                )
                .await?
            }
        };
        executed_outcomes.append(&mut batch_outcomes);
    }

    for ((idx, call), outcome) in
        executable_indices.iter().zip(executable_calls.iter()).zip(executed_outcomes.into_iter())
    {
        runtime_trace::record_event(
            "tool_call_result",
            Some(channel_name),
            Some(provider_name),
            Some(model),
            Some(turn_id),
            Some(outcome.success),
            outcome.error_reason.as_deref(),
            serde_json::json!({
                "iteration": iteration + 1,
                "tool": outcome.tool_name.clone(),
                "duration_ms": outcome.duration.as_millis(),
                "output": scrub_credentials(&outcome.output),
            }),
        );

        if let Some(tx) = on_delta {
            let secs = outcome.duration.as_secs();
            let icon = if outcome.success { "\u{2705}" } else { "\u{274c}" };
            let (_, done_action) = tool_progress_actions(&outcome.tool_name);
            let label = tool_progress_label(&outcome.tool_name, &call.arguments);
            let _ = tx
                .send(format!(
                    "{DRAFT_PROGRESS_SENTINEL}{icon} {} · {} · {secs}s\n",
                    done_action, label
                ))
                .await;

            send_structured_tool_result_progress(tx, &outcome, secs).await;
        }

        ordered_results[*idx] =
            Some((outcome.tool_name.clone(), call.tool_call_id.clone(), outcome));
    }

    // ordered_results 按原始工具调用顺序填充，确保写回历史的顺序与模型请求一致，
    // 即便中间批次可能并行完成。
    let history_entries: Vec<ToolResultHistoryEntry> = ordered_results
        .into_iter()
        .flatten()
        .map(|(tool_name, tool_call_id, outcome)| ToolResultHistoryEntry {
            tool_name,
            tool_call_id,
            output: outcome.output,
        })
        .collect();

    history.push(ChatMessage::assistant(assistant_history_content));
    history.extend(build_tool_result_history_messages(
        native_tool_calls,
        &history_entries,
        use_native_tools,
    ));

    Ok(())
}

async fn send_structured_tool_result_progress(
    tx: &Sender<String>,
    outcome: &ToolExecutionOutcome,
    secs: u64,
) {
    let Some(result_dto) = outcome.result_dto.as_ref() else {
        return;
    };
    let structured = json!({
        "event": "tool_result",
        "name": outcome.tool_name.as_str(),
        "success": outcome.success,
        "duration_secs": secs,
        "result": result_dto,
    });
    let _ =
        tx.send(format!("{DRAFT_PROGRESS_SENTINEL}{DRAFT_WS_EVENT_SENTINEL}{structured}\n")).await;
}

/// 构造无需执行工具的失败结果。
///
/// 用于通道禁用和重复调用等本地拒绝路径；返回值保持与真实执行结果相同的形状，
/// 便于统一写入历史。
fn immediate_failure(
    tool_name: String,
    tool_call_id: Option<String>,
    output: String,
) -> (String, Option<String>, ToolExecutionOutcome) {
    let outcome_tool_name = tool_name.clone();
    (
        tool_name,
        tool_call_id,
        ToolExecutionOutcome {
            tool_name: outcome_tool_name,
            error_reason: Some(output.clone()),
            output,
            success: false,
            result_dto: None,
            duration: Duration::ZERO,
        },
    )
}

fn history_tool_call_id(
    tool_call_id: Option<&str>,
    use_native_tools: bool,
    iteration: usize,
    idx: usize,
) -> Option<String> {
    tool_call_id
        .map(str::to_string)
        .or_else(|| use_native_tools.then(|| format!("fallback_{iteration}_{idx}")))
}
