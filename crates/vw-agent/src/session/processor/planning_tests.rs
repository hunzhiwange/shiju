#[test]
fn planning_tests_module_is_wired() {
    let marker = String::from("planning_tests");
    assert_eq!(marker.as_str(), "planning_tests");
}

use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::todo;
use crate::app::agent::tools::{TODO_WRITE_TOOL_ALIAS, TODO_WRITE_TOOL_ID, ToolRuntimeContext};
use std::collections::HashSet;

fn unique_session_id(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    format!("planning-tests-{name}-{nanos}")
}

fn allowed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn todo_contents(input: &str) -> Vec<String> {
    let value = serde_json::from_str::<serde_json::Value>(input)
        .expect("todowrite input should be valid JSON");
    value["todos"]
        .as_array()
        .expect("todos should be an array")
        .iter()
        .map(|todo| todo["content"].as_str().expect("todo content should be string").to_string())
        .collect()
}

#[test]
fn build_todowrite_from_text_extracts_common_list_formats() {
    let input = super::build_todowrite_from_text(
        r#"
        1. Read the source
        2) Add focused tests
        - [ ] Update docs
        - [x] Keep regression covered
        * Run formatter
        /todowrite {}
        ```
        - ignored code item
        ```
        "#,
    )
    .expect("list text should produce todowrite input");

    let value = serde_json::from_str::<serde_json::Value>(&input)
        .expect("todowrite input should be valid JSON");
    let todos = value["todos"].as_array().expect("todos should be an array");

    assert_eq!(
        todo_contents(&input),
        vec![
            "Read the source",
            "Add focused tests",
            "Update docs",
            "Keep regression covered",
            "Run formatter",
        ]
    );
    assert_eq!(value["merge"].as_bool(), Some(false));
    assert_eq!(todos[0]["id"].as_str(), Some("1"));
    assert_eq!(todos[0]["status"].as_str(), Some("pending"));
    assert_eq!(todos[0]["priority"].as_str(), Some("medium"));
}

#[test]
fn build_todowrite_from_text_deduplicates_normalizes_and_limits_items() {
    let mut answer = String::new();
    answer.push_str("- Ship   feature\n");
    answer.push_str("- ship feature\n");
    for idx in 0..20 {
        answer.push_str(&format!("{}. item {idx}\n", idx + 1));
    }

    let input = super::build_todowrite_from_text(&answer)
        .expect("list text should produce todowrite input");
    let contents = todo_contents(&input);

    assert_eq!(contents.len(), 12);
    assert_eq!(contents[0], "Ship feature");
    assert_eq!(contents[1], "item 0");
    assert_eq!(contents[11], "item 10");
    assert_eq!(contents.iter().filter(|item| item.as_str() == "ship feature").count(), 0);
}

#[test]
fn build_todowrite_from_text_returns_none_for_empty_or_command_only_text() {
    assert!(super::build_todowrite_from_text("").is_none());
    assert!(super::build_todowrite_from_text("/todowrite {}").is_none());
    assert!(super::build_todowrite_from_text("```\\n- hidden\\n```").is_none());
    assert!(super::build_todowrite_from_text("- a").is_none());
}

#[test]
fn build_todowrite_from_text_handles_uppercase_checkbox_and_skips_empty_markers() {
    let input = super::build_todowrite_from_text(
        "- [X] Finished review\n- [ ]\n- [x]\n-\n*\n1.\n2)\n- /todowrite {}",
    )
    .expect("uppercase checkbox should produce a todowrite input");

    assert_eq!(todo_contents(&input), vec!["Finished review"]);
}

#[test]
fn build_todowrite_from_text_only_strips_numeric_list_prefixes() {
    let input = super::build_todowrite_from_text(
        "release.v1 stays intact\nphase) stays intact\n0. zero-based task\n007) padded task",
    )
    .expect("plain and numbered lines should produce a todowrite input");

    assert_eq!(
        todo_contents(&input),
        vec!["release.v1 stays intact", "phase) stays intact", "zero-based task", "padded task",]
    );
}

#[test]
fn build_todowrite_from_text_ignores_remainder_after_unclosed_code_fence() {
    let input = super::build_todowrite_from_text(
        "Visible task\n```rust\n- hidden task\n1. another hidden task",
    )
    .expect("text before an unclosed fence should remain available");

    assert_eq!(todo_contents(&input), vec!["Visible task"]);
}

#[test]
fn ingest_planning_answer_does_not_autogenerate_when_todowrite_is_not_allowed() {
    let session_id = unique_session_id("not-allowed");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut event_count = 0usize;

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        "- first task\n- second task",
        &ctx,
        &HashSet::new(),
        &mut |_event| {
            event_count += 1;
            true
        },
        &mut tool_state,
    );

    assert!(!ran);
    assert!(session.messages.is_empty());
    assert_eq!(event_count, 0);
}

