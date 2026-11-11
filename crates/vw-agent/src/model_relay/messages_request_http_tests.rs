//! Shared wire assertions for the lib test and the production-library integration target.
use serde_json::{Value, json};

fn request() -> Value {
    json!({"model":"selected","max_tokens":64,"messages":[{"role":"user","content":"hello"}]})
}

pub(super) async fn request_reaches_bound_chat_and_finishes_tool_round() {
    use super::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};
    use axum::{Router, http::HeaderMap, routing::post};
    use std::{sync::Arc, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    struct ConfigGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
    impl Drop for ConfigGuard {
        fn drop(&mut self) {
            unsafe {
                crate::env::remove("SHIJU_PROTOCOL014_KEY");
                match self.1.take() {
                    Some(value) => {
                        crate::env::set(
                            "SHIJU_PROTOCOL014_KEY",
                            value.to_string_lossy().to_string(),
                        );
                        std::env::set_var("SHIJU_PROTOCOL014_KEY", value);
                    }
                    None => std::env::remove_var("SHIJU_PROTOCOL014_KEY"),
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
        std::env::var_os("SHIJU_PROTOCOL014_KEY"),
    );
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
        std::env::set_var("SHIJU_PROTOCOL014_KEY", "fixture-only");
        crate::env::set("SHIJU_PROTOCOL014_KEY", "fixture-only");
    }
    let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let captured = received.clone();
    let router = Router::new().route("/v1/chat/completions", post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
        captured.lock().push((headers, body));
        async { axum::Json(json!({"id":"reply","choices":[{"message":{"role":"assistant","content":"done","tool_calls":[{"id":"call-2","type":"function","function":{"name":"lookup","arguments":"{}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":7,"completion_tokens":3}})) }
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
    let tenant = vw_api_types::id::TenantScope::new("protocol014", "wire").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"fixture014":{
        "api":format!("http://{address}/v1"),"adapter":"openai","env":["SHIJU_PROTOCOL014_KEY"],
        "models":{"selected":{"id":"wire-model","status":"active","tool_call":true,"modalities":{"input":["text","image"],"output":["text"]}}}
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
            session: "task14".into(),
            provider: "fixture014".into(),
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
    body["stop_sequences"] = json!(["END"]);
    body["cache_control"] = json!({"type":"ephemeral"});
    body["output_config"] = json!({"format":{"type":"json_schema","schema":{"type":"object","properties":{},"additionalProperties":false}}});
    body["tools"] = json!([{"name":"lookup","input_schema":{"type":"object","properties":{},"required":[],"additionalProperties":false},"strict":true}]);
    body["tool_choice"] = json!({"type":"any","disable_parallel_tool_use":true});
    body["messages"] = json!([
        {"role":"assistant","content":[{"type":"tool_use","id":"call-1","name":"lookup","input":{}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"call-1","content":"result"}]},
        {"role":"user","content":[{"type":"text","text":"look"},{"type":"image","source":{"type":"url","url":"https://example.invalid/image.png"}}]}
    ]);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("http://{}/v1/messages", server.address());
    let response =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let response: Value = response.json().await.unwrap();
    assert_eq!(response["model"], "selected");
    assert_eq!(response["stop_reason"], "tool_use");
    assert_eq!(response["content"][1]["id"], "call-2");
    assert_eq!(response["usage"], json!({"input_tokens":7,"output_tokens":3}));
    {
        let calls = received.lock();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0["authorization"], "Bearer fixture-only");
        let wire = &calls[0].1;
        assert_eq!(wire["model"], "wire-model");
        assert_eq!(wire["stop"], body["stop_sequences"]);
        assert_eq!(
            wire["response_format"]["json_schema"]["schema"],
            body["output_config"]["format"]["schema"]
        );
        assert_eq!(wire["tools"][0]["function"]["strict"], true);
        assert_eq!(wire["tool_choice"], "required");
        assert_eq!(wire["parallel_tool_calls"], false);
        assert_eq!(wire["messages"][1]["tool_call_id"], "call-1");
        assert_eq!(
            wire["messages"][2]["content"][1]["image_url"]["url"],
            "https://example.invalid/image.png"
        );
        assert!(wire.get("cache_control").is_none());
    }
    body["thinking"] = json!({"type":"enabled","budget_tokens":1024});
    let rejected =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(rejected.status(), 422);
    assert_eq!(received.lock().len(), 1);
    body.as_object_mut().unwrap().remove("thinking");
    crate::config::update_project_for_tenant(
        &tenant,
        &root,
        json!({"providers":{"fixture014":{
            "models":{"selected":{"modalities":{"input":["text"],"output":["text"]}}}
        }}}),
    )
    .await
    .unwrap();
    let rejected =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(rejected.status(), 422);
    assert_eq!(received.lock().len(), 1);
    server.shutdown().await.unwrap();
    stop.cancel();
    upstream.await.unwrap();
}
