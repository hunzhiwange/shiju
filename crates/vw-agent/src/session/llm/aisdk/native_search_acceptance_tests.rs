//! Offline acceptance across real Responses HTTP, request conversion and executors.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::provider::provider;
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, Provider};
use crate::session::llm::StreamEvent;
use crate::tools::{Tool, ToolSpec};

const READ: &str = "mcp__fixture__ledger";
const CONTENT: &str = "acceptance-ledger-content";

struct Ledger(std::path::PathBuf);
#[async_trait]
impl Tool for Ledger {
    fn name(&self) -> &str {
        READ
    }
    fn description(&self) -> &str {
        "Read ledger"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn spec(&self) -> ToolSpec {
        let mut spec = ToolSpec::new(READ, self.description(), self.parameters_schema());
        spec.exposure = vw_api_types::tools::ToolExposure::Deferred;
        spec.source = vw_api_types::tools::ToolSource::Mcp;
        spec.identity =
            vw_api_types::tools::ToolIdentity::new(Some("fixture".into()), "ledger").unwrap();
        spec
    }
    async fn execute(&self, _: Value) -> anyhow::Result<crate::tools::ToolResult> {
        let output = std::fs::read_to_string(&self.0)?;
        std::fs::write(self.0.with_extension("invoked"), "executed")?;
        Ok(crate::tools::ToolResult { success: true, output, error: None })
    }
}

/// Own the server and join on every exit, including assertion unwinding. No sleeps.
struct ResponsesFixture {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: std::sync::mpsc::Sender<()>,
    join: Option<std::thread::JoinHandle<()>>,
}
impl ResponsesFixture {
    fn new(responses: Vec<Vec<Value>>) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let (stop, stopping) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            let mut responses = responses.into_iter();
            loop {
                let (mut socket, _) = listener.accept().unwrap();
                if stopping.try_recv().is_ok() {
                    break;
                }
                socket.set_read_timeout(Some(std::time::Duration::from_secs(15))).unwrap();
                let mut data = Vec::new();
                let mut buffer = [0; 8192];
                let end = loop {
                    let n = socket.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    data.extend_from_slice(&buffer[..n]);
                    if let Some(i) = data.windows(4).position(|s| s == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&data[..end]);
                if headers.starts_with("GET ") {
                    let body = r#"{"data":[{"id":"original"}]}"#;
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
                    continue;
                }
                assert!(headers.starts_with("POST /v1/responses "), "{headers}");
                let length: usize = headers
                    .lines()
                    .find_map(|s| {
                        s.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse().unwrap())
                    })
                    .unwrap();
                while data.len() < end + length {
                    let n = socket.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    data.extend_from_slice(&buffer[..n]);
                }
                captured
                    .lock()
                    .unwrap()
                    .push(serde_json::from_slice(&data[end..end + length]).unwrap());
                let output = responses.next().expect("unexpected extra inference request");
                let mut events = Vec::new();
                for (i, item) in output.iter().enumerate() {
                    events.push(json!({"type":"response.output_item.added","sequence_number":i*4+1,"output_index":i,"item":item}));
                    if item["type"] == "function_call" {
                        events.push(json!({"type":"response.function_call_arguments.delta","sequence_number":i*4+2,"item_id":item["id"],"output_index":i,"delta":item["arguments"]}));
                        events.push(json!({"type":"response.function_call_arguments.done","sequence_number":i*4+3,"item_id":item["id"],"output_index":i,"arguments":item["arguments"]}));
                    }
                    events.push(json!({"type":"response.output_item.done","sequence_number":i*4+4,"output_index":i,"item":item}));
                }
                events.push(json!({"type":"response.completed","sequence_number":100,"response":{"id":"fixture-response","output":output}}));
                let body = events.iter().map(|e| format!("data: {e}\n\n")).collect::<String>();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
            }
        });
        Self { url: format!("http://{address}/v1/responses"), requests, stop, join: Some(join) }
    }
    fn requests(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for ResponsesFixture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        let address =
            self.url.strip_prefix("http://").unwrap().strip_suffix("/v1/responses").unwrap();
        let _ = std::net::TcpStream::connect(address);
        if let Some(join) = self.join.take() {
            let result = join.join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
}
fn search(native: bool, args: Value) -> Value {
    if native {
        json!({"type":"tool_search_call","id":"search-item","call_id":"search-call","status":"completed","execution":"client","arguments":args})
    } else {
        json!({"type":"function_call","id":"search-item","call_id":"search-call","status":"completed","name":"tool_search","arguments":args.to_string()})
    }
}
fn read(native: bool) -> Value {
    let mut item = json!({"type":"function_call","id":"read-item","call_id":"read-call","status":"completed","name":if native {"ledger"} else {READ},"arguments":"{}"});
    if native {
        item["namespace"] = json!("fixture");
    }
    item
}
fn finished() -> Value {
    json!({"type":"message","id":"final-item","role":"assistant","status":"completed","content":[{"type":"output_text","text":"ledger inspected","annotations":[],"logprobs":[]}]})
}
async fn configure(
    url: &str,
    native: bool,
) -> crate::session::llm::recovery_test_support::RecoveryConfig {
    let config =
        crate::session::llm::recovery_test_support::RecoveryConfig::new(url, Some("local-fixture"))
            .await;
    crate::config::update(json!({"providers":{"model-recovery":{"adapter":"openai-responses","models":{"original":{"native_tool_search":native,"toolcall":true}}}}})).await.unwrap();
    provider::invalidate_cache().await;
    config
}

/// Only the model boundary is adapted; search, exposure, execution and history use production code.
struct HttpProvider {
    model: provider::Model,
    info: provider::Info,
}
#[async_trait]
impl Provider for HttpProvider {
    fn capabilities(&self) -> crate::providers::traits::ProviderCapabilities {
        provider::resolve_native_search_capabilities(
            &self.model.capabilities,
            "openai-responses",
            "responses",
        )
    }
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("structured request required")
    }
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        let messages: Vec<_> = request
            .messages
            .iter()
            .map(|m| {
                if matches!(m.role.as_str(), "assistant" | "tool")
                    && let Ok(mut value) = serde_json::from_str::<Value>(&m.content)
                    && value.is_object()
                {
                    value["role"] = json!(m.role);
                    return value;
                }
                json!({"role":m.role,"content":m.content})
            })
            .collect();
        let tools =
            request.tools.unwrap_or_default().iter().map(|s| (s.id.clone(), s.clone())).collect();
        let mut response = ChatResponse {
            text: Some(String::new()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        };
        super::do_stream_request_aisdk(
            &self.info,
            None,
            &HashMap::new(),
            &json!({}),
            &self.model,
            &json!(messages),
            &tools,
            None,
            None,
            None,
            0,
            None,
            &mut |event| match event {
                StreamEvent::Delta(text) => response.text.as_mut().unwrap().push_str(&text),
                StreamEvent::ToolCalls(calls) => {
                    response.tool_calls.extend(calls.into_iter().map(|c| {
                        crate::providers::ToolCall {
                            kind: c.kind,
                            namespace: c.namespace,
                            id: c.id,
                            name: c.name,
                            arguments: c.arguments,
                        }
                    }))
                }
                _ => {}
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(response)
    }
}
async fn run_core(
    server: &ResponsesFixture,
    native: bool,
    workspace: &std::path::Path,
) -> (anyhow::Result<String>, Vec<ChatMessage>) {
    let _config = configure(&server.url, native).await;
    let provider = HttpProvider {
        model: provider::get_model("model-recovery", "original").await.unwrap(),
        info: provider::get_provider("model-recovery").await.unwrap().unwrap(),
    };
    let mut tools: Vec<Box<dyn Tool>> = vec![Box::new(Ledger(workspace.join("ledger")))];
    let catalog = crate::tools::discovery::ToolDiscoveryCatalog::new(
        tools.iter().map(|t| t.spec()).collect(),
    );
    tools.push(Box::new(crate::tools::tool_search::ToolSearchTool::new(&catalog)));
    let mut history = vec![ChatMessage::user("inspect ledger")];
    let result = crate::agent::loop_::core::run_tool_call_loop(
        &provider,
        &mut history,
        &tools,
        &crate::observability::NoopObserver,
        "acceptance",
        "model-recovery/original",
        0.0,
        true,
        None,
        "cli",
        &crate::config::MultimodalConfig::default(),
        5,
        None,
        None,
        None,
        None,
        &[],
    )
    .await;
    (result, history)
}
#[tokio::test]
async fn native_tool_search_acceptance_native_and_function_discovery_complete_same_task() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), CONTENT).unwrap();
    for native in [true, false] {
        let server = ResponsesFixture::new(vec![
            vec![search(native, json!({"query":"ledger"}))],
            vec![read(native)],
            vec![finished()],
        ]);
        let (result, history) = run_core(&server, native, workspace.path()).await;
        assert_eq!(result.unwrap(), "ledger inspected");
        assert!(workspace.path().join("ledger.invoked").exists());
        assert!(history.iter().any(|m| m.role == "tool" && m.content.contains(CONTENT)));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(!requests[0].to_string().contains(READ));
        let input = requests[1]["input"].as_array().unwrap();
        let output = input
            .iter()
            .find(|i| {
                i["call_id"] == "search-call"
                    && i["type"]
                        == if native { "tool_search_output" } else { "function_call_output" }
            })
            .expect("search result crosses HTTP");
        if native {
            assert!(
                requests[1]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["type"] == "namespace"
                        && tool["name"] == "fixture"
                        && tool["tools"][0]["name"] == "ledger")
            );
            assert_eq!(output["tools"][0]["type"], "namespace");
            assert_eq!(output["tools"][0]["name"], "fixture");
            assert_eq!(output["tools"][0]["tools"][0]["name"], "ledger");
        } else {
            assert!(
                requests[1]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["type"] == "function" && tool["name"] == READ)
            );
            assert!(output["output"].as_str().unwrap().contains(READ));
            assert!(
                requests.iter().all(|r| r["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|t| t["type"] == "function"))
            );
        }
        assert!(
            requests[2]["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["type"] == "function_call_output"
                    && i["call_id"] == "read-call"
                    && i["output"].as_str().is_some_and(|s| s.contains(CONTENT)))
        );
    }
}

