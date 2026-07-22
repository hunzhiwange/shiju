//! 会话处理主循环模块。
//!
//! 本模块承接原先位于入口文件中的完整执行编排逻辑，
//! 负责初始化上下文、驱动 LLM 循环、处理工具调用并在结束时发出终态事件。

use super::artifacts;
use super::helpers::{
    allowed_tool_ids_for_request_from_specs, is_acp_request, response_preview, tool_call_preview,
};
use super::llm_messages;
use super::llm_runner;
use super::prefetch;
use super::todo_updates;
use super::todos;
use super::tools_exec;
use super::types::{Request, StreamEvent, ToolSessionState};
use super::utils;
use crate::app::agent::tools::{ToolRuntimeContext, ToolUseContext};
use crate::session::prompt;
use crate::session::session::{Message, Role, Session};
use crate::session::ui_types as models;
use serde_json::{Value, json};
use std::collections::HashSet;

const EMPTY_RESPONSE_ERR: &str = "模型未返回内容";
const EMPTY_RESPONSE_RETRY_PROMPT: &str =
    "上一步模型输出为空，已自动重试。请继续当前任务并返回非空答复。";
const EMPTY_RESPONSE_RETRY_LIMIT: usize = 4;

fn request_full_access_enabled(options: &Value) -> bool {
    options.get("full_access").and_then(Value::as_bool).unwrap_or(false)
}

