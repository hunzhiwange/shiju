use super::*;
use axum::{
    Router,
    body::Bytes,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

struct EnvGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            crate::env::remove("SHIJU_RELAY_CHAT_TEST_KEY");
            match self.1.take() {
                Some(v) => {
                    crate::env::set("SHIJU_RELAY_CHAT_TEST_KEY", v.to_string_lossy().to_string());
                    std::env::set_var("SHIJU_RELAY_CHAT_TEST_KEY", v);
                }
                None => std::env::remove_var("SHIJU_RELAY_CHAT_TEST_KEY"),
            }
            match self.0.take() {
                Some(v) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", v),
                None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
            }
        }
    }
}

#[tokio::test]
async fn acp_model_source_relay_chat_roundtrip_stream_tools_and_account_isolation() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(
        std::env::var_os("VIBEWINDOW_CONFIG_DIR"),
        std::env::var_os("SHIJU_RELAY_CHAT_TEST_KEY"),
    );
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
        std::env::set_var("SHIJU_RELAY_CHAT_TEST_KEY", "bound-secret");
        crate::env::set("SHIJU_RELAY_CHAT_TEST_KEY", "bound-secret");
    }
    let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let captured = received.clone();
    let wire = concat!(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"a\",\"function\":{\"name\":\"one\",\"arguments\":\"{\"}},{\"index\":1,\"id\":\"b\",\"function\":{\"name\":\"two\",\"arguments\":\"{}\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3,\"total_tokens\":10}}\n\ndata: [DONE]\n\n"
    );
    let router = Router::new().route("/custom/v9/chat/completions", post(move |headers: HeaderMap, body: Bytes| {
        let captured = captured.clone();
        async move {
            let value: Value = serde_json::from_slice(&body).unwrap();
            captured.lock().push((headers, value.clone()));
            if value["fixture_error"] == true {
                return axum::response::Response::builder().status(500).body(axum::body::Body::from("upstream-private-error")).unwrap();
            }
            if let Some(status) = value["fixture_status"].as_u64() {
                return axum::response::Response::builder()
                    .status(u16::try_from(status).unwrap())
                    .header("x-upstream-secret", "private-header")
                    .header("location", "https://example.invalid/private")
                    .body(axum::body::Body::from("upstream-private-error"))
                    .unwrap();
            }
            if value["stream"] == true {
                let chunks = futures_util::stream::iter(wire.as_bytes().chunks(11).map(|c| Ok::<_, std::io::Error>(Bytes::copy_from_slice(c))).collect::<Vec<_>>());
                axum::response::Response::builder().header("content-type", "text/event-stream").body(axum::body::Body::from_stream(chunks)).unwrap()
            } else {
                axum::response::Response::builder().header("content-type", "application/json").body(axum::body::Body::from(r#"{"choices":[{"message":{"role":"assistant","tool_calls":[{"id":"a","type":"function","function":{"name":"one","arguments":"{}"}}]},"finish_reason":"tool_calls"}],"usage":{"total_tokens":10}}"#)).unwrap()
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let stop = CancellationToken::new();
    let stopping = stop.clone();
    let upstream = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await
            .unwrap();
    });
    let tenant = vw_api_types::id::TenantScope::new("chat", "one").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, &root, json!({"providers":{"relay-chat":{"api":format!("http://{address}/custom/v9/chat/completions"),"env":["SHIJU_RELAY_CHAT_TEST_KEY"],"models":{"selected":{"id":"wire-model","status":"active","headers":{"x-provider":"trusted"},"tool_call":true}}}}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let id = RelayIdentity {
        tenant: tenant.clone(),
        project: project.id,
        session: "one".into(),
        provider: "relay-chat".into(),
        model: "selected".into(),
        generation: 1,
    };
    let binding = RelayBinding::new(id, Duration::from_secs(60)).unwrap();
    let server = RelayServer::start(
        binding.clone(),
        Arc::new(ProviderRelayBackend::new().unwrap()),
        context,
    )
    .await
    .unwrap();
    let url = format!("http://{}/v1/chat/completions", server.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut body = json!({"model":"selected","messages":[{"role":"assistant","content":null,"tool_calls":[{"id":"a","type":"function","function":{"name":"one","arguments":"{}"}}]},{"role":"tool","tool_call_id":"a","content":"result"}],"tools":[{"type":"function","function":{"name":"one","parameters":{"type":"object"}}}],"parallel_tool_calls":true});
    let response = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .header("x-provider", "evil")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: Value = response.json().await.unwrap();
    assert_eq!(result["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(result["usage"]["total_tokens"], 10);
    body["stream"] = true.into();
    let response =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), wire);
    {
        let calls = received.lock();
        assert_eq!(calls.len(), 2);
        for (headers, value) in calls.iter() {
            assert_eq!(headers["authorization"], "Bearer bound-secret");
            assert_eq!(headers["x-provider"], "trusted");
            assert_eq!(value["model"], "wire-model");
            assert_eq!(value["messages"], body["messages"]);
            assert_eq!(value["tools"], body["tools"]);
            assert_eq!(value["parallel_tool_calls"], true);
        }
    }
    body["model"] = "auxiliary-model".into();
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
    assert_eq!(received.lock().len(), 2);
    body["model"] = "selected".into();
    // Auxiliary calls may retain vendor semantics but cannot change account or capabilities.
    for (field, value, expected) in [
        ("tools", json!([{"type":"web_search"}]), 422),
        ("audio", json!({"voice":"alloy"}), 422),
        ("base_url", json!("http://127.0.0.1:1"), 400),
        ("max_tokens", json!(u64::MAX), 422),
    ] {
        let mut bad = body.clone();
        bad[field] = value;
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(binding.bearer_token())
                .json(&bad)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            expected
        );
    }
    crate::config::update_project_for_tenant(&tenant, &root, json!({"providers":{"relay-chat":{"models":{"selected":{"status":"active","tool_call":false}}}}})).await.unwrap();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    crate::config::update_project_for_tenant(&tenant, &root, json!({"providers":{"relay-chat":{"models":{"selected":{"status":"active","tool_call":true}}}}})).await.unwrap();
    unsafe {
        std::env::remove_var("SHIJU_RELAY_CHAT_TEST_KEY");
        crate::env::remove("SHIJU_RELAY_CHAT_TEST_KEY");
    }
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(received.lock().len(), 2);
    unsafe {
        std::env::set_var("SHIJU_RELAY_CHAT_TEST_KEY", "bound-secret");
        crate::env::set("SHIJU_RELAY_CHAT_TEST_KEY", "bound-secret");
    }
    // A real subprocess crosses both HTTP boundaries; this is a transport probe, not a CLI support claim.
    use tokio::io::AsyncWriteExt;
    let mut child = tokio::process::Command::new("python3").arg("-c").arg("import sys,json,urllib.request; v=json.load(sys.stdin); r=urllib.request.urlopen(urllib.request.Request(v['url'], data=json.dumps(v['body']).encode(), headers={'Authorization':'Bearer '+v['token'],'Content-Type':'application/json'})); print(json.load(r)['usage']['total_tokens'])")
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
    body.as_object_mut().unwrap().remove("stream");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            serde_json::to_string(&json!({"url":url,"token":binding.bearer_token(),"body":body}))
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();
    let output = child.wait_with_output().await.unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "10");
    assert_eq!(received.lock().len(), 3);
    body["fixture_error"] = true.into();
    let failed =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(failed.status(), 502);
    assert!(!failed.text().await.unwrap().contains("upstream-private-error"));
    assert_eq!(received.lock().len(), 4, "relay must not retry or duplicate usage");

    body.as_object_mut().unwrap().remove("fixture_error");
    for (index, (upstream_status, expected)) in
        [(403, 403), (401, 401), (400, 400), (429, 429), (307, 502), (503, 502)]
            .into_iter()
            .enumerate()
    {
        body["fixture_status"] = json!(upstream_status);
        let failed =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(failed.status().as_u16(), expected, "upstream status {upstream_status}");
        assert!(failed.headers().get("x-upstream-secret").is_none());
        assert!(failed.headers().get("location").is_none());
        assert!(failed.text().await.unwrap().is_empty());
        assert_eq!(received.lock().len(), 5 + index, "relay must not retry upstream errors");
    }

    server.shutdown().await.unwrap();
    stop.cancel();
    upstream.await.unwrap();
}