struct HttpMode(Option<std::ffi::OsString>);
impl HttpMode {
    fn enable() -> Self {
        let saved = std::env::var_os("VW_RECOVERY_WINDOW_HTTP");
        unsafe {
            std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
        }
        Self(saved)
    }
}
impl Drop for HttpMode {
    fn drop(&mut self) {
        unsafe {
            match &self.0 {
                Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
            }
        }
    }
}
fn desktop_request(workspace: &std::path::Path) -> crate::session::processor::Request {
    crate::session::processor::Request {
        stream: 1,
        session: format!("native-acceptance-{}", uuid::Uuid::new_v4()),
        query: "inspect ledger".into(),
        root: Some(workspace.to_string_lossy().into()),
        model: Some("model-recovery/original".into()),
        reliability: crate::config::ReliabilityConfig {
            provider_retries: 0,
            provider_backoff_ms: 0,
            ..Default::default()
        },
        options: json!({"allowed_tools":["tool_search", READ],"full_access":true}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: crate::session::runtime::SessionToolset::new(vec![Arc::new(Ledger(
            workspace.join("ledger"),
        ))]),
        session_skills: Default::default(),
    }
}
#[test]
fn native_tool_search_acceptance_desktop_completes_both_http_paths() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _http = HttpMode::enable();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), CONTENT).unwrap();
    for native in [true, false] {
        let server = ResponsesFixture::new(vec![
            vec![search(native, json!({"query":"ledger"}))],
            vec![read(native)],
            vec![finished()],
        ]);
        let _config = runtime.block_on(configure(&server.url, native));
        let events = Arc::new(Mutex::new(Vec::new()));
        let collected = events.clone();
        crate::session::processor::run(desktop_request(workspace.path()), move |event| {
            collected.lock().unwrap().push(event);
            true
        });
        let requests = server.requests();
        assert_eq!(requests.len(), 3, "events: {:?}", events.lock().unwrap());
        assert!(!requests[0].to_string().contains("\"name\":\"ledger\""));
        let result_type = if native { "tool_search_output" } else { "function_call_output" };
        assert!(
            requests[1]["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["type"] == result_type && i["call_id"] == "search-call")
        );
        assert!(
            requests[2]["input"].as_array().unwrap().iter().any(|i| i["call_id"] == "read-call"
                && i["output"].as_str().is_some_and(|s| s.contains(CONTENT)))
        );
    }
}
#[tokio::test]
async fn native_tool_search_acceptance_same_batch_and_empty_search_cannot_grant_access() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), CONTENT).unwrap();
    for native in [true, false] {
        let server = ResponsesFixture::new(vec![
            vec![search(native, json!({"query":"ledger"})), read(native)],
            vec![finished()],
        ]);
        let (result, history) = run_core(&server, native, workspace.path()).await;
        assert!(!workspace.path().join("ledger.invoked").exists());
        assert!(!history.iter().any(|m| m.role == "tool" && m.content.contains(CONTENT)));
        if native {
            assert!(
                result.unwrap_err().to_string().contains("unknown or unauthorized tool identity")
            );
            assert_eq!(server.requests().len(), 1);
        } else {
            assert_eq!(result.unwrap(), "ledger inspected");
            assert!(
                history.iter().any(|m| m.content.contains("not exposed in this model request"))
            );
            assert_eq!(server.requests().len(), 2);
        }
        let server = ResponsesFixture::new(vec![
            vec![search(native, json!({"query":"absentfixture"}))],
            vec![finished()],
        ]);
        assert_eq!(
            run_core(&server, native, workspace.path()).await.0.unwrap(),
            "ledger inspected"
        );
        assert!(!server.requests()[1]["tools"].to_string().contains("ledger"));
    }
}
#[tokio::test]
async fn native_tool_search_acceptance_unadvertised_and_forged_calls_fail_closed() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    for (native, call) in [
        (false, search(true, json!({"query":"ledger"}))),
        (true, search(true, json!({"query":"ledger","tools":[READ]}))),
        (true, search(true, json!({"query":""}))),
        (true, {
            let mut c = search(true, json!({"query":"ledger"}));
            c["execution"] = json!("server");
            c
        }),
    ] {
        let server = ResponsesFixture::new(vec![vec![call]]);
        let _config = configure(&server.url, native).await;
        let model = provider::get_model("model-recovery", "original").await.unwrap();
        let info = provider::get_provider("model-recovery").await.unwrap().unwrap();
        let specs = HashMap::from([(
            "tool_search".into(),
            ToolSpec::new(
                "tool_search",
                "Find tools",
                json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}),
            ),
        )]);
        let mut calls = Vec::new();
        let result = super::do_stream_request_aisdk(
            &info,
            None,
            &HashMap::new(),
            &json!({}),
            &model,
            &json!([{"role":"user","content":"inspect ledger"}]),
            &specs,
            None,
            None,
            None,
            0,
            None,
            &mut |e| {
                if let StreamEvent::ToolCalls(found) = e {
                    calls.extend(found);
                }
            },
        )
        .await;
        assert!(result.is_err(), "malformed or unsupported call must fail");
        assert!(calls.is_empty());
        assert_eq!(server.requests().len(), 1);
    }
}