/// 执行会话处理主循环。
///
/// 本函数保留原有入口签名，对外仍通过 `processor::run` 暴露；
/// 这里仅将实现迁移至独立文件，方便后续按职责维护。
pub fn run(req: Request, mut on_event: impl FnMut(StreamEvent) -> bool + Send + 'static) {
    let mut session = Session::new(req.session.clone());
    let is_acp = is_acp_request(&req.options);
    let app_session_scope = if req.persist_app_session_artifacts {
        prefetch::app_session_scope_from_root(req.root.as_deref())
    } else {
        None
    };

    session.messages = req
        .history
        .iter()
        .map(|message| Message {
            role: match message.role {
                models::ChatRole::User => Role::User,
                models::ChatRole::Assistant => Role::Assistant,
                models::ChatRole::System => Role::System,
                models::ChatRole::Tool => Role::Tool,
            },
            content: message.content.clone(),
        })
        .collect();

    let mut tool_use_context = ToolUseContext::new(req.session.clone(), req.root.clone());
    if let Some(channel_name) = req.channel_name.clone() {
        tool_use_context = tool_use_context.with_channel(channel_name);
    }
    if let Some(approval) = req.approval.clone() {
        tool_use_context = tool_use_context.with_approval(approval);
    }
    if let Some(non_cli_approval_context) = req.non_cli_approval_context.clone() {
        tool_use_context = tool_use_context.with_non_cli_approval_context(non_cli_approval_context);
    }
    if let Some(message_id) = req.assistant_message_id.clone() {
        tool_use_context = tool_use_context.with_message_id(message_id);
    }
    let full_access_enabled = request_full_access_enabled(&req.options);
    tool_use_context = tool_use_context.with_full_access_enabled(full_access_enabled);
    if full_access_enabled {
        tool_use_context = tool_use_context.with_bypass_non_cli_approval_for_turn(true);
    }
    let ctx = ToolRuntimeContext::new(req.session.clone(), req.root.clone())
        .with_tool_use_context(tool_use_context);
    let mut tool_state = ToolSessionState::default();
    let available_tool_specs = crate::app::agent::tools::tool_specs_for_context(&ctx);
    let allowed_tools =
        allowed_tool_ids_for_request_from_specs(&available_tool_specs, &req.options);

    if utils::is_docs_request(&req.query) {
        handle_docs_request(&mut session, &ctx, &mut on_event);
        return;
    }

    prefetch::prefetch_seed_context(
        &mut session,
        &req.query,
        &ctx,
        &allowed_tools,
        &mut tool_state,
    );
    utils::ingest_user_query(
        &mut session,
        &req.query,
        &ctx,
        &allowed_tools,
        &mut on_event,
        &mut tool_state,
    );

    let base_system = prompt::system(req.model.as_deref(), req.root.as_deref());
    let primary_system_prompt = req
        .options
        .get("chat_system_prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut llm_messages = llm_messages::session_messages_to_llm_messages(&session);

    let mut step = 0;
    let mut step_index: u32 = 0;
    let mut total_usage = models::TokenUsage::default();
    let mut tried_auto_complete_todos = false;
    let mut empty_response_retries = 0usize;

    loop {
        let is_last_step = step >= 100;
        let empty_tools = HashSet::<String>::new();
        let mut system = vec![base_system.clone()];
        if let Some(prompt) = primary_system_prompt.as_ref() {
            system.push(prompt.clone());
        }
        if is_last_step {
            system.push(prompt::max_steps_text().to_string());
        }
        let tools_for_step = if is_last_step { &empty_tools } else { &allowed_tools };

        let retry_budget = if is_acp {
            2
        } else if todos::has_incomplete_todos(&ctx) {
            5
        } else {
            3
        };
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index = step_index.saturating_add(1),
            retry_budget,
            is_acp,
            "session processor selected retry budget"
        );
        step_index = step_index.saturating_add(1);

        if !on_event(StreamEvent::StepStart {
            step_index,
            created_ms: utils::now_ms(),
            model: req.model.clone(),
        }) {
            return;
        }

        let step_out = match llm_runner::run_llm_step_with_retry(
            &req.session,
            &llm_messages,
            &system,
            req.model.clone(),
            &req.options,
            &available_tool_specs,
            tools_for_step,
            &mut on_event,
            retry_budget,
        ) {
            Ok(output) => output,
            Err(message) => {
                if handle_step_error(
                    &mut session,
                    &mut llm_messages,
                    &mut empty_response_retries,
                    &mut step,
                    &mut on_event,
                    message,
                ) {
                    continue;
                }
                return;
            }
        };

        artifacts::persist_step_ai_call_payload(
            &req,
            &session,
            &total_usage,
            &step_out,
            step_index,
            app_session_scope.as_deref(),
        );

        if !on_event(StreamEvent::StepFinish {
            step_index,
            finished_ms: utils::now_ms(),
            usage: step_out.usage.clone(),
            finish_reason: step_out.finish_reason.clone(),
            model: req.model.clone(),
        }) {
            return;
        }

        artifacts::persist_llm_raw_step_payload(
            &req,
            &ctx.session,
            step_index,
            &llm_messages,
            &step_out,
            app_session_scope.as_deref(),
        );

        total_usage.input_tokens += step_out.usage.input_tokens;
        total_usage.output_tokens += step_out.usage.output_tokens;
        total_usage.cached_tokens += step_out.usage.cached_tokens;
        total_usage.reasoning_tokens += step_out.usage.reasoning_tokens;

        let step_text_preview = response_preview(&step_out.text);
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_acp,
            is_last_step,
            tool_call_count = step_out.tool_calls.len(),
            tool_calls = %step_out
                .tool_calls
                .iter()
                .map(|call| tool_call_preview(&call.name, &call.arguments))
                .collect::<Vec<_>>()
                .join(", "),
            finish_reason = step_out.finish_reason.as_deref().unwrap_or(""),
            text_preview = %step_text_preview,
            "session processor received llm step result"
        );

        if !step_out.tool_calls.is_empty() {
            if super::should_execute_structured_tool_calls_locally(&req.options) {
                empty_response_retries = 0;
                let ran_todo_update = handle_structured_tool_calls(
                    &req,
                    &mut session,
                    &ctx,
                    &allowed_tools,
                    &base_system,
                    &mut llm_messages,
                    &mut total_usage,
                    &mut on_event,
                    &mut tool_state,
                    &step_out,
                    &step_text_preview,
                    is_last_step,
                    step_index,
                );
                if !on_event(StreamEvent::PostToolRound { step_index }) {
                    return;
                }
                step += 1;
                if ran_todo_update {
                    step += 1;
                }
                continue;
            }

            tracing::warn!(
                target: "vw_agent",
                session_id = %req.session,
                step_index,
                tool_call_count = step_out.tool_calls.len(),
                "session processor ignored structured tool calls emitted by ACP request"
            );
        }

        if handle_assistant_text_branch(
            &req,
            &mut session,
            &ctx,
            &mut llm_messages,
            &mut total_usage,
            &mut on_event,
            &mut tool_state,
            &mut empty_response_retries,
            &mut step,
            &mut tried_auto_complete_todos,
            &step_out,
            &step_text_preview,
            &empty_tools,
            is_last_step,
            step_index,
            app_session_scope.as_deref(),
        ) {
            continue;
        }
        return;
    }
}

fn handle_docs_request(
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) {
    match utils::list_docs(ctx.root.as_ref()) {
        Ok(files) => {
            let mut out = String::new();
            out.push_str(
                "<think>\n思考中\n任务分解：\n- 读取 docs 目录\n- 整理文件列表\n</think>\n\n",
            );
            out.push_str("docs 目录下的文件：\n");
            for path in files {
                out.push_str("- ");
                out.push_str(&path);
                out.push('\n');
            }
            session.push(Role::Assistant, out.clone());
            if !on_event(StreamEvent::Delta(out)) {
                return;
            }
            on_event(StreamEvent::Done(models::TokenUsage::default()));
        }
        Err(message) => {
            session.push(Role::System, message.clone());
            on_event(StreamEvent::Error(message));
        }
    }
}

