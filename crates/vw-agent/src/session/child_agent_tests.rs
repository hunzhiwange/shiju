use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tokio::sync::oneshot;

use super::child_agent::{ChildAgentRunInput, ChildAgentTerminalState, run_with_stream_executor};
use super::{message, session};
use crate::app::agent::project;
use crate::gateway::ApiError;
use crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected;

async fn create_parent(directory: String) -> session::Info {
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            session::create_next(session::CreateInput {
                parent_id: None,
                title: Some("Parent session".to_string()),
                directory,
                permission: None,
            })
            .await
        })
    })
    .await
    .expect("project context should initialize")
    .expect("parent session should persist")
}

async fn load_child(
    directory: String,
    child_session_id: String,
) -> (session::Info, Vec<message::WithParts>) {
    project::instance::provide(directory, None, move || {
        Box::pin(async move {
            let info = session::get(&child_session_id).await?;
            let messages = message::messages(&child_session_id, None).await?;
            Ok::<_, session::Error>((info, messages))
        })
    })
    .await
    .expect("project context should initialize")
    .expect("child session should load")
}

fn input(parent_session_id: String, directory: String, prompt: &str) -> ChildAgentRunInput {
    ChildAgentRunInput {
        parent_session_id,
        directory,
        agent_key: "reviewer".to_string(),
        model: None,
        prompt: prompt.to_string(),
        context: vec![json!({"role": "system", "content": "review carefully"})],
    }
}

#[tokio::test(flavor = "current_thread")]
async fn child_agent_run_creates_persistent_child_and_waits() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let completed = Arc::new(AtomicBool::new(false));
    let completed_in_executor = completed.clone();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    let task = tokio::spawn(run_with_stream_executor(
        input(parent.id.clone(), directory.clone(), "inspect the session lifecycle"),
        move |request| async move {
            assert_eq!(request.agent.as_deref(), Some("reviewer"));
            assert_eq!(request.model, None);
            assert!(request.messages.iter().any(|message| {
                message.get("role").and_then(serde_json::Value::as_str) == Some("system")
                    && message
                        .get("content")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|content| content.contains("key evidence"))
            }));
            assert_eq!(
                request.messages.last(),
                Some(&json!({
                    "role": "user",
                    "content": "inspect the session lifecycle"
                }))
            );
            started_tx
                .send(request.session_id.expect("child session id").as_ref().to_owned())
                .expect("signal executor start");
            release_rx.await.expect("release executor");
            completed_in_executor.store(true, Ordering::SeqCst);
            Ok(GatewayChatStreamCollected {
                output: "persistent child complete".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        },
    ));

    let child_session_id = started_rx.await.expect("executor should start");
    assert!(!task.is_finished(), "caller must wait for child execution");
    let (child, messages) = load_child(directory.clone(), child_session_id).await;
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
    assert!(messages.is_empty(), "child should be readable before its first turn completes");
    release_tx.send(()).expect("release child execution");
    let result = task
        .await
        .expect("child task should join")
        .expect("child execution should return a terminal result");

    assert!(completed.load(Ordering::SeqCst));
    assert_eq!(result.terminal_state, ChildAgentTerminalState::Succeeded);
    assert_eq!(result.output.as_deref(), Some("persistent child complete"));
    assert_eq!(result.error, None);

    let (child, messages) = load_child(directory, result.child_session_id.clone()).await;
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0].info, message::Info::Assistant(_)));
    assert!(matches!(messages[1].info, message::Info::User(_)));
    assert!(messages[0].parts.iter().any(
        |part| matches!(part, message::Part::Text(text) if text.text == "persistent child complete")
    ));
    assert!(messages[1].parts.iter().any(
        |part| matches!(part, message::Part::Text(text) if text.text == "inspect the session lifecycle")
    ));
}

#[test]
fn child_agent_handoff_execution_returns_last_visible_result() {
    let output = concat!(
        "<think>inspect implementation details</think>\n",
        "tool bash\n",
        "{\"status\":\"completed\",\"output\":\"cargo test output\"}\n\n",
        "Implemented the fix in src/lib.rs. Validation: focused test passed."
    );

    assert_eq!(
        super::child_agent::child_agent_handoff_from_output(output),
        "Implemented the fix in src/lib.rs. Validation: focused test passed."
    );
}

#[test]
fn child_agent_handoff_plain_text_is_preserved() {
    assert_eq!(
        super::child_agent::child_agent_handoff_from_output("simple execution result"),
        "simple execution result"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn child_agent_handoff_research_returns_sources_and_persists_full_stream() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let full_output = concat!(
        "<think>plan a broad search</think>\n",
        "tool web_search\n",
        "{\"status\":\"completed\",\"output\":\"raw search evidence\"}\n\n",
        "Conclusion: lifecycle state is stale.\n",
        "Evidence: source https://example.test/lifecycle documents terminal events.\n",
        "Uncertainty: reconnect behavior still needs validation."
    )
    .to_string();
    let executor_output = full_output.clone();

    let result = run_with_stream_executor(
        input(parent.id, directory.clone(), "research lifecycle synchronization"),
        move |_request| async move {
            Ok(GatewayChatStreamCollected {
                output: executor_output,
                message_id: None,
                parent_message_id: None,
            })
        },
    )
    .await
    .expect("research child should complete");

    let handoff = result.output.expect("parent should receive a handoff");
    assert!(handoff.starts_with("Conclusion:"));
    assert!(handoff.contains("https://example.test/lifecycle"));
    assert!(handoff.contains("Uncertainty:"));
    assert!(!handoff.contains("raw search evidence"));

    let (_, messages) = load_child(directory, result.child_session_id).await;
    assert!(
        messages[0]
            .parts
            .iter()
            .any(|part| matches!(part, message::Part::Text(text) if text.text == full_output))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn child_agent_failure_persists_child_history() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;

    let result = run_with_stream_executor(
        input(parent.id.clone(), directory.clone(), "find the failing path"),
        |_request| async move { Err(ApiError::internal("provider unavailable")) },
    )
    .await
    .expect("execution failure should still return a terminal result");

    assert_eq!(result.terminal_state, ChildAgentTerminalState::Failed);
    assert_eq!(result.output, None);
    assert!(result.error.as_deref().is_some_and(|error| error.contains("provider unavailable")));

    let (child, messages) = load_child(directory, result.child_session_id).await;
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0].info, message::Info::Assistant(ref info) if info.error.is_some()));
    assert!(messages[0].parts.iter().any(
        |part| matches!(part, message::Part::Text(text) if text.text.contains("provider unavailable"))
    ));
    assert!(messages[1].parts.iter().any(
        |part| matches!(part, message::Part::Text(text) if text.text == "find the failing path")
    ));
}
