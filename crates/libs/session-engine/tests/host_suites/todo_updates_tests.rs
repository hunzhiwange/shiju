//! 验证模型步骤驱动的待办更新、用量累计与工具调用过滤。

use crate::app::agent::session::session::Session;
use crate::app::agent::tools::{
    TODO_WRITE_TOOL_ALIAS, TODO_WRITE_TOOL_ID, ToolRuntimeContext, todo,
};
use crate::session::ui_types as models;
use std::collections::HashSet;

fn unique_session_id(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    format!("todo-updates-tests-{name}-{nanos}")
}

fn allowed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn call_update(
    session_id: &str,
    allowed_tools: &HashSet<String>,
    total_usage: &mut models::TokenUsage,
    llm_messages: &mut Vec<serde_json::Value>,
) -> (bool, Session, Vec<super::StreamEvent>) {
    let ctx = ToolRuntimeContext::new(session_id.to_string(), None);
    let mut session = Session::new(session_id.to_string());
    let mut tool_state = super::super::ToolSessionState::default();
    let mut events = Vec::new();

    let updated = super::maybe_update_todos_after_work(
        &mut session,
        &ctx,
        None,
        "base system",
        allowed_tools,
        llm_messages,
        total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &crate::app::agent::config::ReliabilityConfig::default(),
    );

    (updated, session, events)
}

#[test]
fn maybe_update_todos_returns_false_when_todowrite_is_not_allowed() {
    let session_id = unique_session_id("not-allowed");
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    todo::write(
        &serde_json::json!({
            "todos": [
                { "id": "1", "content": "pending", "status": "pending", "priority": "medium" }
            ]
        })
        .to_string(),
        &ctx,
    )
    .expect("incomplete todo should be written");
    let mut usage = models::TokenUsage {
        input_tokens: 1,
        output_tokens: 2,
        cached_tokens: 3,
        reasoning_tokens: 4,
    };
    let original_usage = usage.clone();
    let mut llm_messages = vec![serde_json::json!({ "role": "user", "content": "work done" })];

    let (updated, session, events) =
        call_update(&session_id, &HashSet::new(), &mut usage, &mut llm_messages);

    assert!(!updated);
    assert_eq!(usage, original_usage);
    assert_eq!(llm_messages.len(), 1);
    assert!(session.messages.is_empty());
    assert!(events.is_empty());
}

#[test]
fn maybe_update_todos_returns_false_when_todo_list_is_empty() {
    let session_id = unique_session_id("empty");
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = Vec::new();

    let (updated, session, events) = call_update(
        &session_id,
        &allowed(&[TODO_WRITE_TOOL_ID, TODO_WRITE_TOOL_ALIAS]),
        &mut usage,
        &mut llm_messages,
    );

    assert!(!updated);
    assert_eq!(usage, models::TokenUsage::default());
    assert!(llm_messages.is_empty());
    assert!(session.messages.is_empty());
    assert!(events.is_empty());
}

#[test]
fn maybe_update_todos_returns_false_when_all_todos_are_completed() {
    let session_id = unique_session_id("completed");
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    todo::write(
        &serde_json::json!({
            "todos": [
                { "id": "1", "content": "done", "status": "completed", "priority": "medium" }
            ]
        })
        .to_string(),
        &ctx,
    )
    .expect("completed todo should be written");
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = vec![serde_json::json!({ "role": "assistant", "content": "done" })];

    let (updated, session, events) =
        call_update(&session_id, &allowed(&[TODO_WRITE_TOOL_ID]), &mut usage, &mut llm_messages);

    assert!(!updated);
    assert_eq!(llm_messages.len(), 1);
    assert!(session.messages.is_empty());
    assert!(events.is_empty());
}