fn handle_step_error(
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    empty_response_retries: &mut usize,
    step: &mut i32,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    message: String,
) -> bool {
    if message.trim() != EMPTY_RESPONSE_ERR {
        session.push(Role::System, message.clone());
        on_event(StreamEvent::Error(message));
        return false;
    }

    *empty_response_retries = empty_response_retries.saturating_add(1);
    if *empty_response_retries <= EMPTY_RESPONSE_RETRY_LIMIT {
        session.push(
            Role::System,
            format!(
                "{}，自动重试中（{}/{}）",
                EMPTY_RESPONSE_ERR, empty_response_retries, EMPTY_RESPONSE_RETRY_LIMIT
            ),
        );
        llm_messages.push(json!({
            "role": "system",
            "content": EMPTY_RESPONSE_RETRY_PROMPT
        }));
        *step += 1;
        return true;
    }

    let final_message = format!(
        "{}，自动重试 {} 次后仍为空，任务终止",
        EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT
    );
    session.push(Role::System, final_message.clone());
    on_event(StreamEvent::Error(final_message));
    false
}

#[allow(clippy::too_many_arguments)]
fn handle_structured_tool_calls(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    base_system: &str,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    let structured_tool_call_count = step_out.tool_calls.len();
    llm_messages.push(utils::tool_calls_to_assistant_message(
        &step_out.tool_calls,
        &step_out.reasoning_content,
    ));
    todos::maybe_mark_todo_in_progress(session, ctx, tool_state);

    let step_has_todowrite = step_out
        .tool_calls
        .iter()
        .any(|call| crate::app::agent::tools::is_todo_write_tool_id(&call.name));
    let non_todo_runs_before = tool_state.non_todo_tool_runs;
    for call in &step_out.tool_calls {
        let tool_ctx = if let Some(message_id) = req.assistant_message_id.as_deref() {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context()
                    .as_ref()
                    .clone()
                    .with_message_id(message_id.to_string())
                    .with_tool_call_id(call.id.clone()),
            )
        } else {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context().as_ref().clone().with_tool_call_id(call.id.clone()),
            )
        };
        let content = if is_last_step || !allowed_tools.contains(call.name.as_str()) {
            format!("tool denied: {}", call.name)
        } else {
            tools_exec::run_tool_and_record(
                session,
                &call.name,
                &call.arguments,
                &tool_ctx,
                true,
                on_event,
                tool_state,
            )
            .unwrap_or_default()
        };
        llm_messages.push(utils::tool_result_to_message(&call.id, &content));
    }

    let non_todo_runs_after = tool_state.non_todo_tool_runs;
    let ran_todo_update = non_todo_runs_after > non_todo_runs_before
        && !step_has_todowrite
        && todo_updates::maybe_update_todos_after_work(
            session,
            ctx,
            req.model.clone(),
            base_system,
            allowed_tools,
            llm_messages,
            total_usage,
            on_event,
            tool_state,
        );
    tracing::info!(
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

#[allow(clippy::too_many_arguments)]
fn handle_assistant_text_branch(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    empty_response_retries: &mut usize,
    step: &mut i32,
    tried_auto_complete_todos: &mut bool,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    empty_tools: &HashSet<String>,
    is_last_step: bool,
    step_index: u32,
    app_session_scope: Option<&str>,
) -> bool {
    let mut ran_tool = false;
    let session_message_count_before_tool_parse = session.messages.len();
    let assistant_text = utils::ingest_assistant_answer(
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
            llm_messages.push(utils::assistant_message_with_reasoning(
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
        return handle_empty_assistant_text(
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
    session.push(Role::Assistant, text.to_string());
    llm_messages.push(utils::assistant_message_with_reasoning(text, &step_out.reasoning_content));

    if is_last_step {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step = true,
            final_text_preview = %response_preview(text),
            "session processor accepted final-step assistant response and completed turn"
        );
        artifacts::persist_final_ai_call_payload(
            req,
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

    let enforce_todos = false;
    let mut incomplete = enforce_todos && todos::has_incomplete_todos(ctx);
    if enforce_todos
        && incomplete
        && !*tried_auto_complete_todos
        && utils::should_try_auto_complete_todos(text, tool_state)
        && todos::maybe_mark_all_todos_completed(session, ctx, on_event, tool_state)
    {
        *tried_auto_complete_todos = true;
        incomplete = enforce_todos && todos::has_incomplete_todos(ctx);
    }

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
        if !on_event(StreamEvent::Delta(format!("\n\n{}\n\n", message))) {
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
    artifacts::persist_final_ai_call_payload(
        req,
        session,
        total_usage,
        text,
        app_session_scope,
        req.persist_app_session_artifacts,
    );
    on_event(StreamEvent::Done(total_usage.clone()));
    false
}

fn handle_empty_assistant_text(
    req: &Request,
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    empty_response_retries: &mut usize,
    step: &mut i32,
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
    llm_messages.push(utils::assistant_message_with_reasoning("", reasoning_content));
    llm_messages.push(json!({
        "role": "system",
        "content": EMPTY_RESPONSE_RETRY_PROMPT
    }));
    *step += 1;
    true
}
#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;
