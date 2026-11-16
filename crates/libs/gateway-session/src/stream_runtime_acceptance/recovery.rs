use super::*;

pub async fn gateway_first_response_failure_preserves_history_until_same_session_retries<H: RuntimeAcceptanceHost>() {

    use llm_runtime::StreamEvent;
    use session_store::ui_store;

    let _env = H::env_lock().lock().await;
    struct RestoreTitleFixtureKey(Option<String>);
    impl Drop for RestoreTitleFixtureKey {
        fn drop(&mut self) {
            match self.0.take() {
                Some(key) => agent_env::set("VW_MODEL_RECOVERY_TEST_KEY", key),
                None => agent_env::remove("VW_MODEL_RECOVERY_TEST_KEY"),
            }
        }
    }
    let _key = RestoreTitleFixtureKey(agent_env::get("VW_MODEL_RECOVERY_TEST_KEY"));
    agent_env::set("VW_MODEL_RECOVERY_TEST_KEY", "fixture-key");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let (title_tx, mut title_requests) = tokio::sync::mpsc::unbounded_channel();
    let provider = axum::Router::new()
        .route(
            "/v1/models",
            axum::routing::get(|| async {
                Json(serde_json::json!({"data": [{"id": "original"}]}))
            }),
        )
        .fallback(move |Json(request): Json<Value>| {
            let title_tx = title_tx.clone();
            async move {
                let _ = title_tx.send(request);
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": {
                        "message": "title provider rejected the request"
                    }})),
                )
            }
        });
    let provider = tokio::spawn(async move { axum::serve(listener, provider).await });
    let _fixture = H::recovery(
        &endpoint,
        Some("fixture-key"),
    )
    .await;
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    let tenant = ui_store::current_tenant_scope();
    let scope =
        ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None).unwrap();
    let session_id = format!("ses_retry_{}", uuid::Uuid::new_v4());
    let request = |content: &str| GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role": "user", "content": content})],
        model: Some("model-recovery/original".into()),
        ..Default::default()
    };
    let query = || Query(InstanceQuery { directory: Some(directory.clone()) });
    H::enqueue(vec![StreamEvent::Error(
        session_engine::message::AssistantError::Unknown { message: "fixture failure".into() },
    )]);
    let response = chat_stream::<H>(query(), HeaderMap::new(), Json(request("failed prompt")))
        .await
        .unwrap()
        .into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(std::str::from_utf8(&body).unwrap().contains("chat.error"));
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(info.is_first_response_pending());
    assert!(info.title.is_empty());
    assert!(
        ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
            .unwrap()
            .is_some()
    );

    let failed = ui_store::load_session_scoped_for_tenant(&tenant, &session_id, Some(&scope))
        .expect("failed turn remains readable");
    assert!(failed.messages.iter().any(|message| message.content == "failed prompt"));
    assert!(failed.messages.iter().any(|message| message.content.contains("fixture failure")));

    assert!(title_requests.try_recv().is_err());

    H::enqueue(vec![
        StreamEvent::Delta("successful answer".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
    let response = chat_stream::<H>(query(), HeaderMap::new(), Json(request("successful retry")))
        .await
        .unwrap()
        .into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(std::str::from_utf8(&body).unwrap().contains("chat.done"), "{body:?}");
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(!info.is_first_response_pending());
    assert!(info.title.is_empty());
    let preview = ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
        .unwrap()
        .unwrap();
    assert_eq!(preview.last_content.as_deref(), Some("successful retry"));
    let title_request =
        tokio::time::timeout(std::time::Duration::from_secs(10), title_requests.recv())
            .await
            .expect("the first committed turn should trigger title generation")
            .unwrap();
    let title_prompt = title_request.to_string();
    assert!(title_prompt.contains("successful retry"));
    assert!(!title_prompt.contains("failed prompt"));
    assert!(title_request["tools"].as_array().is_none_or(Vec::is_empty));
    provider.abort();
    let _ = provider.await;
}

pub async fn gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing<H: RuntimeAcceptanceHost>() {

    use llm_runtime::StreamEvent;
    use model_types::ToolCall;
    use session_store::ui_store;
    use futures_util::StreamExt as _;

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "http://127.0.0.1:1/v1",
        None,
    )
    .await;
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    H::update_config(serde_json::json!({
        "agents": {
            "main": {
                "provider": "model-recovery", "model": "original", "allowed_tools": ["bash"]
            }
        }
    }))
    .await
    .unwrap();
    H::enqueue(vec![
        StreamEvent::ToolCalls(vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "first-response-tool".into(),
            name: "bash".into(),
            arguments: serde_json::json!({
                "command": "sleep 30",
                "description": "wait for stream cancellation",
                "yield_time_ms": 10000
            })
            .to_string(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);

    let request_id = uuid::Uuid::new_v4().to_string();
    let mut headers = HeaderMap::new();
    headers.insert(STREAM_REQUEST_ID_HEADER, request_id.parse().unwrap());
    let request = GatewayChatStreamRequest {
        messages: vec![serde_json::json!({"role": "user", "content": "Run a tool"})],
        model: Some("model-recovery/original".into()),
        allowed_tools: Some(vec!["bash".into()]),
        options: Some(serde_json::json!({"full_access": true})),
        ..Default::default()
    };
    let query = || Query(InstanceQuery { directory: Some(directory.clone()) });
    let response =
        chat_stream::<H>(query(), headers.clone(), Json(request.clone())).await.unwrap().into_response();
    let session_id = response.headers()["x-vw-session-id"].to_str().unwrap().to_string();
    let mut stream = response.into_body().into_data_stream();
    let cursor = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let mut body = String::new();
        while let Some(chunk) = stream.next().await {
            body.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
            if let Some(event) = body.split("\n\n").find(|event| {
                event.contains(r#""type":"chat.tool""#) && event.contains(r#""kind":"start""#)
            }) {
                return event
                    .lines()
                    .find_map(|line| line.strip_prefix("id:").map(str::trim))
                    .unwrap()
                    .to_string();
            }
        }
        panic!("tool start not observed: {body}");
    })
    .await
    .unwrap();
    drop(stream);

    let tenant = ui_store::current_tenant_scope();
    let Json(stop) =
        chat_stream_stop(Extension(tenant.clone()), axum::extract::Path(request_id)).await;
    assert_eq!(stop["cancelled"], true);
    headers.insert("last-event-id", cursor.parse().unwrap());
    let resumed = chat_stream::<H>(query(), headers, Json(request)).await.unwrap().into_response();
    let body = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        axum::body::to_bytes(resumed.into_body(), usize::MAX),
    )
    .await
    .unwrap()
    .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    assert!(body.contains(r#""error":"cancelled""#), "{body}");
    assert!(!body.contains(r#""type":"chat.done""#), "{body}");
    let scope =
        ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None).unwrap();
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(info.is_first_response_pending());
    assert!(info.title.is_empty());
    assert!(
        ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
            .unwrap()
            .is_some()
    );
}

pub async fn agent_native_defaults_do_not_grant_disabled_mcp<H: RuntimeAcceptanceHost>() {

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join(".vwacprc.json"), serde_json::to_vec(&serde_json::json!({
        "mcpServers":[{"name":"disabled-server", "command":"missing-disabled-command", "enabled":false}]
    })).unwrap()).unwrap();
    H::update_config(serde_json::json!({
        "agents": {"main": {"defaults": {"mcp": {"mode":"fixed", "value":["disabled-server"]}}}}
    }))
    .await
    .unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
    let directory = workspace.path().to_string_lossy().to_string();
    let result = H::with_tenant_instance(tenant, directory, || {
        Box::pin(async {
            crate::stream::resolve_delegate_request_overrides::<H>(Some("main".into()), None, None).await
        })
    })
    .await;
    assert!(result.is_err(), "a missing or disabled MCP default must reject admission");
}

pub async fn agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice<H: RuntimeAcceptanceHost>() {

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    H::update_config(serde_json::json!({"agents":{"main":{"defaults":{"full_access":true}}}}))
        .await
        .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
    H::with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), || {
        Box::pin(async {
            let mut request = GatewayChatStreamRequest::default();
            crate::stream::prepare_gateway_agent_request::<H>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], true);
            request.options = Some(serde_json::json!({"full_access":false}));
            crate::stream::prepare_gateway_agent_request::<H>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
            let mut untouched = GatewayChatStreamRequest::default();
            crate::stream::prepare_gateway_agent_request::<H>(&mut untouched, None).await.unwrap();
            H::update_config(
                serde_json::json!({"agents":{"main":{"defaults":{"full_access":false}}}}),
            )
            .await
            .unwrap();
            request.options = None;
            crate::stream::prepare_gateway_agent_request::<H>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
            untouched.options = None;
            crate::stream::prepare_gateway_agent_request::<H>(&mut untouched, None).await.unwrap();
            assert_eq!(untouched.options.as_ref().unwrap()["full_access"], true);
            let mut fresh = GatewayChatStreamRequest::default();
            crate::stream::prepare_gateway_agent_request::<H>(&mut fresh, None).await.unwrap();
            assert_eq!(fresh.options.as_ref().unwrap()["full_access"], false);
            Ok(())
        })
    })
    .await
    .unwrap();
}

