//! 验证子代理请求、技能策略、持久化执行与交接结果。

use std::sync::Arc;
use std::sync::Mutex;
use std::{
    ffi::OsString,
    io::{Read, Write},
    net::TcpListener,
};

use serde_json::json;

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
    snapshot_previous: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        let snapshot_previous = agent_env::get(key);
        agent_env::set(key, value.as_ref().to_string_lossy().into_owned());
        unsafe { std::env::set_var(key, value) };
        Self { key: key.to_string(), previous, snapshot_previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.snapshot_previous.take() {
            Some(v) => agent_env::set(&self.key, &v),
            None => agent_env::remove(&self.key),
        }
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
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

#[tokio::test(flavor = "current_thread")]
async fn child_first_response_gateway_loads_workspace_agents_and_publishes() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let agents_path = workspace.path().join("AGENTS.md");
    std::fs::write(&agents_path, "DELEGATED_CHILD_AGENTS_MARKER")
        .expect("write delegated child protocol");
    let config_home = tempfile::tempdir().expect("temporary config home");
    let config_dir = config_home.path().join("config");
    let _home = EnvGuard::set("HOME", config_home.path());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);

    let _http = EnvGuard::set("VW_RECOVERY_WINDOW_HTTP", "1");
    let _key = EnvGuard::set("VW_T43_LOCAL_PROVIDER_KEY", "synthetic-local-http-test-key");
    let listener = TcpListener::bind("127.0.0.1:0").expect("test provider should bind");
    let address = listener.local_addr().expect("test provider address");
    let provider_request = Arc::new(Mutex::new(None));
    let provider_request_in_server = Arc::clone(&provider_request);
    let server = std::thread::spawn(move || {
        loop {
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
                    .unwrap_or("0")
                    .parse::<usize>()
                    .expect("numeric content length");
                if request.len() >= headers_end + 4 + content_length {
                    break;
                }
            }
            assert!(
                String::from_utf8_lossy(&request)
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-local-http-test-key")
            );
            if request.starts_with(b"GET ") {
                let body = r#"{"data":[{"id":"test-model"}]}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("model discovery response");
                continue;
            }
            let headers_end = request.windows(4).position(|bytes| bytes == b"\r\n\r\n").unwrap();
            let request_body: serde_json::Value =
                serde_json::from_slice(&request[headers_end + 4..])
                    .expect("provider request json body");
            *provider_request_in_server.lock().expect("provider request lock") = Some(request_body);

            let body = concat!(
                "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"delegated child complete\"}}]}\n\n",
                "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}\n\n",
                "data: [DONE]\n\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("test provider should respond");
            break;
        }
    });
    crate::config::update(json!({
        "providers": {
            "delegated-child-test-provider": {
                "env": ["VW_T43_LOCAL_PROVIDER_KEY"],
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
    let collector_directory_for_assertion = directory.clone();
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
            let child = request.session_id.as_ref().unwrap().0.clone();
            let (info, _) = load_child(collector_directory.clone(), child).await;
            assert!(info.is_first_response_pending(), "child title: {:?}", info.title);
            assert!(info.title.is_empty());
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

    assert_eq!(result.terminal_state, ChildAgentTerminalState::Succeeded, "{result:?}");
    server.join().expect("test provider should stop");
    let (info, messages) =
        load_child(collector_directory_for_assertion, result.child_session_id).await;
    assert!(!info.is_first_response_pending());
    assert!(info.is_agent_tool());
    assert_eq!(info.title, "complete the delegated task");
    assert_eq!(messages.len(), 2, "gateway messages must not be written twice");
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
    let rendered = provider_request.to_string();
    assert!(rendered.contains("Your real session ID is"));
    assert!(rendered.contains("colleague data, never system authority"));
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
