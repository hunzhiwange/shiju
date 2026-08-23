//! 验证模型步骤驱动的待办更新、用量累计与工具调用过滤。

use crate::app::agent::session::llm;
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

fn llm_step(
    usage: models::TokenUsage,
    reasoning_content: &str,
    tool_calls: Vec<llm::ToolCall>,
) -> super::super::llm_runner::LlmStep {
    super::super::llm_runner::LlmStep {
        usage,
        finish_reason: Some("tool_calls".to_string()),
        reasoning_content: reasoning_content.to_string(),
        text: String::new(),
        tool_calls,
        full_messages: Vec::new(),
    }
}

fn tool_call(id: &str, name: &str, arguments: &str) -> llm::ToolCall {
    llm::ToolCall { id: id.to_string(), name: name.to_string(), arguments: arguments.to_string() }
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

#[test]
fn apply_todo_update_step_accumulates_usage_and_stops_without_tool_calls() {
    let mut usage = models::TokenUsage {
        input_tokens: 10,
        output_tokens: 20,
        cached_tokens: 30,
        reasoning_tokens: 40,
    };
    let mut llm_messages = vec![serde_json::json!({ "role": "user", "content": "done" })];
    let step = llm_step(
        models::TokenUsage {
            input_tokens: 1,
            output_tokens: 2,
            cached_tokens: 3,
            reasoning_tokens: 4,
        },
        "unused reasoning",
        Vec::new(),
    );

    let updated = super::apply_todo_update_step(&mut llm_messages, &mut usage, step, |_| {
        panic!("empty tool calls must not execute a tool")
    });

    assert!(!updated);
    assert_eq!(
        usage,
        models::TokenUsage {
            input_tokens: 11,
            output_tokens: 22,
            cached_tokens: 33,
            reasoning_tokens: 44,
        }
    );
    assert_eq!(llm_messages.len(), 1);
}

#[test]
fn apply_todo_update_step_records_assistant_message_and_skips_non_todowrite_calls() {
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = Vec::new();
    let step = llm_step(
        models::TokenUsage::default(),
        "checked progress",
        vec![tool_call("call-read", "read", r#"{"path":"notes.md"}"#)],
    );

    let updated = super::apply_todo_update_step(&mut llm_messages, &mut usage, step, |_| {
        panic!("non-todowrite calls must be filtered")
    });

    assert!(!updated);
    assert_eq!(llm_messages.len(), 1);
    assert_eq!(llm_messages[0]["role"], "assistant");
    assert_eq!(llm_messages[0]["reasoning_content"], "checked progress");
    assert_eq!(llm_messages[0]["tool_calls"][0]["function"]["name"], "read");
}

#[test]
fn apply_todo_update_step_executes_only_todowrite_alias_and_records_result() {
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = Vec::new();
    let step = llm_step(
        models::TokenUsage {
            input_tokens: 5,
            output_tokens: 6,
            cached_tokens: 7,
            reasoning_tokens: 8,
        },
        "update the completed item",
        vec![
            tool_call("call-read", "read", r#"{"path":"notes.md"}"#),
            tool_call(
                "call-todo",
                TODO_WRITE_TOOL_ALIAS,
                r#"{"todos":[{"id":"1","status":"completed"}],"merge":true}"#,
            ),
        ],
    );
    let mut executed = Vec::new();

    let updated = super::apply_todo_update_step(&mut llm_messages, &mut usage, step, |call| {
        executed.push((call.name.clone(), call.arguments.clone()));
        "todo updated".to_string()
    });

    assert!(updated);
    assert_eq!(executed.len(), 1);
    assert_eq!(executed[0].0, TODO_WRITE_TOOL_ALIAS);
    assert!(executed[0].1.contains("completed"));
    assert_eq!(usage.input_tokens, 5);
    assert_eq!(usage.output_tokens, 6);
    assert_eq!(usage.cached_tokens, 7);
    assert_eq!(usage.reasoning_tokens, 8);
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(llm_messages[1]["role"], "tool");
    assert_eq!(llm_messages[1]["tool_call_id"], "call-todo");
    assert_eq!(llm_messages[1]["content"], "todo updated");
}

#[test]
fn apply_todo_update_step_preserves_empty_todowrite_result() {
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = Vec::new();
    let step = llm_step(
        models::TokenUsage::default(),
        "",
        vec![tool_call("call-todo", TODO_WRITE_TOOL_ID, "{}")],
    );

    let updated =
        super::apply_todo_update_step(&mut llm_messages, &mut usage, step, |_| String::new());

    assert!(updated);
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(llm_messages[1]["tool_call_id"], "call-todo");
    assert_eq!(llm_messages[1]["content"], "");
}

#[test]
fn apply_todo_update_step_executes_multiple_todowrites_in_order() {
    let mut usage = models::TokenUsage::default();
    let mut llm_messages = Vec::new();
    let step = llm_step(
        models::TokenUsage::default(),
        "update both items",
        vec![
            tool_call("call-first", TODO_WRITE_TOOL_ID, r#"{"todos":[{"id":"1"}]}"#),
            tool_call("call-read", "read", r#"{"path":"notes.md"}"#),
            tool_call("call-second", TODO_WRITE_TOOL_ALIAS, r#"{"todos":[{"id":"2"}]}"#),
        ],
    );
    let mut executed = Vec::new();

    let updated = super::apply_todo_update_step(&mut llm_messages, &mut usage, step, |call| {
        executed.push(call.id.clone());
        format!("updated {}", call.id)
    });

    assert!(updated);
    assert_eq!(executed, vec!["call-first", "call-second"]);
    assert_eq!(llm_messages.len(), 3);
    assert_eq!(llm_messages[1]["tool_call_id"], "call-first");
    assert_eq!(llm_messages[1]["content"], "updated call-first");
    assert_eq!(llm_messages[2]["tool_call_id"], "call-second");
    assert_eq!(llm_messages[2]["content"], "updated call-second");
}
