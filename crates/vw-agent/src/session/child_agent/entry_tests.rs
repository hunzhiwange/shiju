//! Both public execution paths use the real tool factory and child Gateway loops.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct HttpMode(Option<std::ffi::OsString>, Option<String>);
impl Drop for HttpMode {
    fn drop(&mut self) {
        match self.1.take() {
            Some(v) => crate::env::set("VW_AGCOLLAB_ENTRY_KEY", &v),
            None => crate::env::remove("VW_AGCOLLAB_ENTRY_KEY"),
        }
        unsafe {
            match &self.0 {
                Some(v) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", v),
                None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
            }
        }
    }
}

struct ParentModel {
    root: String,
}
#[async_trait::async_trait]
impl crate::providers::Provider for ParentModel {
    fn capabilities(&self) -> crate::providers::traits::ProviderCapabilities {
        crate::providers::traits::ProviderCapabilities {
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
        request: crate::providers::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<crate::providers::ChatResponse> {
        assert!(request.tools.unwrap_or_default().iter().any(|t| t.id == "AgentTool"));
        let tasks = crate::session::child_agent::coordinator::global_child_agent_coordinator()
            .snapshots(&self.root, &[])?;
        let call = |id: &str, args: Value| crate::providers::ToolCall {
            id: id.into(),
            name: "AgentTool".into(),
            arguments: args.to_string(),
        };
        let mut response = crate::providers::ChatResponse {
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

#[tokio::test]
async fn agcollab_end_to_end_gateway_and_query_engine_entries() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let http = HttpMode(
        std::env::var_os("VW_RECOVERY_WINDOW_HTTP"),
        crate::env::get("VW_AGCOLLAB_ENTRY_KEY"),
    );
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    crate::env::set("VW_AGCOLLAB_ENTRY_KEY", "synthetic-key");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        &format!("http://{address}/v1"),
        Some("synthetic-key"),
    )
    .await;
    crate::config::update(json!({"providers":{"model-recovery":{"env":["VW_AGCOLLAB_ENTRY_KEY"],"models":{"original":{"status":"active","tool_call":true}}}}})).await.unwrap();
    crate::provider::provider::invalidate_cache().await;
    let stopped = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let root = Arc::new(Mutex::new(String::new()));
    let server = {
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
                        let tasks = crate::session::child_agent::coordinator::global_child_agent_coordinator().snapshots(&root.lock().unwrap(), &[]).unwrap();
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
    };
    for gateway in [true, false] {
        let workspace = tempfile::tempdir().unwrap();
        crate::config::update_project(workspace.path(),json!({"agents":{
            "main":{"provider":"model-recovery","model":"original","allowed_tools":["AgentTool"],"max_iterations":12},
            "worker":{"provider":"model-recovery","model":"original","allowed_tools":[],"allow_as_subagent":true}
        }})).await.unwrap();
        let directory = workspace.path().to_string_lossy().to_string();
        let path = directory.clone();
        let parent = crate::project::instance::provide(directory.clone(), None, move || {
            Box::pin(async move {
                crate::session::session::create_next(crate::session::session::CreateInput {
                    parent_id: None,
                    title: None,
                    directory: path,
                    permission: None,
                })
                .await
            })
        })
        .await
        .unwrap()
        .unwrap();
        *root.lock().unwrap() = parent.id.clone();
        let before = requests.lock().unwrap().len();
        let result = if gateway {
            crate::gateway::api::handlers::session::stream::collect_gateway_chat_stream(
                directory.clone(),
                vw_api_types::chat::GatewayChatStreamRequest {
                    session_id: Some(parent.id.clone().into()),
                    messages: vec![json!({"role":"user","content":"ROOT_ENTRY_WORKFLOW"})],
                    agent: Some("main".into()),
                    allowed_tools: Some(vec!["AgentTool".into()]),
                    model: Some("model-recovery/original".into()),
                    options: Some(json!({"full_access":true})),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .output
        } else {
            let mut config = crate::config::get_project(workspace.path()).await.unwrap();
            config.default_provider = Some("model-recovery".into());
            config.api_url = Some(format!("http://{address}/v1"));
            config.api_key = Some("synthetic-key".into());
            config.default_model = Some("original".into());
            config.workspace_dir = workspace.path().to_path_buf();
            config.autonomy.level = crate::security::AutonomyLevel::Full;
            let mut engine = crate::agent::loop_::query_engine::QueryEngine::new_with_provider(
                config,
                &parent.id,
                Some(Arc::new(ParentModel { root: parent.id.clone() })),
            )
            .await
            .unwrap();
            engine.submit_message("ROOT_ENTRY_WORKFLOW").await.unwrap()
        };
        assert!(result.contains("ENTRY_INTEGRATED_ALPHA_BETA"), "gateway={gateway}: {result}");
        let tasks = crate::session::child_agent::coordinator::global_child_agent_coordinator()
            .snapshots(&parent.id, &[])
            .unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(requests.lock().unwrap().len() >= before + if gateway { 4 } else { 2 });
        for child in tasks {
            crate::session::child_agent::coordinator::global_child_agent_coordinator()
                .cancel(&parent.id, &child.child_session_id)
                .unwrap();
        }
    }
    stopped.store(true, Ordering::SeqCst);
    let _ = std::net::TcpStream::connect(address);
    server.join().unwrap();
    drop(fixture);
    drop(http);
}
