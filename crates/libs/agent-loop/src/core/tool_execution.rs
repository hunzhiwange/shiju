//! 工具调用调度、去重、权限拒绝与历史写回规则。
use std::{sync::Arc, collections::HashSet, time::Duration};
use anyhow::Result;
use model_types::{ChatMessage, ToolCall};
use agent_observability::{Observer, runtime_trace};
use tool_types::Tool;
use tool_context::{context::ToolUseContext, host::ContextHost, ToolResultHistoryEntry};
use tool_context::executor::build_tool_result_history_messages;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use serde_json::{Value, json};
use super::tool_loop::LoopFuture;
use vw_api_tools::ToolResultDto;

/// 借用已有解析/调度类型，避免在迁移时复制AST或结果所有者。
pub struct CallView<'a> { pub name: &'a str, pub arguments: &'a Value, pub tool_call_id: Option<&'a str> }
pub struct OutcomeView<'a> {
    pub tool_name: &'a str, pub output: &'a str, pub success: bool,
    pub error_reason: Option<&'a str>, pub result_dto: Option<&'a ToolResultDto>, pub duration: Duration,
}
pub struct ToolBatch<P> { pub parallel: bool, pub calls: Vec<P> }
pub struct ToolExecutionServices<C: ContextHost,T,P,O,N> {
    pub call_view: for<'a> fn(&'a T) -> CallView<'a>,
    pub pending_view: for<'a> fn(&'a P) -> CallView<'a>,
    pub pending_call: fn(String, Value, Option<String>) -> P,
    pub schedule: fn(&[P], &[Box<dyn Tool>]) -> Vec<ToolBatch<P>>,
    pub outcome_view: for<'a> fn(&'a O) -> OutcomeView<'a>,
    pub take_output: fn(O) -> String,
    pub failure: fn(String, String) -> O,
    pub execute_parallel: for<'a> fn(&'a [P], &'a [Box<dyn Tool>], &'a dyn Observer, Arc<ToolUseContext<C>>, Option<&'a CancellationToken>) -> LoopFuture<'a, Result<Vec<O>>>,
    pub execute_sequential: for<'a> fn(&'a [P], &'a [Box<dyn Tool>], &'a dyn Observer, Arc<ToolUseContext<C>>, Option<&'a CancellationToken>) -> LoopFuture<'a, Result<Vec<O>>>,
    pub inject_cron_delivery: fn(&str, &mut Value, &str, Option<&str>),
    pub call_signature: fn(&str, &Value) -> (String,String),
    pub scrub_credentials: fn(&str) -> String,
    pub progress_actions: fn(&str) -> (&'static str, &'static str),
    pub progress_label: fn(&str, &Value) -> String,
    pub progress_sentinel: &'static str,
    pub serialize_native: fn(&N) -> Result<String>,
    pub ws_event_sentinel: &'static str,
    #[cfg(not(target_arch = "wasm32"))]
    pub native_search_history: fn(&C::ToolDiscoverySession, &ToolCall) -> Result<(N,N)>,
}
pub fn immediate_failure<C: ContextHost,T,P,O,N>(services: &ToolExecutionServices<C,T,P,O,N>, name: String, id: Option<String>, output: String) -> (String, Option<String>, O) {
    let outcome = (services.failure)(name.clone(), output);
    (name,id,outcome)
}
pub async fn execute_tool_calls_and_update_history<C: ContextHost, T, P, O, N>(
    services: &ToolExecutionServices<C,T,P,O,N>,
    history: &mut Vec<ChatMessage>,
    tool_calls: &[T],
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
    tool_use_context: &Arc<ToolUseContext<C>>,
    on_delta: Option<&Sender<String>>,
    excluded_tools: &[String],
    _bypass_non_cli_approval_for_turn: bool,
    cancellation_token: Option<&CancellationToken>,
    seen_tool_signatures: &mut HashSet<(String, String)>,
    use_native_tools: bool,
) -> Result<()> {
    let catalog_registry = tool_use_context.catalog_registry();
    let tools_registry = catalog_registry.as_deref().unwrap_or(tools_registry);
    // 每轮工具执行都携带当前历史快照，工具可以基于一致视图读取上下文，而不会
    // 观察到本轮尚未写回的部分结果。
    let iteration_tool_use_context = Arc::new(
        tool_use_context
            .as_ref()
            .clone()
            .with_iteration(iteration)
            .with_messages_view(history.clone()),
    );
    #[cfg(target_arch = "wasm32")]
    anyhow::ensure!(
        native_tool_calls.iter().all(|call| call.kind == model_types::ToolCallKind::Function),
        "native tool search requires a non-wasm target"
    );
    #[cfg(not(target_arch = "wasm32"))]
    let (mut native_outputs, native_calls) = {
        // Validate and prepare dedicated output before the executor can load any hit.
        let mut native_outputs = std::collections::HashMap::new();
        let mut native_calls = Vec::new();
        for call in native_tool_calls
            .iter()
            .filter(|call| call.kind == model_types::ToolCallKind::Search)
        {
            anyhow::ensure!(
                !excluded_tools.contains(&call.name),
                "native search is excluded in this channel"
            );
            let discovery = tool_use_context
                .tool_discovery()
                .ok_or_else(|| anyhow::anyhow!("native search requires runtime discovery"))?;
            let (output, history_call) = (services.native_search_history)(&discovery, call)?;
            native_outputs.insert(call.id.clone(), output);
            native_calls.push(history_call);
        }
        (native_outputs, native_calls)
    };
    let mut ordered_results: Vec<Option<(String, Option<String>, O)>> =
        (0..tool_calls.len()).map(|_| None).collect();
    let mut executable_indices = Vec::new();
    let mut executable_calls: Vec<P> = Vec::new();

    for (idx, call) in tool_calls.iter().enumerate() {
        let call = (services.call_view)(call);
        let tool_name = call.name.to_string();
        let mut tool_args = call.arguments.clone();

        (services.inject_cron_delivery)(
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
                    "arguments": (services.scrub_credentials)(&tool_args.to_string()),
                    "blocked_by_channel_policy": true,
                }),
            );
            ordered_results[idx] = Some(immediate_failure(services,
                tool_name.clone(),
                history_tool_call_id(
                    call.tool_call_id,
                    use_native_tools,
                    iteration,
                    idx,
                ),
                blocked,
            ));
            continue;
        }

        let mut signature = (services.call_signature)(&tool_name, &tool_args);
        if let Some(tool_call_id) =
            call.tool_call_id.filter(|id| !id.starts_with("fallback_"))
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
                    "arguments": (services.scrub_credentials)(&tool_args.to_string()),
                    "deduplicated": true,
                }),
            );
            ordered_results[idx] = Some(immediate_failure(services,
                tool_name,
                history_tool_call_id(
                    call.tool_call_id,
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
                "arguments": (services.scrub_credentials)(&tool_args.to_string()),
            }),
        );

        if let Some(tx) = on_delta {
            let (start_action, _) = (services.progress_actions)(&tool_name);
            let label = (services.progress_label)(&tool_name, &tool_args);
            let progress = format!("⏳ {} · {}\n", start_action, label);
            let _ = tx.send(format!("{}{progress}", services.progress_sentinel)).await;
        }

        executable_indices.push(idx);
        executable_calls.push((services.pending_call)(tool_name, tool_args, call.tool_call_id.map(str::to_string)));
    }

    let mut executed_outcomes = Vec::with_capacity(executable_calls.len());
    for batch in (services.schedule)(&executable_calls, tools_registry) {
        // 调度器根据工具声明决定并行或顺序执行，既保留可并发工具的效率，也避免
        // 对有副作用或顺序要求的工具打乱执行次序。
        let mut batch_outcomes = match batch.parallel {
            true => {
                (services.execute_parallel)(
                    &batch.calls,
                    tools_registry,
                    observer,
                    iteration_tool_use_context.clone(),
                    cancellation_token,
                )
                .await?
            }
            false => {
                (services.execute_sequential)(
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
        let call = (services.pending_view)(call);
        let view = (services.outcome_view)(&outcome);
        runtime_trace::record_event(
            "tool_call_result",
            Some(channel_name),
            Some(provider_name),
            Some(model),
            Some(turn_id),
            Some(view.success),
            view.error_reason,
            serde_json::json!({
                "iteration": iteration + 1,
                "tool": view.tool_name.to_string(),
                "duration_ms": view.duration.as_millis(),
                "output": (services.scrub_credentials)(&view.output),
            }),
        );

        if let Some(tx) = on_delta {
            let secs = view.duration.as_secs();
            let icon = if view.success { "\u{2705}" } else { "\u{274c}" };
            let (_, done_action) = (services.progress_actions)(&view.tool_name);
            let label = (services.progress_label)(&view.tool_name, &call.arguments);
            let _ = tx
                .send(format!(
                    "{}{icon} {} · {} · {secs}s\n",
                    services.progress_sentinel, done_action, label
                ))
                .await;

            send_structured_tool_result_progress(tx, &view, secs, services).await;
        }

        ordered_results[*idx] =
            Some((view.tool_name.to_string(), call.tool_call_id.map(str::to_string), outcome));
    }

    #[cfg(not(target_arch = "wasm32"))]
    for (_, id, outcome) in ordered_results.iter().flatten() {
        let view = (services.outcome_view)(outcome);
        if id.as_ref().is_some_and(|id| native_outputs.contains_key(id)) {
            anyhow::ensure!(
                view.success,
                "native tool search execution failed: {}",
                view.output
            );
        }
    }

    // ordered_results 按原始工具调用顺序填充，确保写回历史的顺序与模型请求一致，
    // 即便中间批次可能并行完成。
    let history_entries: Vec<ToolResultHistoryEntry> = ordered_results
        .into_iter()
        .flatten()
        .map(|(tool_name, tool_call_id, outcome)| ToolResultHistoryEntry {
            tool_name,
            tool_call_id,
            output: (services.take_output)(outcome),
        })
        .collect();

    #[cfg(not(target_arch = "wasm32"))]
    if !native_calls.is_empty() {
        let ordinary_calls: Vec<_> = native_tool_calls
            .iter()
            .filter(|call| call.kind == model_types::ToolCallKind::Function)
            .cloned()
            .collect();
        let mut assistant: serde_json::Value = serde_json::from_str(&assistant_history_content)?;
        if let Some(calls) = assistant["tool_calls"].as_array_mut() {
            calls.retain(|call| {
                !native_outputs.contains_key(call["id"].as_str().unwrap_or_default())
            });
        }
        if !ordinary_calls.is_empty()
            || assistant["content"].as_str().is_some_and(|text| !text.is_empty())
        {
            history.push(ChatMessage::assistant(assistant.to_string()));
        }
        for call in native_calls {
            history.push(ChatMessage {
                role: "native".into(),
                content: (services.serialize_native)(&call)?,
            });
        }
        for entry in history_entries {
            if let Some(output) =
                entry.tool_call_id.as_ref().and_then(|id| native_outputs.remove(id))
            {
                history.push(ChatMessage {
                    role: "native".into(),
                    content: (services.serialize_native)(&output)?,
                });
            } else {
                let calls: Vec<_> = ordinary_calls
                    .iter()
                    .filter(|call| Some(&call.id) == entry.tool_call_id.as_ref())
                    .cloned()
                    .collect();
                history.extend(build_tool_result_history_messages(
                    &calls,
                    &[entry],
                    use_native_tools,
                ));
            }
        }
        return Ok(());
    }
    history.push(ChatMessage::assistant(assistant_history_content));
    history.extend(build_tool_result_history_messages(
        native_tool_calls,
        &history_entries,
        use_native_tools,
    ));

    Ok(())
}

pub async fn send_structured_tool_result_progress<C: ContextHost,T,P,O,N>(
    tx: &Sender<String>,
    outcome: &OutcomeView<'_>,
    secs: u64,
    services: &ToolExecutionServices<C,T,P,O,N>,
) {
    let Some(result_dto) = outcome.result_dto else {
        return;
    };
    let structured = json!({
        "event": "tool_result",
        "name": outcome.tool_name,
        "success": outcome.success,
        "duration_secs": secs,
        "result": result_dto,
    });
    let _ =
        tx.send(format!("{}{}{structured}\n", services.progress_sentinel, services.ws_event_sentinel)).await;
}

pub fn history_tool_call_id(
    tool_call_id: Option<&str>,
    use_native_tools: bool,
    iteration: usize,
    idx: usize,
) -> Option<String> {
    tool_call_id
        .map(str::to_string)
        .or_else(|| use_native_tools.then(|| format!("fallback_{iteration}_{idx}")))
}
