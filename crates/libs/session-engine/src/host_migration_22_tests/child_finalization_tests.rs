//! Legacy finalization contract retained with its real domain coordinator.
use crate::child_agent::coordinator::{ChildAgentCoordinator, ChildAgentTaskSnapshot, ChildAgentTaskState};
use crate::session_memory::{Session, Role};
use crate::processor_events::StreamEvent;
use crate::tools_exec::tool_update_event;
use std::time::Duration;
fn agent_tool_snapshot_status(snapshot: &ChildAgentTaskSnapshot, waiting: bool) -> &'static str {
    if waiting && !snapshot.state.is_terminal() {
        return "waiting";
    }
    match snapshot.state {
        ChildAgentTaskState::Queued | ChildAgentTaskState::Running => "running",
        ChildAgentTaskState::Succeeded => "completed",
        ChildAgentTaskState::Failed => "failed",
        ChildAgentTaskState::Cancelled => "cancelled",
    }
}

fn agent_tool_snapshot_payload(
    snapshot: &ChildAgentTaskSnapshot,
    waiting: bool,
) -> serde_json::Value {
    let output = serde_json::to_string(&serde_json::json!({
        "action": "status",
        "state": snapshot.state,
        "parent_session_id": snapshot.parent_session_id,
        "child_session_id": snapshot.child_session_id,
        "agent": snapshot.agent,
        "task": snapshot.task,
        "output": snapshot.output,
        "error": snapshot.error,
    }))
    .expect("child agent snapshot should serialize");
    serde_json::json!({
        "status": agent_tool_snapshot_status(snapshot, waiting),
        "title": "AgentTool",
        "metadata": {
            "parent_session_id": snapshot.parent_session_id,
            "child_session_id": snapshot.child_session_id,
            "agent": snapshot.agent,
            "task": snapshot.task,
        },
        "output": output,
    })
}

fn finalize_agent_tool_children_with(
    coordinator: &ChildAgentCoordinator,
    parent_session_id: &str,
    session: &mut Session,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    child_session_ids: &[String],
) -> bool {
    if child_session_ids.is_empty() {
        return true;
    }

    let mut waiting_emitted = std::collections::HashSet::new();
    loop {
        let outcome = match coordinator.wait_for_descendants_blocking(
            parent_session_id,
            child_session_ids,
            Duration::from_millis(250),
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = coordinator.cancel_descendants(parent_session_id, child_session_ids);
                on_event(StreamEvent::Error(format!("AgentTool 子会话终态同步失败：{error}")));
                return false;
            }
        };

        if outcome.timed_out {
            for snapshot in outcome.tasks.iter().filter(|snapshot| {
                snapshot.parent_session_id == parent_session_id
                    && !snapshot.state.is_terminal()
                    && waiting_emitted.insert(snapshot.child_session_id.clone())
            }) {
                let payload = agent_tool_snapshot_payload(snapshot, true);
                if !on_event(tool_update_event(
                    snapshot.child_session_id.clone(),
                    "AgentTool",
                    payload,
                )) {
                    return false;
                }
            }
            continue;
        }

        for snapshot in
            outcome.tasks.iter().filter(|snapshot| snapshot.parent_session_id == parent_session_id)
        {
            let payload = agent_tool_snapshot_payload(snapshot, false);
            if !on_event(tool_update_event(
                snapshot.child_session_id.clone(),
                "AgentTool",
                payload.clone(),
            )) {
                return false;
            }
            let message = format!("tool AgentTool\n{payload}\n");
            session.push(Role::Tool, message);
        }
        return true;
    }
}

#[test]
fn agent_tool_finalization_emits_terminal_payloads_before_done() {
    let coordinator = crate::child_agent::coordinator::ChildAgentCoordinator::new(8);
    coordinator
        .register("parent", "child", "researcher", "collect evidence")
        .expect("child should register");
    coordinator.mark_running("parent", "child").expect("child should start");

    let finishing = coordinator.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        finishing
            .mark_succeeded("parent", "child", "findings with source")
            .expect("child should finish");
    });

    let mut session = Session::new("parent".to_string());
    let mut events = Vec::new();
    assert!(finalize_agent_tool_children_with(
        &coordinator,
        "parent",
        &mut session,
        &mut |event| {
            events.push(event);
            true
        },
        &["child".to_string()],
    ));
    events.push(StreamEvent::Done(Default::default()));
    worker.join().expect("worker should join");

    let texts = events
        .iter()
        .map(|event| match event {
            StreamEvent::Delta(text) => format!("delta:{text}"),
            StreamEvent::Tool(event) => {
                format!("delta:tool {}\n{}\n", event.name, event.payload)
            }
            StreamEvent::Done(_) => "done".to_string(),
            _ => "other".to_string(),
        })
        .collect::<Vec<_>>();
    assert!(texts[0].contains(r#""status":"completed""#));
    assert!(texts[0].contains(r#""child_session_id":"child""#));
    assert!(texts[0].contains("findings with source"));
    assert_eq!(texts.last().map(String::as_str), Some("done"));
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, Role::Tool);
    assert!(session.messages[0].content.contains(r#""status":"completed""#));
}

#[test]
// 事件接收端关闭不等于取消后台子代理，生命周期由协调器继续管理。
fn agent_tool_finalization_keeps_background_child_running_when_event_sink_closes() {
    let coordinator = crate::child_agent::coordinator::ChildAgentCoordinator::new(8);
    let cancellation = coordinator
        .register("parent", "child", "researcher", "collect evidence")
        .expect("child should register");
    coordinator.mark_running("parent", "child").expect("child should start");

    let mut session = Session::new("parent".to_string());
    assert!(!finalize_agent_tool_children_with(
        &coordinator,
        "parent",
        &mut session,
        &mut |_event| false,
        &["child".to_string()],
    ));

    let snapshots = coordinator
        .snapshots("parent", &["child".to_string()])
        .expect("child snapshot should remain available");
    assert_eq!(
        snapshots[0].state,
        crate::child_agent::coordinator::ChildAgentTaskState::Running
    );
    assert!(
        !cancellation.is_cancelled(),
        "closing the parent event sink must not cancel a background child"
    );
}

#[test]
fn tool_classification_helpers_cover_todo_streaming_and_regular_tools() {
    assert!(crate::tools_exec::should_rewrite_todowrite("todowrite", 0));
    assert!(!crate::tools_exec::should_rewrite_todowrite("todowrite", 1));
    assert!(!crate::tools_exec::should_rewrite_todowrite("todoread", 0));
    assert!(!crate::tools_exec::should_rewrite_todowrite("bash", 0));

    assert!(!crate::tools_exec::is_non_todo_tool("todowrite"));
    assert!(!crate::tools_exec::is_non_todo_tool("todoread"));
    assert!(crate::tools_exec::is_non_todo_tool("bash"));
    assert!(crate::tools_exec::is_non_todo_tool("write"));

    assert!(crate::tools_exec::should_emit_running_tool(true, "file_read"));
    assert!(!crate::tools_exec::should_emit_running_tool(false, "file_read"));
    assert!(!crate::tools_exec::should_emit_running_tool(true, "todoread"));
}
