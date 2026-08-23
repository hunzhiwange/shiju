//! 验证用户与助手消息摄取时的去重、工具解析和事件生成行为。

use crate::app::agent::session::processor::StreamEvent;
use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::ToolRuntimeContext;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
}

fn allowed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn ctx(session: &str, root: Option<String>) -> ToolRuntimeContext {
    ToolRuntimeContext::new(session.to_string(), root)
}

#[test]
fn push_user_dedup_skips_empty_and_duplicate_trailing_user_message() {
    let mut session = Session::new(unique_session("ingest-dedup"));

    super::push_user_dedup(&mut session, "   ".to_string());
    assert!(session.messages.is_empty());

    super::push_user_dedup(&mut session, "hello".to_string());
    super::push_user_dedup(&mut session, " hello ".to_string());

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "hello");
}

#[test]
fn push_user_dedup_allows_same_text_after_non_user_message() {
    let mut session = Session::new(unique_session("ingest-dedup-after-tool"));

    super::push_user_dedup(&mut session, "hello".to_string());
    session.push(Role::Tool, "tool output".to_string());
    super::push_user_dedup(&mut session, "hello".to_string());

    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[2].role, Role::User);
}

#[test]
fn push_user_dedup_preserves_original_nonempty_content() {
    let mut session = Session::new(unique_session("ingest-preserve-user-content"));

    super::push_user_dedup(&mut session, "  hello\n".to_string());

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "  hello\n");
}

#[test]
fn ingest_user_query_ignores_whitespace_only_input() {
    let session_id = unique_session("ingest-user-whitespace");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&[]);
    let mut events = Vec::new();
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        " \n\t\n ",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
    );

    assert!(session.messages.is_empty());
    assert!(events.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn ingest_user_query_keeps_malformed_allowed_tool_call_as_text() {
    let session_id = unique_session("ingest-user-malformed-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["todoread"]);
    let mut events = Vec::new();
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        "/todoread {not-json}\nkept",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "/todoread {not-json}\nkept");
    assert!(events.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn ingest_user_query_consumes_multiline_tool_input_without_user_text() {
    let session_id = unique_session("ingest-user-multiline-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["definitely_missing_tool"]);
    let mut events = Vec::new();
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        "/definitely_missing_tool\n{\n  \"value\": true\n}",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::Tool);
    assert!(session.messages[0].content.contains("Unknown tool: definitely_missing_tool"));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::Delta(text) if text.contains("Unknown tool: definitely_missing_tool")
        )
    }));
    assert_eq!(tool_state.non_todo_tool_runs, 1);
}

#[test]
fn ingest_user_query_pushes_text_and_executes_inline_tool_calls() {
    let session_id = unique_session("ingest-user-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["todoread"]);
    let mut events = Vec::new();
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        "first paragraph\n/todoread {}\nsecond paragraph",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "first paragraph");
    assert_eq!(session.messages[1].role, Role::Tool);
    assert!(session.messages[1].content.contains("tool todoread"));
    assert_eq!(session.messages[2].role, Role::User);
    assert_eq!(session.messages[2].content, "second paragraph");
    assert!(
        events.iter().any(
            |event| matches!(event, StreamEvent::Delta(text) if text.contains("tool todoread"))
        )
    );
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn ingest_user_query_replaces_duplicate_last_user_content_with_trimmed_text() {
    let session_id = unique_session("ingest-user-replace");
    let mut session = Session::new(session_id.clone());
    session.push(Role::User, "hello ".to_string());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&[]);
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        " hello ",
        &ctx,
        &allowed,
        &mut |_event| true,
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "hello");
}

#[test]
fn ingest_user_query_ignores_disallowed_tool_lines_as_text() {
    let session_id = unique_session("ingest-user-disallowed");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["todoread"]);
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        "/not_allowed {}\nkept",
        &ctx,
        &allowed,
        &mut |_event| true,
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "/not_allowed {}\nkept");
}

