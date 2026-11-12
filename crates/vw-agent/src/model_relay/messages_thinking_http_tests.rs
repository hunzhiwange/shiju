//! Shared wire assertions for the lib test and the production-library integration target.
use serde_json::{Value, json};

fn request() -> Value {
    json!({"model":"selected","max_tokens":64,"messages":[{"role":"user","content":"hello"}]})
}

pub(super) async fn thinking_replays_through_bound_http() {
    use super::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};
    use axum::{Router, http::HeaderMap, routing::post};
    use std::{sync::Arc, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    struct ConfigGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
    impl Drop for ConfigGuard {
        fn drop(&mut self) {
            unsafe {
                crate::env::remove("SHIJU_PROTOCOL015_KEY");
                match self.1.take() {
                    Some(value) => {
                        crate::env::set(
                            "SHIJU_PROTOCOL015_KEY",
                            value.to_string_lossy().to_string(),
                        );
                        std::env::set_var("SHIJU_PROTOCOL015_KEY", value);
                    }
                    None => std::env::remove_var("SHIJU_PROTOCOL015_KEY"),
                }
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let _guard = ConfigGuard(
        std::env::var_os("VIBEWINDOW_CONFIG_DIR"),
        std::env::var_os("SHIJU_PROTOCOL015_KEY"),
    );
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
        std::env::set_var("SHIJU_PROTOCOL015_KEY", "fixture-only");
        crate::env::set("SHIJU_PROTOCOL015_KEY", "fixture-only");
    }
    let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let captured = received.clone();
    let router = Router::new().route("/v1/chat/completions", post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
        let streaming = body["stream"] == true;
        captured.lock().push((headers, body));
        async move {
            if streaming {
                use axum::response::IntoResponse;
                let chunks = [
                    json!({"id":"reply","choices":[{"index":0,"delta":{"reasoning_content":"wire "}}]}),
                    json!({"id":"reply","choices":[{"index":0,"delta":{"reasoning_content":"","content":"done"}}]}),
                    json!({"id":"reply","choices":[{"index":0,"delta":{"content":"","reasoning_content":"reasoning","tool_calls":[{"index":0,"id":"call-2","type":"function","function":{"name":"lookup","arguments":"{"}}]}}]}),
                    json!({"id":"reply","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}),
                    json!({"id":"reply","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3}}),
                ];
                let wire = chunks.iter().map(|v| format!("data: {v}\n\n")).collect::<String>() + "data: [DONE]\n\n";
                let bytes = wire.into_bytes().into_iter().map(|b| Ok::<_, std::io::Error>(axum::body::Bytes::from(vec![b])));
                return ([ ("content-type", "text/event-stream") ], axum::body::Body::from_stream(futures_util::stream::iter(bytes))).into_response();
            }
            use axum::response::IntoResponse;
            axum::Json(json!({"id":"reply","choices":[{"message":{"role":"assistant","content":"done","reasoning_content":"wire reasoning","tool_calls":[{"id":"call-2","type":"function","function":{"name":"lookup","arguments":"{}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":7,"completion_tokens":3}})).into_response() }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let stop = tokio_util::sync::CancellationToken::new();
    let stopping = stop.clone();
    let upstream = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await
            .unwrap();
    });
    let tenant = vw_api_types::id::TenantScope::new("protocol015", "wire").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"fixture015":{
        "api":format!("http://{address}/v1"),"adapter":"openai","env":["SHIJU_PROTOCOL015_KEY"],
        "models":{"selected":{"id":"gpt-5.4","status":"active","tool_call":true,"reasoning":true,"modalities":{"input":["text","image"],"output":["text"]}}}
    }}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let binding = RelayBinding::new(
        RelayIdentity {
            tenant: tenant.clone(),
            project: project.id,
            session: "task15".into(),
            provider: "fixture015".into(),
            model: "selected".into(),
            generation: 1,
        },
        Duration::from_secs(60),
    )
    .unwrap();
    let server = RelayServer::start(
        binding.clone(),
        Arc::new(ProviderRelayBackend::new().unwrap()),
        context,
    )
    .await
    .unwrap();
    let mut body = request();
    body["thinking"] = json!({"type":"adaptive"});
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("http://{}/v1/messages", server.address());
    let result =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(result.status(), 200);
    let result: Value = result.json().await.unwrap();
    assert_eq!(result["model"], "selected");
    assert_eq!(result["content"][0]["thinking"], "wire reasoning");
    assert!(result["content"][0]["signature"].as_str().unwrap().starts_with("shiju-bridge-v1:"));
    body["messages"] = json!([{"role":"user","content":"hello"},
        {"role":"assistant","content":result["content"]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"call-2","content":"lookup result"}]}]);
    let next =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(next.status(), 200);
    let next: Value = next.json().await.unwrap();
    assert_eq!(next["stop_reason"], "tool_use");
    {
        let calls = received.lock();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0["authorization"], "Bearer fixture-only");
        assert_eq!(calls[0].1["model"], "gpt-5.4");
        assert_eq!(calls[0].1["reasoning_effort"], "high");
        assert_eq!(calls[1].1["messages"][1]["reasoning_content"], "wire reasoning");
        assert_eq!(calls[1].1["messages"][2]["tool_call_id"], "call-2");
        assert_eq!(calls[1].1["messages"][2]["content"], "lookup result");
    }
    let mut tampered = body.clone();
    tampered["messages"][1]["content"][0]["thinking"] = "tampered".into();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&tampered)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(received.lock().len(), 2);
    body["stream"] = true.into();
    let streamed =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(streamed.status(), 200);
    assert_eq!(streamed.headers()["content-type"], "text/event-stream");
    let text = streamed.text().await.unwrap();
    assert!(
        text.contains("thinking_delta")
            && text.contains("wire reasoning")
            && text.contains("signature_delta")
    );
    assert!(text.contains("input_json_delta") && text.contains("message_stop"));
    assert_eq!(received.lock().len(), 3);
    assert_eq!(received.lock()[2].1["stream"], true);
    let events: Vec<Value> = text
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.first().unwrap()["type"], "message_start");
    assert_eq!(events.last().unwrap()["type"], "message_stop");
    let starts: Vec<_> = events
        .iter()
        .filter(|v| v["type"] == "content_block_start")
        .map(|v| v["content_block"]["type"].as_str().unwrap())
        .collect();
    assert_eq!(starts, ["thinking", "text", "tool_use"]);
    let usage = events.iter().find(|v| v["type"] == "message_delta").unwrap();
    assert_eq!(usage["usage"], json!({"input_tokens":7,"output_tokens":3}));
    assert_eq!(received.lock()[2].1["stream_options"]["include_usage"], true);
    let mut blocks = Vec::new();
    let mut arguments = std::collections::BTreeMap::<usize, String>::new();
    for event in events {
        let index = event["index"].as_u64().unwrap_or(0) as usize;
        match event["type"].as_str() {
            Some("content_block_start") => {
                assert_eq!(index, blocks.len());
                blocks.push(event["content_block"].clone());
            }
            Some("content_block_delta") => {
                let delta = &event["delta"];
                let field = match delta["type"].as_str().unwrap() {
                    "thinking_delta" => "thinking",
                    "signature_delta" => "signature",
                    "text_delta" => "text",
                    "input_json_delta" => {
                        arguments
                            .entry(index)
                            .or_default()
                            .push_str(delta["partial_json"].as_str().unwrap());
                        continue;
                    }
                    _ => panic!("unexpected delta"),
                };
                let mut value = blocks[index][field].as_str().unwrap_or("").to_owned();
                value.push_str(delta[field].as_str().unwrap());
                blocks[index][field] = value.into();
            }
            Some("content_block_stop") => {
                if let Some(arguments) = arguments.remove(&index) {
                    blocks[index]["input"] = serde_json::from_str(&arguments).unwrap();
                }
            }
            _ => {}
        }
    }
    body["messages"][1]["content"] = blocks.into();
    body["stream"] = false.into();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(received.lock().len(), 4);
    crate::config::update_project_for_tenant(
        &tenant,
        &root,
        json!({"providers":{"fixture015":{"models":{"selected":{"id":"gpt-5.4-mini"}}}}}),
    )
    .await
    .unwrap();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(received.lock().len(), 4);
    server.shutdown().await.unwrap();
    stop.cancel();
    upstream.await.unwrap();
}
