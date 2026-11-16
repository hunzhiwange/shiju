//! Session runner implementation; instantiated with the concrete IO host.
use super::*;

#[allow(clippy::too_many_arguments)]
pub fn handle_assistant_text_branch<H: RunnerHost>(
    req: &Request<H>,
    session: &mut Session,
    ctx: &ToolRuntimeContext<H::CtxHost>,
    allowed_tools: &HashSet<String>,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    empty_response_retries: &mut usize,
    step: &mut usize,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    empty_tools: &HashSet<String>,
    is_last_step: bool,
    step_index: u32,
    app_session_scope: Option<&str>,
    stop_hook_active: &mut bool,
) -> bool {
    let mut ran_tool = false;
    let session_message_count_before_tool_parse = session.messages.len();
    let assistant_text = crate::runner_ingest::ingest_assistant_answer::<H>(
        session,
        &step_out.text,
        ctx,
        empty_tools,
        on_event,
        &mut ran_tool,
        tool_state,
    );
    let assistant_text_preview = response_preview(&assistant_text);
    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        ran_inline_tool = ran_tool,
        assistant_text_preview = %assistant_text_preview,
        raw_text_preview = %step_text_preview,
        "session processor evaluated assistant text branch"
    );

    if ran_tool {
        *empty_response_retries = 0;
        llm_messages::extend_llm_messages_from_session_range(
            llm_messages,
            session,
            session_message_count_before_tool_parse,
        );

        let inline_text = assistant_text.trim();
        if !inline_text.is_empty() {
            session.push(Role::Assistant, inline_text.to_string());
            llm_messages.push(crate::runner_messages::assistant_message_with_reasoning(
                inline_text,
                &step_out.reasoning_content,
            ));
        }

        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            assistant_text_preview = %assistant_text_preview,
            "session processor continuing after inline tool execution parsed from assistant text"
        );
        if !on_event(StreamEvent::PostToolRound { step_index }) {
            return false;
        }
        *step += 1;
        return true;
    }

    let text = assistant_text.trim();
    if text.is_empty() {
        return handle_empty_assistant_text::<H>(
            req,
            session,
            llm_messages,
            on_event,
            empty_response_retries,
            step,
            &step_out.reasoning_content,
            is_last_step,
            step_index,
        );
    }

    *empty_response_retries = 0;
    match integrate_agent_tool_children::<H>(
        ctx.tool_use_context().as_ref(),
        llm_messages,
        tool_state,
        req.abort_token.as_ref(),
        is_last_step,
    ) {
        Ok(true) if is_last_step => {
            on_event(StreamEvent::Error(
                "Maximum tool iterations reached: child results remain unintegrated".into(),
            ));
            return false;
        }
        Ok(true) => {
            *step += 1;
            return true;
        }
        Ok(false) => {}
        Err(error) => {
            on_event(StreamEvent::Error(error.to_string()));
            return false;
        }
    }
    session.push(Role::Assistant, text.to_string());
    llm_messages.push(crate::runner_messages::assistant_message_with_reasoning(
        text,
        &step_out.reasoning_content,
    ));

    if is_last_step {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step = true,
            final_text_preview = %response_preview(text),
            "session processor accepted final-step assistant response and completed turn"
        );
        match H::stop_hook(ctx.tool_use_context().clone(), false, *stop_hook_active, Some(text)) {
            Ok(outcome) if !outcome.contexts.is_empty() => {
                on_event(StreamEvent::Error(
                    "Stop hook continuation exceeded maximum tool iterations".into(),
                ));
                return false;
            }
            Ok(_) => {}
            Err(error) => {
                on_event(StreamEvent::Error(error.to_string()));
                return false;
            }
        }
        if req.abort_token.as_ref().is_some_and(|token| token.is_cancelled()) {
            return false;
        }
        crate::artifacts::persist_final_ai_call_payload::<H>(
            &artifact_request::<H>(req),
            session,
            total_usage,
            text,
            app_session_scope,
            true,
        );
        on_event(StreamEvent::Done(total_usage.clone()));
        return false;
    }

    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        final_text_preview = %response_preview(text),
        "session processor accepted assistant response without tool calls"
    );

    // TodoWrite 可用时强制续轮：未完成 todo 不得提前结束，避免模型“口头完成”后意外退出。
    let enforce_todos =
        allowed_tools.iter().any(|name| tool_types::todo_ids::is_todo_write_tool_id(name));
    let incomplete = enforce_todos && crate::runner_todos::has_incomplete_todos(&ctx.session);

    if enforce_todos && incomplete {
        let message = "任务未完成：todo 列表仍有未完成项，请继续推进直至全部完成，再输出最终答复。"
            .to_string();
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            "session processor continuing because enforce_todos found incomplete items"
        );
        session.push(Role::User, message.clone());
        llm_messages.push(json!({ "role": "user", "content": message.clone() }));
        if !on_event(StreamEvent::TodoContinuation) {
            return false;
        }
        *step += 1;
        return true;
    }

    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        final_text_preview = %response_preview(text),
        "session processor completed turn with assistant response"
    );
    match H::stop_hook(ctx.tool_use_context().clone(), false, *stop_hook_active, Some(text)) {
        Ok(outcome) if !outcome.contexts.is_empty() => {
            for message in outcome.contexts {
                session.push(Role::User, message.clone());
                llm_messages.push(json!({"role":"user", "content":message}));
            }
            *stop_hook_active = true;
            *step += 1;
            return true;
        }
        Ok(_) => {}
        Err(error) => {
            on_event(StreamEvent::Error(error.to_string()));
            return false;
        }
    }
    if req.abort_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return false;
    }
    crate::artifacts::persist_final_ai_call_payload::<H>(
        &artifact_request::<H>(req),
        session,
        total_usage,
        text,
        app_session_scope,
        req.persist_app_session_artifacts,
    );
    on_event(StreamEvent::Done(total_usage.clone()));
    false
}

pub fn handle_empty_assistant_text<H: RunnerHost>(
    req: &Request<H>,
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    empty_response_retries: &mut usize,
    step: &mut usize,
    reasoning_content: &str,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    *empty_response_retries = empty_response_retries.saturating_add(1);
    if is_last_step {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step,
            empty_response_retries,
            "session processor saw empty final-step assistant text"
        );
    } else {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step,
            empty_response_retries,
            "session processor saw empty non-final assistant text"
        );
    }

    if *empty_response_retries > EMPTY_RESPONSE_RETRY_LIMIT {
        let final_message = format!(
            "{}，自动重试 {} 次后仍为空，任务终止",
            EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT
        );
        session.push(Role::System, final_message.clone());
        on_event(StreamEvent::Error(final_message));
        return false;
    }

    session.push(
        Role::System,
        format!(
            "{}，自动重试中（{}/{}）",
            EMPTY_RESPONSE_ERR, empty_response_retries, EMPTY_RESPONSE_RETRY_LIMIT
        ),
    );
    llm_messages
        .push(crate::runner_messages::assistant_message_with_reasoning("", reasoning_content));
    llm_messages.push(json!({
        "role": "system",
        "content": EMPTY_RESPONSE_RETRY_PROMPT
    }));
    *step += 1;
    true
}
