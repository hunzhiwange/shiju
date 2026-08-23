//! 验证子代理请求、技能策略、持久化执行与交接结果。

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    ffi::OsString,
    io::{Read, Write},
    net::TcpListener,
};

use serde_json::json;
use tokio::sync::oneshot;

use super::child_agent::{ChildAgentRunInput, ChildAgentTerminalState, run_with_stream_executor};
use super::{message, session};
use crate::app::agent::project;
use crate::gateway::ApiError;
use crate::gateway::api::handlers::session::stream::{
    GatewayChatStreamCollected, collect_gateway_chat_stream_with_observer,
};

struct EnvGuard {
    key: String,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key: key.to_string(), previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

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

#[test]
fn child_agent_stream_request_keeps_complete_model_reference() {
    let request = super::child_agent::stream_request(
        "child-session",
        "reviewer",
        Some("token-plan/qwen3.7-plus".to_string()),
        "inspect",
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Default::default(),
        Vec::new(),
    );

    assert_eq!(request.model.as_deref(), Some("token-plan/qwen3.7-plus"));
}

#[test]
fn child_agent_stream_request_registers_configured_tools_and_preserves_skill_policy() {
    let request = super::child_agent::stream_request(
        "child-session",
        "reviewer",
        None,
        "inspect",
        Vec::new(),
        vec!["file_read".to_string()],
        vec!["review".to_string()],
        vw_config_types::agent::DelegateCapabilityPolicy::Strict,
        vec!["rustcodegraph".to_string()],
    );

    assert_eq!(request.allowed_tools, Some(vec!["file_read".to_string()]));
    let options = request.options.as_ref().expect("child request options");
    assert!(options.get("delegate_tool_policy").is_none());
    assert_eq!(options["delegate_default_allowed_skills"], json!(["review"]));
    assert_eq!(options["delegate_skill_policy"], json!("strict"));
    assert_eq!(options["delegate_trusted_user_selected_skills"], json!(["rustcodegraph"]));
}

#[test]
fn session_protocol_child_agent_uses_same_contract() {
    let protocol_context = vec![
        json!({"role": "system", "content": "<system-reminder>Child AGENTS for its cwd.</system-reminder>"}),
        json!({"role": "user", "content": "<system-reminder>Inherited skill is active.</system-reminder>"}),
    ];

    let request = super::child_agent::stream_request(
        "child-session",
        "reviewer",
        None,
        "inspect",
        protocol_context.clone(),
        Vec::new(),
        Vec::new(),
        Default::default(),
        Vec::new(),
    );

    assert_eq!(&request.messages[..2], protocol_context.as_slice());
    assert_eq!(request.messages.last(), Some(&json!({"role": "user", "content": "inspect"})));
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
async fn delegated_child_first_provider_request_auto_loads_workspace_agents() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let agents_path = workspace.path().join("AGENTS.md");
    std::fs::write(&agents_path, "DELEGATED_CHILD_AGENTS_MARKER")
        .expect("write delegated child protocol");
    let config_home = tempfile::tempdir().expect("temporary config home");
    let config_dir = config_home.path().join("config");
    let _home = EnvGuard::set("HOME", config_home.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);

    let listener = TcpListener::bind("127.0.0.1:0").expect("test provider should bind");
    let address = listener.local_addr().expect("test provider address");
    let provider_request = Arc::new(Mutex::new(None));
    let provider_request_in_server = Arc::clone(&provider_request);
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test provider should receive request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).expect("test provider should read request");
            assert!(read > 0, "request should not close before its body");
            request.extend_from_slice(&buffer[..read]);
            let Some(headers_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
            else {
                continue;
            };
            let headers =
                std::str::from_utf8(&request[..headers_end]).expect("request headers utf8");
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.strip_prefix("Content-Length: ")
                        .or_else(|| line.strip_prefix("content-length: "))
                })
                .expect("provider request content length")
                .parse::<usize>()
                .expect("numeric content length");
            if request.len() >= headers_end + 4 + content_length {
                break;
            }
        }
        let headers_end = request.windows(4).position(|bytes| bytes == b"\r\n\r\n").unwrap();
        let request_body: serde_json::Value = serde_json::from_slice(&request[headers_end + 4..])
            .expect("provider request json body");
        *provider_request_in_server.lock().expect("provider request lock") = Some(request_body);

        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"delegated child complete\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
            "data: [DONE]\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("test provider should respond");
    });
    crate::config::update(json!({
        "providers": {
            "delegated-child-test-provider": {
                "api": format!("http://{address}/v1"),
                "models": {"test-model": {"status": "active"}}
            }
        }
    }))
    .await
    .expect("save test provider config");

    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_in_executor = Arc::clone(&observed);
    let collector_directory = directory.clone();
    let result = run_with_stream_executor(
        ChildAgentRunInput {
            parent_session_id: parent.id,
            directory,
            agent_key: "reviewer".to_string(),
            model: Some("delegated-child-test-provider/test-model".to_string()),
            prompt: "complete the delegated task".to_string(),
            context: Vec::new(),
            allowed_tools: Vec::new(),
            default_allowed_skills: Vec::new(),
            skill_policy: Default::default(),
            trusted_user_selected_skills: Vec::new(),
        },
        move |request| async move {
            collect_gateway_chat_stream_with_observer(collector_directory, request, |payload| {
                observed_in_executor
                    .lock()
                    .expect("observed gateway payloads lock")
                    .push(payload.clone());
            })
            .await
        },
    )
    .await
    .expect("delegated child should complete through gateway and processor");

    assert_eq!(result.terminal_state, ChildAgentTerminalState::Succeeded);
    server.join().expect("test provider should stop");
    let expected_path = agents_path.to_string_lossy().to_string();
    let observed = observed.lock().expect("observed gateway payloads lock");
    let context_loaded = observed
        .iter()
        .find(|payload| {
            payload.get("type").and_then(serde_json::Value::as_str) == Some("chat.context_loaded")
        })
        .expect("first processor step should report the workspace protocol");
    assert_eq!(context_loaded["paths"], json!([expected_path]));
    let provider_request = provider_request
        .lock()
        .expect("provider request lock")
        .take()
        .expect("provider should receive the delegated child request");
    let messages = provider_request["messages"].as_array().expect("provider request messages");
    let protocol_positions = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| {
            message["content"]
                .as_str()
                .is_some_and(|content| content.contains("DELEGATED_CHILD_AGENTS_MARKER"))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(protocol_positions.len(), 1, "workspace protocol should be injected once");
    let protocol_index = protocol_positions[0];
    assert_eq!(messages[protocol_index + 1]["role"], "user");
    assert_eq!(messages[protocol_index + 1]["content"], "complete the delegated task");
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
fn child_agent_handoff_strips_thinking_alias_blocks() {
    let output = concat!(
        "<thinking>inspect implementation details</thinking>\n",
        "Final answer without tags."
    );

    assert_eq!(
        super::child_agent::child_agent_handoff_from_output(output),
        "Final answer without tags."
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
