mod http {
    use super::super::*;
    use axum::{
        Router,
        body::Bytes,
        http::{HeaderMap, StatusCode},
        routing::post,
    };
    use futures_util::StreamExt;
    use serde_json::{Value, json};
    use std::{sync::Arc, time::Duration};
    use tokio_util::sync::CancellationToken;

    struct EnvGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                agent_env::remove("SHIJU_RELAY_MESSAGES_TEST_KEY");
                match self.1.take() {
                    Some(v) => {
                        agent_env::set(
                            "SHIJU_RELAY_MESSAGES_TEST_KEY",
                            v.to_string_lossy().to_string(),
                        );
                        std::env::set_var("SHIJU_RELAY_MESSAGES_TEST_KEY", v);
                    }
                    None => std::env::remove_var("SHIJU_RELAY_MESSAGES_TEST_KEY"),
                }
                match self.0.take() {
                    Some(v) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", v),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }

    #[tokio::test]
    async fn acp_model_source_relay_messages_http_stream_cancellation_errors_and_isolation() {
        let _lock = crate::test_support::global_async_env_lock().lock().await;
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvGuard(
            std::env::var_os("VIBEWINDOW_CONFIG_DIR"),
            std::env::var_os("SHIJU_RELAY_MESSAGES_TEST_KEY"),
        );
        unsafe {
            std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
            std::env::set_var("SHIJU_RELAY_MESSAGES_TEST_KEY", "bound-secret");
            agent_env::set("SHIJU_RELAY_MESSAGES_TEST_KEY", "bound-secret");
        }
        let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let captured = received.clone();
        let router = Router::new().route("/custom/v9/chat/completions", post(move |headers: HeaderMap, body: Bytes| {
        let captured = captured.clone();
        async move {
            let value: Value = serde_json::from_slice(&body).unwrap();
            captured.lock().push((headers, value.clone()));
            let prompt = value["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap_or("").to_owned();
            if prompt == "error" { return axum::response::Response::builder().status(500).body(axum::body::Body::from("private upstream error")).unwrap(); }
            if value["stream"] == true {
                let values = [
                    json!({"id":"msg-fixture","choices":[{"index":0,"delta":{"role":"assistant","content":"你"}}]}),
                    json!({"id":"msg-fixture","choices":[{"index":0,"delta":{"content":"好","tool_calls":[{"index":0,"id":"a","type":"function","function":{"name":"one","arguments":"{"}},{"index":1,"id":"b","type":"function","function":{"name":"two","arguments":"{}"}}]}}]}),
                    json!({"id":"msg-fixture","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}),
                    json!({"id":"msg-fixture","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3}}),
                ];
                let wire = values.iter().map(|v| format!("data: {v}\r\n\r\n")).collect::<String>() + "data: [DONE]\r\n\r\n";
                let chunks = wire.as_bytes().chunks(7).map(|c| Ok::<_, std::io::Error>(Bytes::copy_from_slice(c))).collect::<Vec<_>>();
                let stream = futures_util::stream::iter(chunks);
                let body = if prompt == "wait" { axum::body::Body::from_stream(stream.take(30).chain(futures_util::stream::pending())) } else { axum::body::Body::from_stream(stream) };
                axum::response::Response::builder().header("content-type", "text/event-stream").body(body).unwrap()
            } else {
                let reason = if prompt == "length" {"length"} else {"tool_calls"};
                axum::response::Response::builder().header("content-type", "application/json").body(axum::body::Body::from(json!({"id":"msg-fixture","choices":[{"message":{"role":"assistant","content":"hello","tool_calls":[{"id":"a","type":"function","function":{"name":"one","arguments":"{}"}}]},"finish_reason":reason}],"usage":{"prompt_tokens":7,"completion_tokens":3}}).to_string())).unwrap()
            }
        }
    }));
        let native_received = received.clone();
        let router = router.route("/custom/v9/messages",post(move |headers: HeaderMap, body: Bytes| {
            let native_received = native_received.clone();
            async move {
                let value: Value=serde_json::from_slice(&body).unwrap();
                native_received.lock().push((headers,value.clone()));
                let message=json!({"id":"native-id","type":"message","role":"assistant","model":"wire-model","content":[{"type":"thinking","thinking":"plan","signature":"signed-native"},{"type":"text","text":"done"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":2,"output_tokens":1},"vendor_extension":{"retained":true}});
                if value["stream"] == true {
                    let mut start = message.clone();
                    start["content"] = json!([]);
                    start["stop_reason"] = Value::Null;
                    start["usage"]["output_tokens"] = 0.into();
                    let events = [
                        json!({"type":"message_start","message":start}),
                        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}),
                        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"plan"}}),
                        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"signed-native"}}),
                        json!({"type":"content_block_stop","index":0}),
                        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
                        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"done"}}),
                        json!({"type":"content_block_stop","index":1}),
                        json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}),
                        json!({"type":"message_stop"}),
                    ];
                    let wire = events.iter().map(|event| format!("event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap())).collect::<String>();
                    axum::response::Response::builder().header("content-type","text/event-stream").body(axum::body::Body::from(wire)).unwrap()
                } else { axum::response::Response::builder().header("content-type","application/json").body(axum::body::Body::from(message.to_string())).unwrap() }
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
        let tenant = vw_api_transport::TenantScope::new("messages", "one").unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
        crate::config::update_project_for_tenant(&tenant, &root, json!({"providers":{"relay-messages":{"api":format!("http://{address}/custom/v9/chat/completions"),"env":["SHIJU_RELAY_MESSAGES_TEST_KEY"],"models":{"selected":{"id":"wire-model","status":"active","headers":{"x-provider":"trusted"},"tool_call":true}}}}})).await.unwrap();
        let context = crate::project::instance::Context {
            directory: root.to_string_lossy().into(),
            worktree: project.worktree.clone(),
            project: project.clone(),
        };
        let id = RelayIdentity {
            tenant: tenant.clone(),
            project: project.id,
            session: "one".into(),
            provider: "relay-messages".into(),
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
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        // The fixed Claude SDK beta resource uses this exact query, including
        // ordinary text/tool requests. It does not authorize routing overrides.
        let url = format!("http://{}/v1/messages?beta=true", server.address());
        for query in ["beta=false", "beta=true&model=other", "beta=true&beta=true"] {
            let rejected = client.post(format!("http://{}/v1/messages?{query}",server.address()))
                .bearer_auth(binding.bearer_token())
                .json(&json!({"model":"selected","max_tokens":16,"messages":[{"role":"user","content":"hello"}]}))
                .send().await.unwrap();
            assert_eq!(rejected.status(), 400);
        }
        let mut body = json!({"model":"selected","max_tokens":64,"messages":[{"role":"assistant","content":[{"type":"tool_use","id":"a","name":"one","input":{}}]},{"role":"user","content":[{"type":"tool_result","tool_use_id":"a","content":"result"}]}],"tools":[{"name":"one","input_schema":{"type":"object"}}]});
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["model"], "selected");
        assert_eq!(
            result["content"][1],
            json!({"type":"tool_use","id":"a","name":"one","input":{}})
        );
        assert_eq!(result["stop_reason"], "tool_use");
        assert_eq!(result["usage"], json!({"input_tokens":7,"output_tokens":3}));
        {
            let calls = received.lock();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].0["authorization"], "Bearer bound-secret");
            assert_eq!(calls[0].1["model"], "wire-model");
            assert_eq!(
                calls[0].1["messages"][1],
                json!({"role":"tool","tool_call_id":"a","content":"result"})
            );
        }
        body["stream"] = true.into();
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let wire = response.text().await.unwrap();
        let events = wire
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(|s| serde_json::from_str::<Value>(s).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(events.first().unwrap()["type"], "message_start");
        assert_eq!(events[0]["message"]["model"], "selected");
        assert_eq!(events.last().unwrap()["type"], "message_stop");
        assert_eq!(events.iter().filter(|e| e["type"] == "content_block_start").count(), 3);
        let text = events
            .iter()
            .filter(|e| e["delta"]["type"] == "text_delta")
            .map(|e| e["delta"]["text"].as_str().unwrap())
            .collect::<String>();
        assert_eq!(text, "你好");
        let delta = &events[events.len() - 2];
        assert_eq!(delta["delta"]["stop_reason"], "tool_use");
        assert_eq!(delta["usage"], json!({"input_tokens":7,"output_tokens":3}));
        for index in 0..3 {
            let start = events
                .iter()
                .position(|e| e["type"] == "content_block_start" && e["index"] == index)
                .unwrap();
            let end = events
                .iter()
                .position(|e| e["type"] == "content_block_stop" && e["index"] == index)
                .unwrap();
            assert!(start < end);
        }
        for (field, value, status) in [
            ("model", json!("other"), 403),
            ("thinking", json!({"type":"enabled"}), 422),
            ("base_url", json!("http://127.0.0.1:1"), 400),
        ] {
            let mut bad = body.clone();
            bad[field] = value;
            let response = client
                .post(&url)
                .bearer_auth(binding.bearer_token())
                .json(&bad)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status);
            assert_eq!(response.json::<Value>().await.unwrap()["type"], "error");
        }
        assert_eq!(received.lock().len(), 2);
        // Capability from another tenant/session cannot be replayed against this listener.
        let mut other = binding.identity().clone();
        other.tenant = vw_api_transport::TenantScope::new("other", "tenant").unwrap();
        let other = RelayBinding::new(other, Duration::from_secs(60)).unwrap();
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(other.bearer_token())
                .json(&body)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        body["messages"] = json!([{"role":"user","content":"error"}]);
        body["stream"] = false.into();
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 502);
        let error = response.text().await.unwrap();
        assert!(error.contains("api_error"));
        assert!(!error.contains("private"));
        body["messages"] = json!([{"role":"user","content":"length"}]);
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.json::<Value>().await.unwrap()["stop_reason"], "max_tokens");
        // A subprocess probes the real relay socket; no supplier CLI support is claimed here.
        use tokio::io::AsyncWriteExt;
        let mut child=tokio::process::Command::new("python3").arg("-c").arg("import sys,json,urllib.request; v=json.load(sys.stdin); r=urllib.request.urlopen(urllib.request.Request(v['url'],data=json.dumps(v['body']).encode(),headers={'Authorization':'Bearer '+v['token'],'Content-Type':'application/json'})); print(json.load(r)['model'])").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                json!({"url":url,"token":binding.bearer_token(),"body":body})
                    .to_string()
                    .as_bytes(),
            )
            .await
            .unwrap();
        let output = child.wait_with_output().await.unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "selected");
        crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-messages":{"api":format!("http://{address}/custom/v9"),"adapter":"anthropic","models":{"selected":{"status":"active","reasoning":true}}}}})).await.unwrap();
        body["messages"] = json!([{"role":"assistant","content":[{"type":"thinking","thinking":"plan","signature":"signed-native"},{"type":"text","text":"done","cache_control":{"type":"ephemeral"}}]}]);
        body["metadata"] = json!({"user_id":"fixture"});
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let native: Value = response.json().await.unwrap();
        assert_eq!(native["model"], "selected");
        assert_eq!(native["content"][0]["signature"], "signed-native");
        assert_eq!(native["vendor_extension"]["retained"], true);
        {
            let captured = received.lock();
            let (headers, value) = captured.last().unwrap();
            assert_eq!(headers["x-api-key"], "bound-secret");
            assert!(!headers.contains_key("authorization"));
            assert_eq!(value["metadata"], body["metadata"]);
            assert_eq!(value["messages"], body["messages"]);
        }
        body["stream"] = true.into();
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        let native_wire = response.text().await.unwrap();
        assert!(native_wire.contains("signed-native"));
        assert!(native_wire.contains("selected"));
        assert!(!native_wire.contains("wire-model"));
        assert!(native_wire.contains("message_stop"));
        body.as_object_mut().unwrap().remove("metadata");
        // Restore the nonthinking fixture after exercising native signed thinking.
        crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-messages":{"api":format!("http://{address}/custom/v9/chat/completions"),"adapter":"openai","models":{"selected":{"reasoning":false}}}}})).await.unwrap();
        body["messages"] = json!([{"role":"user","content":"wait"}]);
        body["stream"] = true.into();
        let response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let mut stream = response.bytes_stream();
        let first = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("converted stream must start before revocation")
            .expect("revocation fixture ended before its first frame")
            .unwrap();
        assert!(first.starts_with(b"event: message_start"));
        binding.revoke();
        tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(chunk) = stream.next().await {
                if chunk.is_err() {
                    break;
                }
            }
        })
        .await
        .unwrap();
        drop(stream);
        server.shutdown().await.unwrap();
        stop.cancel();
        upstream.await.unwrap();
    }
}
