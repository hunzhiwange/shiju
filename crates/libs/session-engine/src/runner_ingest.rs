//! 会话消息写入辅助逻辑，负责把用户输入、助手回答和工具调用事件合并进会话历史。

use crate::processor_events::StreamEvent;
use crate::runner_host::RunnerHost;
use crate::session_memory::{Role, Session};
use crate::tool_parsing::parse_tool_at;
use std::collections::HashSet;
use tool_assembly::runtime::ToolRuntimeContext;

/// 执行 push_user_dedup 操作，并返回调用方需要的结果。
pub fn push_user_dedup(session: &mut Session, content: String) {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return;
    }
    if session
        .messages
        .last()
        .is_some_and(|m| matches!(m.role, Role::User) && m.content.trim() == trimmed)
    {
        return;
    }
    session.push(Role::User, content);
}

/// 执行 ingest_assistant_answer 操作，并返回调用方需要的结果。
pub fn ingest_assistant_answer<H: RunnerHost>(
    session: &mut Session,
    answer: &str,
    ctx: &ToolRuntimeContext<H::CtxHost>,
    allowed_tools: &HashSet<String>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    ran_tool: &mut bool,
    tool_state: &mut crate::runner_types::ToolSessionState,
) -> String {
    let lines: Vec<&str> = answer.lines().collect();
    let mut i = 0usize;
    let mut assistant_text = String::new();
    while i < lines.len() {
        if let Some((name, input, consumed)) = parse_tool_at(&lines, i, allowed_tools) {
            *ran_tool = true;
            crate::runner_todos::maybe_mark_todo_in_progress::<H>(session, ctx, tool_state);
            let call = if input.trim().is_empty() {
                format!("/{}", name)
            } else {
                format!("/{} {}", name, input.trim())
            };
            session.push(Role::Assistant, call);
            let _ = crate::tools_exec::run_tool_and_record::<H::CtxHost>(session, &name, &input, ctx, true, on_event, tool_state);
            i += consumed;
            continue;
        }

        let line = lines[i].trim();
        if !line.is_empty() {
            if !assistant_text.is_empty() {
                assistant_text.push('\n');
            }
            assistant_text.push_str(line);
        }
        i += 1;
    }
    assistant_text
}

/// 执行 ingest_user_query 操作，并返回调用方需要的结果。
pub fn ingest_user_query<H: RunnerHost>(
    session: &mut Session,
    query: &str,
    ctx: &ToolRuntimeContext<H::CtxHost>,
    allowed_tools: &HashSet<String>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut crate::runner_types::ToolSessionState,
) {
    let lines: Vec<&str> = query.lines().collect();
    let mut i = 0usize;
    let mut buf = String::new();
    while i < lines.len() {
        if let Some((name, input, consumed)) = parse_tool_at(&lines, i, allowed_tools) {
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                push_user_dedup(session, trimmed.to_string());
            }
            buf.clear();
            let _ = crate::tools_exec::run_tool_and_record::<H::CtxHost>(session, &name, &input, ctx, true, on_event, tool_state);
            i += consumed;
            continue;
        }

        let line = lines[i].trim();
        if !line.is_empty() {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
        }
        i += 1;
    }

    let trimmed = buf.trim();
    if !trimmed.is_empty() {
        let content = trimmed.to_string();
        if let Some(last) = session.messages.last_mut()
            && matches!(last.role, Role::User)
            && last.content.trim() == trimmed
        {
            last.content = content;
            return;
        }
        push_user_dedup(session, content);
    }
}

#[cfg(test)]
#[path = "session_ingest_domain_tests.rs"]
mod session_ingest_domain_tests;
