//! 会话工件持久化模块。
//!
//! 本模块集中处理会话处理器在运行过程中产出的持久化载荷，
//! 包括 AI 调用摘要与 LLM 原始步骤数据，避免主循环重复拼装大块 JSON。

use super::llm_runner::LlmStep;
use super::prompting;
use super::types::Request;
use super::utils;
use crate::session::session::{Role, Session};
use crate::session::ui_types as models;
use serde_json::{Value, json};

const RAW_REQUEST_OPTION_ALLOWLIST: [&str; 5] =
    ["reasoning_effort", "temperature", "top_p", "max_tokens", "max_output_tokens"];

fn raw_request_config(req: &Request, resolved_model: &str) -> Value {
    let mut config = serde_json::Map::new();
    config.insert("model".to_string(), Value::String(resolved_model.to_string()));
    for key in RAW_REQUEST_OPTION_ALLOWLIST {
        if let Some(value) = req.options.get(key) {
            config.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(config)
}

fn raw_request_tools(tool_specs: &[crate::app::agent::tools::ToolSpec]) -> Vec<Value> {
    tool_specs
        .iter()
        .map(|tool| {
            json!({
                "id": tool.id,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect()
}

/// UI 会话中的 `calls` 只用于计数和诊断；完整 prompt 已按 step 单独保存在
/// `session_step_llm_raw`，这里不能再次把不断增长的历史完整复制进去。
const COMPACT_CALL_PROMPT_BYTES: usize = 4 * 1024;
const COMPACT_CALL_MESSAGES_BYTES: usize = 4 * 1024;
const COMPACT_CALL_ANSWER_BYTES: usize = 4 * 1024;
const COMPACT_CALL_MESSAGE_BYTES: usize = 2 * 1024;

fn compact_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }

    const MARKER: &str = "\n…<context compacted>…\n";
    let content_budget = max_bytes.saturating_sub(MARKER.len());
    let head_budget = content_budget / 2;
    let tail_budget = content_budget.saturating_sub(head_budget);
    let mut head_end = head_budget.min(value.len());
    while head_end > 0 && !value.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = value.len().saturating_sub(tail_budget);
    while tail_start < value.len() && !value.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    format!("{}{}{}", &value[..head_end], MARKER, &value[tail_start..])
}

fn session_messages_payload(session: &Session) -> Vec<Value> {
    let mut messages = Vec::new();
    let mut used_bytes = 0usize;

    for message in session.messages.iter().rev() {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };
        let value = json!({
            "role": role,
            "content": compact_text(&message.content, COMPACT_CALL_MESSAGE_BYTES),
        });
        let value_bytes = serde_json::to_vec(&value).map_or(0, |serialized| serialized.len());
        if !messages.is_empty()
            && used_bytes.saturating_add(value_bytes) > COMPACT_CALL_MESSAGES_BYTES
        {
            break;
        }
        used_bytes = used_bytes.saturating_add(value_bytes);
        messages.push(value);
    }

    messages.reverse();
    messages
}

/// 持久化单步 AI 调用摘要载荷。
pub(crate) fn persist_step_ai_call_payload(
    req: &Request,
    session: &Session,
    total_usage: &models::TokenUsage,
    step: &LlmStep,
    step_index: u32,
    app_session_scope: Option<&str>,
) {
    if !req.persist_app_session_artifacts || req.session.is_empty() {
        return;
    }

    let usage_now = models::TokenUsage {
        input_tokens: total_usage.input_tokens + step.usage.input_tokens,
        output_tokens: total_usage.output_tokens + step.usage.output_tokens,
        cached_tokens: total_usage.cached_tokens + step.usage.cached_tokens,
        reasoning_tokens: total_usage.reasoning_tokens + step.usage.reasoning_tokens,
    };
    let payload = json!({
        "time": { "created_ms": utils::now_ms() },
        "session_id": req.session,
        "stream_id": req.stream,
        "step_index": step_index,
        "model": req.model,
        "root": req.root,
        "usage": {
            "input_tokens": usage_now.input_tokens,
            "output_tokens": usage_now.output_tokens,
            "cached_tokens": usage_now.cached_tokens,
            "reasoning_tokens": usage_now.reasoning_tokens,
        },
        "prompt": compact_text(
            &prompting::build_prompt(session, req.model.as_deref(), req.root.as_deref(), None),
            COMPACT_CALL_PROMPT_BYTES,
        ),
        "answer": compact_text(&step.text, COMPACT_CALL_ANSWER_BYTES),
        "messages": session_messages_payload(session),
        "context_compacted": true,
    });
    let _ = crate::session::ui_store::persist_ai_call_payload(
        &req.session,
        req.stream,
        &payload,
        app_session_scope,
    );
}

/// 持久化原始 LLM 步骤载荷。
pub(crate) fn persist_llm_raw_step_payload(
    req: &Request,
    session_id: &str,
    step_index: u32,
    llm_messages: &[Value],
    message_sources: &[Value],
    resolved_model: &str,
    system: &[String],
    tool_specs: &[crate::app::agent::tools::ToolSpec],
    step: &LlmStep,
    app_session_scope: Option<&str>,
) -> anyhow::Result<()> {
    if !req.persist_app_session_artifacts || session_id.is_empty() {
        return Ok(());
    }

    let tool_calls = step
        .tool_calls
        .iter()
        .map(|call| {
            json!({
                "id": call.id,
                "type": "function",
                "function": { "name": call.name, "arguments": call.arguments }
            })
        })
        .collect::<Vec<_>>();
    let message_sources = llm_messages
        .iter()
        .enumerate()
        .map(|(index, _)| message_sources.get(index).cloned().unwrap_or(Value::Null))
        .collect::<Vec<_>>();
    let payload = json!({
        "session_id": session_id,
        "step_index": step_index,
        "model": req.model.clone(),
        "system": system,
        "messages": llm_messages,
        "message_sources": message_sources,
        "request": {
            "config": raw_request_config(req, resolved_model),
            "tools": raw_request_tools(tool_specs),
        },
        "output": {
            "text": step.text,
            "reasoning_content": step.reasoning_content,
            "finish_reason": step.finish_reason,
            "tool_calls": tool_calls,
            "usage": step.usage,
        }
    });
    crate::session::ui_store::try_persist_llm_raw_step(
        session_id,
        step_index,
        &payload,
        app_session_scope,
    )?;
    Ok(())
}

/// 持久化最终 AI 调用载荷。
pub(crate) fn persist_final_ai_call_payload(
    req: &Request,
    session: &Session,
    total_usage: &models::TokenUsage,
    answer: &str,
    app_session_scope: Option<&str>,
    should_persist: bool,
) {
    if !should_persist {
        return;
    }

    let created_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let payload = json!({
        "time": { "created_ms": created_ms },
        "session_id": req.session,
        "stream_id": req.stream,
        "model": req.model,
        "root": req.root,
        "usage": {
            "input_tokens": total_usage.input_tokens,
            "output_tokens": total_usage.output_tokens,
            "cached_tokens": total_usage.cached_tokens,
            "reasoning_tokens": total_usage.reasoning_tokens,
        },
        "prompt": compact_text(
            &prompting::build_prompt(session, req.model.as_deref(), req.root.as_deref(), None),
            COMPACT_CALL_PROMPT_BYTES,
        ),
        "answer": compact_text(answer, COMPACT_CALL_ANSWER_BYTES),
        "messages": session_messages_payload(session),
        "context_compacted": true,
    });
    let _ = crate::session::ui_store::persist_ai_call_payload(
        &req.session,
        req.stream,
        &payload,
        app_session_scope,
    );
}
#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod artifacts_tests;