#[test]
fn ingest_user_query_deduplicates_text_before_tool_but_keeps_same_text_after_tool() {
    let session_id = unique_session("ingest-user-dedup-around-tool");
    let mut session = Session::new(session_id.clone());
    session.push(Role::User, "repeat".to_string());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["definitely_missing_tool"]);
    let mut tool_state = super::super::super::ToolSessionState::default();

    super::ingest_user_query(
        &mut session,
        "repeat\n/definitely_missing_tool\nrepeat",
        &ctx,
        &allowed,
        &mut |_event| true,
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "repeat");
    assert_eq!(session.messages[1].role, Role::Tool);
    assert!(session.messages[1].content.contains("Unknown tool: definitely_missing_tool"));
    assert_eq!(session.messages[2].role, Role::User);
    assert_eq!(session.messages[2].content, "repeat");
    assert_eq!(tool_state.non_todo_tool_runs, 1);
}

#[test]
fn ingest_assistant_answer_returns_trimmed_text_when_no_tool_is_present() {
    let session_id = unique_session("ingest-assistant-text");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["file_read"]);
    let mut ran_tool = false;
    let mut tool_state = super::super::super::ToolSessionState::default();

    let text = super::ingest_assistant_answer(
        &mut session,
        "\n first line \n\n second line ",
        &ctx,
        &allowed,
        &mut |_event| true,
        &mut ran_tool,
        &mut tool_state,
    );

    assert_eq!(text, "first line\nsecond line");
    assert!(!ran_tool);
    assert!(session.messages.is_empty());
}

#[test]
fn ingest_assistant_answer_executes_tool_call_and_keeps_surrounding_text() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("note.txt"), "hello from file\n").expect("file");
    let session_id = unique_session("ingest-assistant-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, Some(workspace.path().to_string_lossy().to_string()));
    let allowed = allowed(&["file_read"]);
    let mut events = Vec::new();
    let mut ran_tool = false;
    let mut tool_state = super::super::super::ToolSessionState::default();

    let text = super::ingest_assistant_answer(
        &mut session,
        "before\n/file_read {\"path\":\"note.txt\"}\nafter",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut ran_tool,
        &mut tool_state,
    );

    assert_eq!(text, "before\nafter");
    assert!(ran_tool);
    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].role, Role::Assistant);
    assert!(session.messages[0].content.contains("/file_read"));
    assert_eq!(session.messages[1].role, Role::Tool);
    assert!(session.messages[1].content.contains("hello from file"));
    assert!(events.iter().any(
        |event| matches!(event, StreamEvent::Delta(text) if text.contains("\"status\":\"running\""))
    ));
    assert!(
        events.iter().any(
            |event| matches!(event, StreamEvent::Delta(text) if text.contains("tool file_read"))
        )
    );
    assert_eq!(tool_state.non_todo_tool_runs, 1);
}

#[test]
fn ingest_assistant_answer_keeps_rejected_tool_lines_as_text() {
    let session_id = unique_session("ingest-assistant-rejected-tools");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["todoread"]);
    let mut events = Vec::new();
    let mut ran_tool = false;
    let mut tool_state = super::super::super::ToolSessionState::default();

    let text = super::ingest_assistant_answer(
        &mut session,
        "/not_allowed {}\n/todoread {not-json}",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut ran_tool,
        &mut tool_state,
    );

    assert_eq!(text, "/not_allowed {}\n/todoread {not-json}");
    assert!(!ran_tool);
    assert!(session.messages.is_empty());
    assert!(events.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn ingest_assistant_answer_records_empty_input_unknown_tool_failure() {
    let session_id = unique_session("ingest-assistant-unknown-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ctx(&session_id, None);
    let allowed = allowed(&["definitely_missing_tool"]);
    let mut events = Vec::new();
    let mut ran_tool = false;
    let mut tool_state = super::super::super::ToolSessionState::default();

    let text = super::ingest_assistant_answer(
        &mut session,
        "/definitely_missing_tool",
        &ctx,
        &allowed,
        &mut |event| {
            events.push(event);
            true
        },
        &mut ran_tool,
        &mut tool_state,
    );

    assert!(text.is_empty());
    assert!(ran_tool);
    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].role, Role::Assistant);
    assert_eq!(session.messages[0].content, "/definitely_missing_tool");
    assert_eq!(session.messages[1].role, Role::Tool);
    assert!(session.messages[1].content.contains("Unknown tool: definitely_missing_tool"));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::Delta(text) if text.contains("Unknown tool: definitely_missing_tool")
        )
    }));
    assert_eq!(tool_state.non_todo_tool_runs, 1);
}
