//! Gemini tool continuation must not inherit unrequested Chat reasoning defaults.
use axum::{Router, body::Body, http::HeaderMap, response::Response, routing::post};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use vw_agent::model_relay::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};

#[tokio::test]
async fn gemini_tool_continuation_keeps_selected_model_and_returns_text() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "gemini_chat_child", "--ignored", "--nocapture"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &root)
        .env("VIBEWINDOW_TEST_HOME", &root)
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .current_dir(&root)
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), child.output())
        .await
        .expect("Gemini relay fixture must finish within 30 seconds")
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "run by the parent with private storage and fixture credentials"]
async fn gemini_chat_child() {
    let home = std::env::current_dir().unwrap();
    assert!(vw_agent::global::paths().home.starts_with(&home));
    vw_agent::env::set("SHIJU_GEMINI_CHAT_TEST_KEY", "fixture-only");
    let received = Arc::new(parking_lot::Mutex::new(Vec::<Value>::new()));
    let captured = received.clone();
    let router = Router::new().route("/v1/chat/completions", post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
        let captured = captured.clone();
        async move {
            assert_eq!(headers["authorization"], "Bearer fixture-only");
            let reasoning = if body["enable_thinking"] == false { "" } else { "default upstream reasoning" };
            captured.lock().push(body.clone());
            if body["stream"] == true {
                let chunks = [
                    json!({"choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":reasoning},"finish_reason":null}]}),
                    json!({"choices":[{"index":0,"delta":{"content":"GEMINI_TOOL_OK"},"finish_reason":"stop"}]}),
                ];
                let wire = chunks.iter().map(|chunk| format!("data: {chunk}\n\n")).collect::<String>() + "data: [DONE]\n\n";
                Response::builder().header("content-type", "text/event-stream").body(Body::from(wire)).unwrap()
            } else {
                Response::builder().header("content-type", "application/json").body(Body::from(json!({"choices":[{"message":{"role":"assistant","content":"GEMINI_TOOL_OK","reasoning_content":reasoning},"finish_reason":"stop"}]}).to_string())).unwrap()
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let upstream = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let tenant = vw_api_types::id::TenantScope::new("gemini-chat", "wire").unwrap();
    let root = home.join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = vw_agent::project::from_directory(&tenant, &root).await.unwrap();
    vw_agent::config::update_project_for_tenant(&tenant, &root, json!({"providers":{"alibaba-token-plan-cn":{
        "api":format!("http://{address}/v1"),"adapter":"openai-compatible","env":["SHIJU_GEMINI_CHAT_TEST_KEY"],
        "models":{"selected":{"id":"deepseek-v4.1-flash-1017","status":"active","reasoning":true,"tool_call":true}}
    }}})).await.unwrap();
    let binding = RelayBinding::new(
        RelayIdentity {
            tenant,
            project: project.id.clone(),
            session: "tool-round:gemini".into(),
            provider: "alibaba-token-plan-cn".into(),
            model: "selected".into(),
            generation: 1,
        },
        Duration::from_secs(60),
    )
    .unwrap();
    let context = vw_agent::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let server = RelayServer::start(
        binding.clone(),
        Arc::new(ProviderRelayBackend::new().unwrap()),
        context,
    )
    .await
    .unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let body = json!({"contents":[
        {"role":"user","parts":[{"text":"list then reply"}]},
        {"role":"model","parts":[
            {"functionCall":{"id":"call-1","name":"list_directory","args":{"dir_path":"."}}},
            {"functionCall":{"id":"call-2","name":"read_file","args":{"path":"README.md"}}}
        ]},
        {"role":"user","parts":[
            {"functionResponse":{"id":"call-1","name":"list_directory","response":{"output":"fixture"}}},
            {"functionResponse":{"id":"call-2","name":"read_file","response":{"output":"fixture"}}},
            {"text":"Now write the requested files."}
        ]}
    ],"generationConfig":{}});
    for action in ["generateContent", "streamGenerateContent?alt=sse"] {
        let response = client
            .post(format!("http://{}/v1beta/models/selected:{action}", server.address()))
            .header("x-goog-api-key", binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let text = response.text().await.unwrap();
        assert!(text.contains("GEMINI_TOOL_OK"), "tool continuation must return text: {text}");
        assert!(!text.contains("default upstream reasoning"));
    }
    // Text cannot precede outstanding results without reordering the user's content.
    let mut interleaved = body.clone();
    interleaved["contents"][2]["parts"].as_array_mut().unwrap().swap(1, 2);
    let rejected = client
        .post(format!("http://{}/v1beta/models/selected:generateContent", server.address()))
        .header("x-goog-api-key", binding.bearer_token())
        .json(&interleaved)
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 422);
    let requests = received.lock().clone();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert_eq!(request["model"], "deepseek-v4.1-flash-1017");
        assert_eq!(request["enable_thinking"], false);
        assert_eq!(request["messages"][1]["tool_calls"][0]["id"], "call-1");
        assert_eq!(request["messages"][2]["tool_call_id"], "call-1");
        assert_eq!(request["messages"][3]["tool_call_id"], "call-2");
        assert_eq!(
            request["messages"][4],
            json!({"role":"user","content":"Now write the requested files."})
        );
    }
    server.shutdown().await.unwrap();
    upstream.abort();
}