#[test]
fn native_tool_search_acceptance_restored_history_and_revocation_do_not_load_old_catalog() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _http = HttpMode::enable();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), CONTENT).unwrap();
    let server = ResponsesFixture::new(vec![
        vec![search(true, json!({"query":"ledger"}))],
        vec![finished()],
    ]);
    let _config = runtime.block_on(configure(&server.url, true));
    let card = Arc::new(Mutex::new(None));
    let captured = card.clone();
    crate::session::processor::run(desktop_request(workspace.path()), move |event| {
        if let crate::session::processor::StreamEvent::Tool(event) = event
            && event.payload.get("_vw_native_history").is_some()
        {
            *captured.lock().unwrap() = Some(format!("tool tool_search\n{}\n", event.payload));
        }
        true
    });
    assert_eq!(server.requests().len(), 2);
    // The restored card retains replay data. The current tool catalog grants nothing old.
    let server = ResponsesFixture::new(vec![vec![finished()]]);
    let _restored_config = runtime.block_on(configure(&server.url, true));
    let mut request = desktop_request(workspace.path());
    request.history.push(crate::session::ui_types::ChatMessage {
        role: crate::session::ui_types::ChatRole::Assistant,
        content: card.lock().unwrap().take().expect("native card persisted"),
        model: None,
        think_timing: vec![],
        processing_timing: None,
    });
    request.options["allowed_tools"] = json!(["tool_search"]);
    crate::session::processor::run(request, |_| true);
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0]["tools"].to_string().contains("ledger"));
    let input = requests[0]["input"].as_array().unwrap();
    assert_eq!(input.iter().filter(|i| i["type"] == "tool_search_call").count(), 1);
    assert_eq!(input.iter().filter(|i| i["type"] == "tool_search_output").count(), 1);

    // Revoke after search completion, before the next request snapshot is assembled.
    let server = ResponsesFixture::new(vec![
        vec![search(true, json!({"query":"ledger"}))],
        vec![read(true)],
        vec![finished()],
    ]);
    let _revoked_config = runtime.block_on(configure(&server.url, true));
    let request = desktop_request(workspace.path());
    let catalog = Arc::new(crate::tools::catalog::ToolCatalog::default());
    let tool: Arc<dyn Tool> = Arc::new(Ledger(workspace.path().join("ledger")));
    let discovery = crate::tools::discovery::ToolDiscoveryCatalog::new(vec![tool.spec()]);
    let search: Arc<dyn Tool> =
        Arc::new(crate::tools::tool_search::ToolSearchTool::new(&discovery));
    catalog
        .replace(vec![tool.clone(), search.clone()], [READ.into(), "tool_search".into()])
        .unwrap();
    let context = Arc::new(
        crate::tools::ToolUseContext::new(request.session.clone(), request.root.clone())
            .with_live_tool_catalog(catalog.clone()),
    );
    crate::tools::scope_tool_use_context_sync(context, || {
        crate::session::processor::run(request, move |event| {
            if matches!(
                event,
                crate::session::processor::StreamEvent::PostToolRound { step_index: 1 }
            ) {
                catalog
                    .replace(vec![tool.clone(), search.clone()], ["tool_search".into()])
                    .unwrap();
            }
            true
        });
    });
    let requests = server.requests();
    assert!(requests.len() >= 2);
    assert!(!workspace.path().join("ledger.invoked").exists());
    assert!(requests.iter().skip(1).all(|r| !r["tools"].to_string().contains("ledger")));
    assert!(!requests.iter().skip(1).any(|r| {
        r["input"].as_array().unwrap().iter().any(|i| {
            i["type"] == "function_call_output"
                && i["output"].as_str().is_some_and(|s| s.contains(CONTENT))
        })
    }));
}