pub async fn gateway_chat_git_line_counts<H: RuntimeAcceptanceHost>(initialize: impl FnOnce(&std::path::Path)) {
    use llm_runtime::StreamEvent;
    use model_types::ToolCall;
    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap();
    let file = directory.join("example.txt");
    std::fs::write(&file, "keep\nold\n").unwrap();
    initialize(&directory);
    H::update_config(serde_json::json!({
        "agents": {
            "main": {
                "provider": "model-recovery", "model": "original",
                "allowed_tools": ["apply_patch"]
            }
        }
    }))
    .await
    .unwrap();

    H::enqueue(vec![
        StreamEvent::ToolCalls(vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "edit-example".into(),
            name: "apply_patch".into(),
            arguments: serde_json::json!({
                "patchText": "*** Begin Patch\n*** Update File: example.txt\n@@\n keep\n-old\n+new\n+extra\n*** End Patch"
            })
            .to_string(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);
    H::enqueue(vec![
        StreamEvent::Delta("updated the example".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);

    let session_id = format!("ses_git_stats_{}", uuid::Uuid::new_v4());
    let request = GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role":"user","content":"Update the example text."})],
        model: Some("model-recovery/original".into()),
        agent: Some("main".into()),
        allowed_tools: Some(vec!["apply_patch".into()]),
        options: Some(serde_json::json!({"full_access":true})),
        ..Default::default()
    };
    let directory = directory.to_string_lossy().into_owned();
    let mut reported = None;
    let mut reported_during_tools = false;
    tokio::time::timeout(
        std::time::Duration::from_secs(20),
        collect_gateway_chat_stream_with_observer::<H>(directory.clone(), request, |event| {
            if event["type"] == "chat.session_updated" {
                assert_eq!(event["session_id"], session_id);
                reported = Some(event["session"]["summary"].clone());
            }
            if event["type"] == "chat.post_tool_round" {
                let summary =
                    reported.as_ref().expect("counts must arrive before the next model round");
                assert_eq!(summary["additions"], 2);
                assert_eq!(summary["deletions"], 1);
                reported_during_tools = true;
            }
        }),
    )
    .await
    .expect("stream should finish")
    .unwrap();

    assert!(reported_during_tools);
    assert_eq!(std::fs::read_to_string(file).unwrap(), "keep\nnew\nextra\n");
    let reported = reported.expect("stream should report the persisted summary");
    assert_eq!(
        (reported["additions"].as_i64(), reported["deletions"].as_i64()),
        (Some(2), Some(1))
    );

    let tenant = session_store::ui_store::current_tenant_scope();
    let scope = session_store::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .unwrap();
    let stored = session_store::ui_store::load_agent_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .unwrap()
    .summary
    .expect("persisted summary");
    assert_eq!((stored.additions, stored.deletions, stored.files), (2, 1, 1));
}
