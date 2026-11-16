//! Persistent child lifecycle uses real session and message stores.
use super::storage::{Host, session, Context as ToolUseContext};
use crate::message;
use agent_project as project;
use gateway_error::ApiError;
use session_port::chat_stream::GatewayChatStreamCollected;
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::oneshot;
use tool_context::context::scope_tool_use_context;
use crate::child_agent::run::{ChildAgentRunInput, ChildAgentTerminalState};
use permission_runtime::next as permission_types;
fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_19_CHILD_TEST";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        return false;
    }
    let root = tempfile::tempdir().expect("temporary storage root");
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg(format!("{}::{test}", module_path!().split_once("::").expect("crate module").1))
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_TEST, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VIBEWINDOW_TEST_HOME", root.path())
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test failed: {test}: {status}");
    true
}

// 父会话夹具先持久化根消息，为子会话归属和历史加载提供真实基线。
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
        allowed_tools: Vec::new(),
        default_allowed_skills: Vec::new(),
        skill_policy: Default::default(),
        trusted_user_selected_skills: Vec::new(),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn child_agent_run_creates_persistent_child_and_waits() {
    if run_in_temporary_storage("child_agent_run_creates_persistent_child_and_waits") { return; }
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let completed = Arc::new(AtomicBool::new(false));
    let completed_in_executor = completed.clone();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    let task = tokio::spawn(crate::child_agent::run::run_with_stream_executor::<Host, _, _>(
        input(parent.id.clone(), directory.clone(), "inspect the session lifecycle"),
        move |request| async move {
            assert_eq!(request.agent.as_deref(), Some("reviewer"));
            assert_eq!(request.model, None);
            assert!(request.messages.iter().any(|message| {
                message.get("role").and_then(serde_json::Value::as_str) == Some("system")
                    && message.get("content").and_then(serde_json::Value::as_str).is_some_and(
                        |content| {
                            content.contains("key evidence")
                                && content.contains("Do not pass the full task")
                        },
                    )
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

#[tokio::test(flavor = "current_thread")]
async fn child_agent_handoff_research_returns_sources_and_persists_full_stream() {
    if run_in_temporary_storage("child_agent_handoff_research_returns_sources_and_persists_full_stream") { return; }
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

    let result = crate::child_agent::run::run_with_stream_executor::<Host, _, _>(
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
    if run_in_temporary_storage("child_agent_failure_persists_child_history") { return; }
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;

    let result = crate::child_agent::run::run_with_stream_executor::<Host, _, _>(
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

#[tokio::test(flavor = "current_thread")]
async fn agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate() {
    if run_in_temporary_storage("agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate") { return; }
    use crate::child_agent::run::ForkTurns;
    use model_types::ChatMessage;
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    for fork in [ForkTurns::All, ForkTurns::None] {
        let context = Arc::new(
            ToolUseContext::new(&parent.id, Some(directory.clone()))
                .with_messages_view(vec![
                    ChatMessage::user("parent constraint"),
                    ChatMessage::assistant(
                        "<think>inherited-secret-reasoning</think>visible conclusion",
                    ),
                ])
                .with_tool_exposure_snapshot(["file_read".to_string()])
                .with_delegate_skill_policy(
                    config_agent::DelegateCapabilityPolicy::Strict,
                    ["allowed"],
                ),
        );
        let mut request_input = input(parent.id.clone(), directory.clone(), "separate task");
        request_input.default_allowed_skills = vec!["forbidden".into()];
        request_input.allowed_tools = vec!["file_read".into(), "shell".into()];
        let created =
            scope_tool_use_context(context, crate::child_agent::run::create_with_fork::<Host>(request_input, fork)).await.unwrap();
        let changed = Arc::new(
            ToolUseContext::new(&parent.id, Some(directory.clone()))
                .with_messages_view(vec![ChatMessage::user("later mutation")]),
        );
        let result = scope_tool_use_context(
            changed,
            crate::child_agent::run::run_created_with_stream_executor::<Host, _, _>(created, move |request| async move {
                assert_eq!(request.allowed_tools, Some(vec!["file_read".to_string()]));
                let serialized = serde_json::to_string(&request.messages).unwrap();
                assert_eq!(serialized.contains("parent constraint"), fork == ForkTurns::All);
                assert_eq!(serialized.contains("visible conclusion"), fork == ForkTurns::All);
                assert!(!serialized.contains("inherited-secret-reasoning"));
                assert!(!serialized.contains("later mutation"));
                assert_eq!(
                    request.messages.last(),
                    Some(&json!({"role":"user","content":"separate task"}))
                );
                assert!(
                    request
                        .messages
                        .iter()
                        .any(|message| message
                            == &json!({"role":"user","content":"review carefully"}))
                );
                assert_eq!(request.options.as_ref().unwrap()["delegate_skill_policy"], "strict");
                assert_eq!(
                    request.options.as_ref().unwrap()["delegate_default_allowed_skills"],
                    json!(["allowed"])
                );
                Ok(GatewayChatStreamCollected {
                    output: "done".into(),
                    message_id: None,
                    parent_message_id: None,
                })
            }),
        )
        .await
        .unwrap();
        let (child, messages) = load_child(directory.clone(), result.child_session_id).await;
        assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(messages.len(), 2);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn agcollab_child_rejects_cross_workspace_and_tenant() {
    if run_in_temporary_storage("agcollab_child_rejects_cross_workspace_and_tenant") { return; }
    let workspace = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let context =
        Arc::new(ToolUseContext::new(&parent.id, Some(other.path().to_string_lossy().to_string())));
    let result = scope_tool_use_context(
        context,
        crate::child_agent::run::create::<Host>(input(parent.id.clone(), directory.clone(), "task")),
    )
    .await;
    assert!(format!("{}", result.unwrap_err()).contains("workspace mismatch"));
    let context = Arc::new(ToolUseContext::new(&parent.id, Some(directory.clone())));
    let tenant = vw_api_transport::TenantScope::new("other-platform", "other-company").unwrap();
    let result = session_store::ui_store::with_tenant_scope(
        tenant,
        scope_tool_use_context(
            context,
            crate::child_agent::run::create::<Host>(input(parent.id, directory, "task")),
        ),
    )
    .await;
    assert!(format!("{}", result.unwrap_err()).contains("tenant mismatch"));
}

#[tokio::test(flavor = "current_thread")]
async fn agcollab_stored_history_and_permissions_are_snapshotted() {
    if run_in_temporary_storage("agcollab_stored_history_and_permissions_are_snapshotted") { return; }
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let parent_id = parent.id.clone();
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            session::update(&parent_id, |info| {
                info.permission = Some(vec![permission_types::Rule {
                    permission: "edit".into(),
                    pattern: "private/**".into(),
                    action: permission_types::Action::Deny,
                }]);
            })
            .await
            .unwrap();
            <Host as crate::child_agent::run::ChildHost>::persist_child_agent_turn(
                &parent_id,
                "stored user constraint",
                "stored visible conclusion",
                "main",
                None,
            )
            .await
            .unwrap();
        })
    })
    .await
    .unwrap();
    let created =
        crate::child_agent::run::create::<Host>(input(parent.id.clone(), directory.clone(), "new task"))
            .await
            .unwrap();
    let parent_id = parent.id.clone();
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            <Host as crate::child_agent::run::ChildHost>::persist_child_agent_turn(
                &parent_id,
                "later parent mutation",
                "later answer",
                "main",
                None,
            )
            .await
            .unwrap();
        })
    })
    .await
    .unwrap();
    let result =
        crate::child_agent::run::run_created_with_stream_executor::<Host, _, _>(created, |request| async move {
            let serialized = serde_json::to_string(&request.messages).unwrap();
            assert!(serialized.contains("stored user constraint"));
            assert!(serialized.contains("stored visible conclusion"));
            assert!(!serialized.contains("later parent mutation"));
            Ok(GatewayChatStreamCollected {
                output: "done".into(),
                message_id: None,
                parent_message_id: None,
            })
        })
        .await
        .unwrap();
    let (child, _) = load_child(directory, result.child_session_id).await;
    let rules = child.permission.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern, "private/**");
    assert_eq!(rules[0].action, permission_types::Action::Deny);
}