#[test]
fn ingest_planning_answer_does_not_run_allowed_todowrite_without_extractable_items() {
    let session_id = unique_session_id("allowed-no-items");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut event_count = 0usize;

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        "/todowrite {}\n```\n- hidden task\n```",
        &ctx,
        &allowed(&[TODO_WRITE_TOOL_ID]),
        &mut |_event| {
            event_count += 1;
            true
        },
        &mut tool_state,
    );

    assert!(!ran);
    assert!(session.messages.is_empty());
    assert_eq!(event_count, 0);
}

#[test]
fn ingest_planning_answer_records_explicit_todowrite_without_input() {
    let session_id = unique_session_id("explicit-empty");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut event_count = 0usize;

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        &format!("/{TODO_WRITE_TOOL_ID}"),
        &ctx,
        &allowed(&[TODO_WRITE_TOOL_ID]),
        &mut |_event| {
            event_count += 1;
            true
        },
        &mut tool_state,
    );

    assert!(ran);
    assert_eq!(session.messages[0].role, Role::Assistant);
    assert_eq!(session.messages[0].content, format!("/{TODO_WRITE_TOOL_ID}"));
    assert!(session.messages.iter().any(|message| matches!(message.role, Role::Tool)));
    assert_eq!(event_count, 1);
}

#[test]
fn ingest_planning_answer_autogenerates_todowrite_for_list_text() {
    let session_id = unique_session_id("implicit");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut events = Vec::new();

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        "Plan:\n- inspect module\n- add tests",
        &ctx,
        &allowed(&[TODO_WRITE_TOOL_ID]),
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
    );

    assert!(ran);
    assert_eq!(session.messages[0].role, Role::Assistant);
    assert_eq!(session.messages[0].content, format!("/{TODO_WRITE_TOOL_ID}"));
    assert!(session.messages.iter().any(|message| matches!(message.role, Role::Tool)));
    assert!(events.iter().any(
        |event| matches!(event, super::StreamEvent::Delta(text) if text.contains("tool TodoWrite"))
    ));

    let stored = todo::read(&ctx).expect("implicit todowrite should write todos");
    assert!(stored.contains("inspect module"));
    assert!(stored.contains("add tests"));
}

#[test]
fn ingest_planning_answer_runs_explicit_todowrite_and_skips_implicit_fallback() {
    let session_id = unique_session_id("explicit");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut event_count = 0usize;
    let input = serde_json::json!({
        "todos": [
            { "id": "a", "content": "explicit task", "status": "pending", "priority": "high" }
        ]
    });
    let answer = format!("/{TODO_WRITE_TOOL_ALIAS} {input}\n- fallback task");

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        &answer,
        &ctx,
        &allowed(&[TODO_WRITE_TOOL_ALIAS]),
        &mut |_event| {
            event_count += 1;
            true
        },
        &mut tool_state,
    );

    assert!(ran);
    assert_eq!(session.messages[0].role, Role::Assistant);
    assert!(session.messages[0].content.starts_with("/todowrite "));
    assert_eq!(event_count, 1);

    let stored = todo::read(&ctx).expect("explicit todowrite should write todos");
    assert!(stored.contains("explicit task"));
    assert!(!stored.contains("fallback task"));
}

#[test]
fn ingest_planning_answer_consumes_multiline_todowrite_input_once() {
    let session_id = unique_session_id("explicit-multiline");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut event_count = 0usize;
    let answer = format!(
        "/{TODO_WRITE_TOOL_ID}\n{{\n  \"todos\": [\n    {{\"id\": \"1\", \"content\": \"multiline task\", \"status\": \"pending\", \"priority\": \"medium\"}}\n  ]\n}}"
    );

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        &answer,
        &ctx,
        &allowed(&[TODO_WRITE_TOOL_ID]),
        &mut |_event| {
            event_count += 1;
            true
        },
        &mut tool_state,
    );

    assert!(ran);
    assert_eq!(
        session.messages.iter().filter(|message| message.role == Role::Assistant).count(),
        1
    );
    assert_eq!(event_count, 1);
    let stored = todo::read(&ctx).expect("multiline todowrite should write todos");
    assert!(stored.contains("multiline task"));
}

#[test]
fn ingest_planning_answer_ignores_non_todowrite_tool_calls() {
    let session_id = unique_session_id("non-todo-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ToolRuntimeContext::new(session_id, None);
    let mut tool_state = super::super::ToolSessionState::default();
    let mut allowed_tools = allowed(&["file_read"]);
    allowed_tools.insert(TODO_WRITE_TOOL_ID.to_string());

    let ran = super::ingest_planning_answer_todowrite_only(
        &mut session,
        r#"/file_read {"path":"missing.rs"}"#,
        &ctx,
        &allowed_tools,
        &mut |_event| true,
        &mut tool_state,
    );

    assert!(!ran);
    assert!(session.messages.is_empty());
}
