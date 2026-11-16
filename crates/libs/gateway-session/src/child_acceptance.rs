//! 验证子代理请求、技能策略、持久化执行与交接结果。

use std::sync::Arc;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use std::{
    ffi::OsString,
    io::{Read, Write},
    net::TcpListener,
};

use serde_json::{Value, json};

use session_engine::child_agent::run::{ChildAgentRunInput, ChildAgentTerminalState};
use session_engine::message;
use session_engine::session_lifecycle as session;
use agent_project as project;

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
async fn create_parent<L: session_engine::session_lifecycle::LifecycleHost>(directory: String) -> session::Info {
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            session::create_next::<L>(session::CreateInput {
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

async fn load_child<L: session_engine::session_lifecycle::LifecycleHost>(
    directory: String,
    child_session_id: String,
) -> (session::Info, Vec<message::WithParts>) {
    project::instance::provide(directory, None, move || {
        Box::pin(async move {
            let info = session::get::<L>(&child_session_id).await?;
            let messages = message::messages(&child_session_id, None).await?;
            Ok::<_, session::Error>((info, messages))
        })
    })
    .await
    .expect("project context should initialize")
    .expect("child session should load")
}

pub async fn child_first_response_gateway_loads_workspace_agents_and_publishes<H, C, L>()
where H: crate::stream_host::StreamHost, C: session_engine::child_agent::run::ChildHost, L: session_engine::session_lifecycle::LifecycleHost {
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
    agent_config::runtime::update(json!({
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
    let parent = create_parent::<L>(directory.clone()).await;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_in_executor = Arc::clone(&observed);
    let collector_directory = directory.clone();
    let collector_directory_for_assertion = directory.clone();
    let result = session_engine::child_agent::run::run_with_stream_executor::<C, _, _>(
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
            let (info, _) = load_child::<L>(collector_directory.clone(), child).await;
            assert!(info.is_first_response_pending(), "child title: {:?}", info.title);
            assert!(info.title.is_empty());
            crate::stream::collect_gateway_chat_stream_with_observer::<H>(collector_directory, request, |payload| {
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
        load_child::<L>(collector_directory_for_assertion, result.child_session_id).await;
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

pub async fn child_first_response_continuation_preserves_published_title_and_history<C, L>()
where C: session_engine::child_agent::run::ChildHost, L: session_engine::session_lifecycle::LifecycleHost {
    use agent_project as project;
    use session_port::chat_stream::GatewayChatStreamCollected;
    use session_engine::child_agent::run::{self as child_agent, ChildAgentRunInput};
    use session_engine::{message, session_lifecycle as session};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().to_string();
    let parent_path = path.clone();
    let parent = project::instance::provide(path.clone(), None, move || {
        Box::pin(async move {
            session::create_next::<L>(session::CreateInput {
                parent_id: None,
                title: None,
                directory: parent_path,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    let mut created = child_agent::create::<C>(ChildAgentRunInput {
        parent_session_id: parent.id,
        directory: path.clone(),
        agent_key: "reviewer".into(),
        model: None,
        prompt: "first".into(),
        context: vec![],
        allowed_tools: vec![],
        default_allowed_skills: vec![],
        skill_policy: Default::default(),
        trusted_user_selected_skills: vec![],
    })
    .await
    .unwrap();
    let id = created.child_session_id.clone();
    child_agent::run_created_with_stream_executor::<C, _, _>(created.clone(), |_| async {
        Ok(GatewayChatStreamCollected {
            output: "first result".into(),
            message_id: None,
            parent_message_id: None,
        })
    })
    .await
    .unwrap();
    created.prepare_followup("second".into()).await.unwrap();
    let expected = id.clone();
    child_agent::run_created_with_stream_executor::<C, _, _>(created, move |request| async move {
        assert_eq!(request.session_id.unwrap().0, expected);
        assert!(request.messages.iter().any(|m| m["content"] == "first result"));
        Ok(GatewayChatStreamCollected {
            output: "second result".into(),
            message_id: None,
            parent_message_id: None,
        })
    })
    .await
    .unwrap();
    let (info, count) = project::instance::provide(path, None, move || {
        Box::pin(async move {
            (session::get::<L>(&id).await.unwrap(), message::messages(&id, None).await.unwrap().len())
        })
    })
    .await
    .unwrap();
    assert_eq!(count, 4);
    assert!(info.is_agent_tool());
    assert!(!info.is_first_response_pending());
    assert_eq!(info.title, "first");
}

pub struct HttpMode(pub Option<std::ffi::OsString>, pub Option<String>);
impl Drop for HttpMode {
    fn drop(&mut self) {
        match self.1.take() {
            Some(v) => agent_env::set("VW_AGCOLLAB_ENTRY_KEY", &v),
            None => agent_env::remove("VW_AGCOLLAB_ENTRY_KEY"),
        }
        unsafe {
            match &self.0 {
                Some(v) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", v),
                None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
            }
        }
    }
}

pub struct ParentModel {
    pub root: String,
}
#[async_trait::async_trait]
impl model_types::Provider for ParentModel {
    fn capabilities(&self) -> model_types::ProviderCapabilities {
        model_types::ProviderCapabilities {
            native_tool_search: false,
            native_tool_calling: true,
            vision: false,
        }
    }
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("structured boundary required")
    }
    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        assert!(request.tools.unwrap_or_default().iter().any(|t| t.id == "AgentTool"));
        let tasks = session_engine::child_agent::coordinator::global_child_agent_coordinator()
            .snapshots(&self.root, &[])?;
        let call = |id: &str, args: Value| model_types::ToolCall {
            kind: Default::default(),
            namespace: None,
            id: id.into(),
            name: "AgentTool".into(),
            arguments: args.to_string(),
        };
        let mut response = model_types::ChatResponse {
            text: None,
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        if tasks.is_empty() {
            response.tool_calls = vec![
                call(
                    "alpha",
                    json!({"agent":"worker","prompt":"child-task-alpha","fork_turns":"none"}),
                ),
                call(
                    "beta",
                    json!({"agent":"worker","prompt":"child-task-beta","fork_turns":"none"}),
                ),
            ];
        } else if !(request.messages.iter().any(|m| m.content.contains("ENTRY_CHILD_FACT_ALPHA"))
            && request.messages.iter().any(|m| m.content.contains("ENTRY_CHILD_FACT_BETA")))
        {
            response.tool_calls = vec![call(
                "wait",
                json!({"action":"wait","child_session_ids":tasks.iter().map(|t| &t.child_session_id).collect::<Vec<_>>(),"condition":"all","timeout_ms":10000}),
            )];
        } else {
            assert_eq!(tasks.len(), 2);
            assert!(tasks.iter().all(|t| t.state.is_terminal()));
            response.text = Some("ENTRY_INTEGRATED_ALPHA_BETA".into());
        }
        Ok(response)
    }
}


pub fn start_entry_server(
    listener: TcpListener,
    stopped: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<Value>>>,
    root: Arc<Mutex<String>>,
) -> std::thread::JoinHandle<()> {
    {
        let stopped = stopped.clone();
        let requests = requests.clone();
        let root = root.clone();
        std::thread::spawn(move || {
            let mut workers = Vec::new();
            while !stopped.load(Ordering::SeqCst) {
                let (mut stream, _) = listener.accept().unwrap();
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let requests = requests.clone();
                let root = root.clone();
                workers.push(std::thread::spawn(move || {
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(20))).unwrap();
                    let mut bytes = Vec::new(); let mut buffer = [0; 8192];
                    let (offset, length) = loop {
                        let n = match stream.read(&mut buffer) { Ok(0) => return, Ok(n) => n, Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) && bytes.is_empty() => return, Err(e) => panic!("provider request read: {e}") }; bytes.extend_from_slice(&buffer[..n]);
                        if let Some(offset) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&bytes[..offset]);
                            let length = headers.lines().find_map(|s| s.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                            if bytes.len() >= offset + 4 + length { break (offset + 4, length); }
                        }
                    };
                    if bytes.starts_with(b"GET ") {
                        let body = r#"{"data":[{"id":"original"}]}"#;
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap(); return;
                    }
                    let request: Value = serde_json::from_slice(&bytes[offset..offset+length]).unwrap();
                    { let mut captured = requests.lock().unwrap(); assert!(captured.len() < 24, "model script failed to advance: first={} second={}", captured.first().unwrap()["tools"], captured.get(1).unwrap()["messages"]); captured.push(request.clone()); }
                    let parent = request.to_string().contains("ROOT_ENTRY_WORKFLOW");
                    let response = if parent {
                        let text = request["messages"].to_string();
                        let tasks = session_engine::child_agent::coordinator::global_child_agent_coordinator().snapshots(&root.lock().unwrap(), &[]).unwrap();
                        if tasks.is_empty() {
                            json!({"tool_calls":[
                                {"index":0,"id":"alpha","type":"function","function":{"name":"AgentTool","arguments":json!({"agent":"worker","prompt":"child-task-alpha","fork_turns":"none"}).to_string()}},
                                {"index":1,"id":"beta","type":"function","function":{"name":"AgentTool","arguments":json!({"agent":"worker","prompt":"child-task-beta","fork_turns":"none"}).to_string()}}
                            ]})
                        } else if !(text.contains("ENTRY_CHILD_FACT_ALPHA") && text.contains("ENTRY_CHILD_FACT_BETA")) {
                            json!({"tool_calls":[{"index":0,"id":"wait","type":"function","function":{"name":"AgentTool","arguments":json!({"action":"wait","child_session_ids":tasks.iter().map(|t| &t.child_session_id).collect::<Vec<_>>(),"condition":"all","timeout_ms":10000}).to_string()}}]})
                        } else {
                            assert_eq!(tasks.len(), 2);
                            assert!(tasks.iter().all(|t| t.state.is_terminal()));
                            json!({"content":"ENTRY_INTEGRATED_ALPHA_BETA"})
                        }
                    } else {
                        assert!(request["messages"].to_string().contains("child-task-"));
                        json!({"content": if request["messages"].to_string().contains("child-task-alpha") { "ENTRY_CHILD_FACT_ALPHA" } else { "ENTRY_CHILD_FACT_BETA" }})
                    };
                    let finish = if response.get("tool_calls").is_some() { "tool_calls" } else { "stop" };
                    let (content_type, body) = if request["stream"] == true {
                        ("text/event-stream",format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",json!({"choices":[{"index":0,"delta":response}]}),json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}})))
                    } else { ("application/json",json!({"id":"fixture","choices":[{"index":0,"message":response,"finish_reason":finish}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}).to_string()) };
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        })
    }
}

/// Create the real persisted parent used by the AgentTool acceptance cases.
pub async fn create_collaboration_parent<L: session_engine::session_lifecycle::LifecycleHost>(directory: String) -> session::Info {
    let path = directory.clone();
    project::instance::provide(directory, None, move || {
        Box::pin(async move {
            session::create_next::<L>(session::CreateInput { parent_id: None, title: None, directory: path, permission: None }).await
        })
    }).await.unwrap().unwrap()
}
