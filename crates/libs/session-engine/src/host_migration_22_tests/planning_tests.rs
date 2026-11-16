use crate::{session_memory::Session, processor_events::StreamEvent};
use std::collections::HashSet;
use tool_types::todo_ids::TODO_WRITE_TOOL_ID;
struct GuardHost;
impl crate::planning::PlanningHost for GuardHost {
    type Context = ();
    type State = ();
    fn run_tool_and_record(_: &mut Session, _: &str, _: &str, _: &(), _: bool, _: &mut dyn FnMut(StreamEvent) -> bool, _: &mut ()) {
        panic!("rejected planning input must not invoke execution");
    }
}
fn unique_session_id(name: &str) -> String { format!("planning-{name}") }
fn allowed(names: &[&str]) -> HashSet<String> { names.iter().map(|s| s.to_string()).collect() }
#[test]
fn ingest_planning_answer_does_not_autogenerate_when_todowrite_is_not_allowed() {
    let session_id = unique_session_id("not-allowed");
    let mut session = Session::new(session_id.clone());
    let ctx = ();
    let mut tool_state = ();
    let mut event_count = 0usize;

    let ran = crate::planning::ingest_planning_answer_todowrite_only::<GuardHost>(
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
    let ctx = ();
    let mut tool_state = ();
    let mut event_count = 0usize;

    let ran = crate::planning::ingest_planning_answer_todowrite_only::<GuardHost>(
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
fn ingest_planning_answer_ignores_non_todowrite_tool_calls() {
    let session_id = unique_session_id("non-todo-tool");
    let mut session = Session::new(session_id.clone());
    let ctx = ();
    let mut tool_state = ();
    let mut allowed_tools = allowed(&["file_read"]);
    allowed_tools.insert(TODO_WRITE_TOOL_ID.to_string());

    let ran = crate::planning::ingest_planning_answer_todowrite_only::<GuardHost>(
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
