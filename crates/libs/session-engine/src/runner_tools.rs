//! Session runner implementation; instantiated with the concrete IO host.
use super::*;

#[allow(clippy::too_many_arguments)]
pub fn handle_structured_tool_calls<H: RunnerHost>(
    req: &Request<H>,
    session: &mut Session,
    ctx: &ToolRuntimeContext<H::CtxHost>,
    allowed_tools: &HashSet<String>,
    base_system: &str,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    session_protocol: &mut DesktopSessionProtocol,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    parallel_tools: bool,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    let structured_tool_call_count = step_out.tool_calls.len();
    let ordinary_calls = step_out
        .tool_calls
        .iter()
        .filter(|call| call.kind == model_types::ToolCallKind::Function)
        .cloned()
        .collect::<Vec<_>>();
    if !ordinary_calls.is_empty() {
        llm_messages.push(crate::runner_messages::tool_calls_to_assistant_message(
            &ordinary_calls,
            &step_out.reasoning_content,
        ));
    }
    crate::runner_todos::maybe_mark_todo_in_progress::<H>(session, ctx, tool_state);

    let step_has_todowrite = step_out
        .tool_calls
        .iter()
        .any(|call| tool_types::todo_ids::is_todo_write_tool_id(&call.name));
    let non_todo_runs_before = tool_state.non_todo_tool_runs;
    let tool_context = |tool_call_id: &str| {
        if let Some(message_id) = req.assistant_message_id.as_deref() {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context()
                    .as_ref()
                    .clone()
                    .with_message_id(message_id.to_string())
                    .with_tool_call_id(tool_call_id.to_string()),
            )
        } else {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context().as_ref().clone().with_tool_call_id(tool_call_id.to_string()),
            )
        }
    };

    let mut call_index = 0;
    while call_index < step_out.tool_calls.len() {
        let call = &step_out.tool_calls[call_index];
        if call.kind == model_types::ToolCallKind::Search {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use crate::native_history::NativeHistoryItem;
                if !session_protocol.native_call_ids.insert(call.id.clone()) {
                    on_event(StreamEvent::Error(
                        "native search call_id was already executed".into(),
                    ));
                    call_index += 1;
                    continue;
                }
                let arguments = serde_json::from_str(&call.arguments)
                    .unwrap_or_else(|_| Value::String(call.arguments.clone()));
                let history_call = NativeHistoryItem::ToolSearchCall(aisdk::core::ToolSearchCall {
                    id: None,
                    call_id: Some(call.id.clone()),
                    status: Some("completed".into()),
                    execution: aisdk::core::ToolSearchExecution::Client,
                    arguments,
                    encrypted_content: None,
                });
                llm_messages.push(llm_messages::native_protocol_message(history_call.clone()));
                let result = if is_last_step || !allowed_tools.contains(&call.name) {
                    Err(anyhow::anyhow!("native search is not exposed for this request"))
                } else {
                    crate::tools_exec::run_native_search_and_record::<H::CtxHost>(
                        session,
                        call,
                        &tool_context(&call.id),
                        on_event,
                        tool_state,
                    )
                };
                let output = match result {
                    Ok(output) => output,
                    Err(error) => {
                        on_event(StreamEvent::Error(error.to_string()));
                        NativeHistoryItem::ToolSearchOutput(aisdk::core::ToolSearchOutput {
                            id: None,
                            call_id: Some(call.id.clone()),
                            status: "failed".into(),
                            execution: aisdk::core::ToolSearchExecution::Client,
                            tools: Vec::new(),
                        })
                    }
                };
                crate::tools_exec::record_native_history_pair(session, history_call, output.clone(), on_event);
                llm_messages.push(llm_messages::native_protocol_message(output));
            }
            #[cfg(target_arch = "wasm32")]
            on_event(StreamEvent::Error("native search requires a non-wasm target".into()));
            call_index += 1;
            continue;
        }
        let content = if is_last_step {
            format!(
                "tool unavailable: maximum tool iteration budget reached; \
                 `{}` was not rejected by the permission policy",
                call.name
            )
        } else if !allowed_tools.contains(call.name.as_str()) {
            format!("tool denied: {}", call.name)
        } else if crate::tools_exec::is_agent_tool(&call.name) {
            let group_start = call_index;
            while call_index < step_out.tool_calls.len() {
                let grouped_call = &step_out.tool_calls[call_index];
                if !crate::tools_exec::is_agent_tool(&grouped_call.name)
                    || !allowed_tools.contains(grouped_call.name.as_str())
                {
                    break;
                }
                call_index += 1;
            }
            let grouped_calls = &step_out.tool_calls[group_start..call_index];
            let planned = grouped_calls
                .iter()
                .map(|grouped_call| {
                    crate::tools_exec::AgentToolPlannedCall::<H::CtxHost>::new(
                        &grouped_call.name,
                        &grouped_call.arguments,
                        tool_context(&grouped_call.id),
                    )
                })
                .collect::<Vec<_>>();
            let current_agent = req
                .options
                .get("agent")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("main");
            let max_parallel =
                if parallel_tools { H::parallel_limit(ctx, current_agent) } else { 1 };
            let contents = crate::tools_exec::run_agent_tools_and_record::<H::CtxHost>(
                session,
                &planned,
                max_parallel,
                true,
                on_event,
                tool_state,
            );
            for (grouped_call, content) in grouped_calls.iter().zip(contents) {
                llm_messages.push(crate::runner_messages::tool_result_to_message(
                    &grouped_call.id,
                    &content,
                ));
            }
            continue;
        } else {
            let output = crate::tools_exec::run_tool_and_record_with_protocol::<H::CtxHost>(
                session,
                &call.name,
                &call.arguments,
                &tool_context(&call.id),
                true,
                on_event,
                tool_state,
            );
            if let Some(output) = output {
                session_protocol.consume_tool_output(
                    &call.name,
                    &output.extra_messages,
                    llm_messages,
                );
                output.content_for_model
            } else {
                String::new()
            }
        };
        llm_messages.push(crate::runner_messages::tool_result_to_message(&call.id, &content));
        call_index += 1;
    }

    let non_todo_runs_after = tool_state.non_todo_tool_runs;
    let ran_todo_update = non_todo_runs_after > non_todo_runs_before
        && !step_has_todowrite
        && crate::runner_todo_updates::maybe_update_todos_after_work::<H>(
            session,
            ctx,
            req.model.clone(),
            base_system,
            allowed_tools,
            llm_messages,
            total_usage,
            on_event,
            tool_state,
            &req.reliability,
        );
    tracing::debug!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        tool_call_count = structured_tool_call_count,
        step_has_todowrite,
        non_todo_runs_before,
        non_todo_runs_after,
        ran_todo_update,
        text_preview = %step_text_preview,
        "session processor continuing after structured tool calls"
    );
    ran_todo_update
}

pub fn integrate_agent_tool_children<H: RunnerHost>(
    context: &ToolUseContext<H::CtxHost>,
    messages: &mut Vec<serde_json::Value>,
    tool_state: &ToolSessionState,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
    budget_exhausted: bool,
) -> anyhow::Result<bool> {
    let coordinator = crate::child_agent::coordinator::global_child_agent_coordinator();
    loop {
        if cancellation.is_some_and(|token| token.is_cancelled()) {
            coordinator
                .cancel_descendants(context.session(), &tool_state.agent_background_children)?;
            anyhow::bail!("执行已取消：子任务结果尚未整合");
        }
        if budget_exhausted
            && coordinator
                .descendant_snapshots(context.session(), &[])?
                .iter()
                .any(|task| !task.state.is_terminal())
        {
            anyhow::bail!("Maximum tool iterations reached: child tasks remain unfinished");
        }
        let outcome = coordinator.wait_for_descendants_blocking(
            context.session(),
            &[],
            std::time::Duration::from_millis(100),
        )?;
        if !outcome.timed_out {
            break;
        }
    }
    H::deliver_model_events(context, messages)
}
