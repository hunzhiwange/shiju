//! Domain streaming acceptance assertions exercised with the actual native StreamHost.
//! Environment locks, model fixtures and native wiring remain in the host tests.
use crate::stream::*;
use axum::{Json, extract::Query, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use gateway_instance::host::InstanceQuery;
use gateway_resumable_stream::STREAM_REQUEST_ID_HEADER;
use serde_json::Value;
use session_store::ui_store;
use vw_api_chat::chat::GatewayChatStreamRequest;

pub fn docs_stream_request(session_id: Option<&str>) -> GatewayChatStreamRequest {
    GatewayChatStreamRequest {
        session_id: session_id.map(vw_api_transport::SessionId::from),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "请列出 docs 文件列表"
        })],
        // Resolve a registered model normally before entering the local docs branch.
        model: None,
        ..GatewayChatStreamRequest::default()
    }
}

pub fn make_docs_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let docs = temp.path().join("docs");
    std::fs::create_dir(&docs).expect("docs directory");
    std::fs::write(docs.join("guide.md"), "# Guide\n").expect("docs file");
    temp
}

pub async fn chat_stream_rejects_missing_workspace_without_retryable_status<H: crate::stream_host::StreamHost>() {
    let workspace = tempfile::tempdir().unwrap();
    let missing = workspace.path().join("deleted-worktree");
    let result = chat_stream::<H>(
        Query(InstanceQuery { directory: Some(missing.to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(docs_stream_request(None)),
    )
    .await;

    let error = result.err().expect("missing workspace must reject the request");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.message.contains("无法访问会话工作目录"));
}

pub async fn collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids<H: crate::stream_host::StreamHost>() {
    let temp = make_docs_workspace();
    let session_id = format!("ses_stream_docs_{}", uuid::Uuid::new_v4());
    let request = docs_stream_request(Some(&session_id));
    let mut seen_types = Vec::new();

    let collected = collect_gateway_chat_stream_with_observer::<H>(
        temp.path().to_string_lossy().to_string(),
        request,
        |payload| {
            if let Some(kind) = payload.get("type").and_then(Value::as_str) {
                seen_types.push(kind.to_string());
            }
        },
    )
    .await
    .expect("docs stream should complete");

    assert!(collected.output.contains("docs 目录下的文件"));
    assert!(collected.output.contains("docs/guide.md"));
    assert!(collected.message_id.as_deref().is_some_and(|id| id.starts_with("msg_")));
    assert!(collected.parent_message_id.as_deref().is_some_and(|id| id.starts_with("msg_")));
    assert_eq!(seen_types, vec!["chat.delta", "chat.done"]);

    let stored = session_engine::message::messages(&session_id, None)
        .await
        .expect("persisted messages should load");
    assert_eq!(stored.len(), 2);
}

pub async fn gateway_first_turn_registers_parent_before_thread_tool_use<H: crate::stream_host::StreamHost>() {
    let temp = make_docs_workspace();
    let directory = temp.path().to_string_lossy().to_string();
    let session_id = format!("ses_stream_parent_{}", uuid::Uuid::new_v4());
    let request = docs_stream_request(Some(&session_id));
    let tenant = session_store::ui_store::current_tenant_scope();
    let scope = session_store::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .expect("project scope");

    assert!(
        session_store::ui_store::load_agent_session_scoped_for_tenant(
            &tenant,
            &session_id,
            Some(&scope),
        )
        .is_none()
    );

    let tenant_for_stream = tenant.clone();
    let directory_for_stream = directory.clone();
    let receiver = H::with_tenant_instance(tenant.clone(), directory.clone(), move || {
        Box::pin(async move {
            start_gateway_chat_stream::<H>(tenant_for_stream, directory_for_stream, request, None, None)
                .await
        })
    })
    .await
    .expect("first gateway stream should start");

    let parent = session_store::ui_store::load_agent_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .expect("parent session must exist before streamed tool activity");
    assert_eq!(parent.project_id, scope);
    assert_eq!(parent.directory, directory);
    assert_eq!(parent.parent_id, None);
    drop(receiver);
}

// 轨迹提交一次性持久化步骤、工具和原始产物，不依赖后续 UI 快照保存。
pub async fn chat_stream_returns_sse_response_for_docs_request<H: crate::stream_host::StreamHost>() {
    let temp = make_docs_workspace();
    let request = docs_stream_request(None);
    let request_id = format!("chat-resume-{}", uuid::Uuid::new_v4());
    let mut headers = HeaderMap::new();
    headers.insert(
        gateway_resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );
    let response = chat_stream::<H>(
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) }),
        headers,
        Json(request.clone()),
    )
    .await
    .expect("chat stream response")
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).and_then(|value| value.to_str().ok()),
        Some("no-cache")
    );
    assert_eq!(
        response.headers().get(header::CONNECTION).and_then(|value| value.to_str().ok()),
        Some("keep-alive")
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("sse body");
    let body = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(body.contains(r#""type":"chat.delta""#), "{body}");
    assert!(body.contains(r#""type":"chat.done""#), "{body}");
    assert!(body.contains("docs/guide.md"));
    let event_ids = body
        .lines()
        .filter_map(|line| line.strip_prefix("id:"))
        .map(str::trim)
        .map(|id| id.parse::<u64>().expect("numeric SSE id"))
        .collect::<Vec<_>>();
    assert_eq!(event_ids, vec![1, 2]);

    let mut resume_headers = HeaderMap::new();
    resume_headers.insert(
        gateway_resumable_stream::STREAM_REQUEST_ID_HEADER,
        request_id.parse().expect("valid request id header"),
    );
    resume_headers.insert("last-event-id", "1".parse().expect("valid cursor header"));
    let resumed = chat_stream::<H>(
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) }),
        resume_headers,
        Json(request),
    )
    .await
    .expect("resumed chat stream response")
    .into_response();
    let resumed_body =
        axum::body::to_bytes(resumed.into_body(), usize::MAX).await.expect("resumed SSE body");
    let resumed_body = String::from_utf8(resumed_body.to_vec()).expect("utf8 resumed body");
    assert!(!resumed_body.contains(r#""type":"chat.delta""#));
    assert!(resumed_body.contains("id: 2") || resumed_body.contains("id:2"));
    assert!(resumed_body.contains(r#""type":"chat.done""#));
}

pub async fn agent_session_identity_rejects_forged_ui_binding<H: crate::stream_host::StreamHost>() {
    let workspace = tempfile::tempdir().unwrap();
    let result = chat_stream::<H>(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_binding_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("main".into()),
            messages: vec![serde_json::json!({"role":"user", "content":"hello"})],
            options: Some(serde_json::json!({"agent_binding": {
                "agent":"main", "runtime":{"kind":"acp", "backend":"codex"},
                "permission":{"*":"allow"}
            }})),
            ..Default::default()
        }),
    )
    .await;
    assert!(result.is_err(), "client UI JSON must not supply a trusted runtime binding");
}

pub async fn agent_session_identity_derives_backend_and_rejects_conflicts<H: crate::stream_host::StreamHost>() {
    let workspace = tempfile::tempdir().unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
    H::with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), || {
        Box::pin(async {
            let mut request =
                GatewayChatStreamRequest { agent: Some("acp-codex".into()), ..Default::default() };
            let overrides = prepare_gateway_agent_request::<H>(&mut request, None).await.unwrap();
            assert_eq!(overrides.agent.as_deref(), Some("acp-codex"));
            assert_eq!(request.acp_agent.as_deref(), Some("codex"));
            let mut conflicting = request.clone();
            conflicting.acp_agent = Some("claude".into());
            conflicting.options = Some(serde_json::json!({"goal_evaluation":true}));
            assert!(prepare_gateway_agent_request::<H>(&mut conflicting, None).await.is_err());
            let mut conflicting = request;
            conflicting.agent = Some("acp-claude".into());
            assert!(prepare_gateway_agent_request::<H>(&mut conflicting, None).await.is_err());
            Ok(())
        })
    })
    .await
    .unwrap();
}

pub async fn agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority<H: crate::stream_host::StreamHost>() {
    let workspace = tempfile::tempdir().unwrap();
    H::with_tenant_instance(
        session_store::ui_store::current_tenant_scope(),
        workspace.path().to_string_lossy().into_owned(),
        || {
            Box::pin(async {
                for explicit in [
                    None,
                    Some("auto"),
                    Some("  AuTo  "),
                    Some("   "),
                    Some("model-recovery/selected"),
                ] {
                    let mut request = GatewayChatStreamRequest {
                        agent: Some("custom-model".into()),
                        model: explicit.map(str::to_owned),
                        ..Default::default()
                    };
                    let expected = explicit
                        .filter(|value| {
                            !value.trim().is_empty() && !value.trim().eq_ignore_ascii_case("auto")
                        })
                        .unwrap_or("model-recovery/original");
                    for _ in 0..2 {
                        let resolved =
                            prepare_gateway_agent_request::<H>(&mut request, None).await.unwrap();
                        assert_eq!(
                            resolved.model.as_deref(),
                            Some(expected),
                            "new/reopened {explicit:?}"
                        );
                        assert_eq!(request.acp_agent.as_deref(), Some("codex"));
                    }
                }
                let mut inherited = GatewayChatStreamRequest {
                    agent: Some("custom-inherit".into()),
                    ..Default::default()
                };
                assert!(
                    prepare_gateway_agent_request::<H>(&mut inherited, None)
                        .await
                        .unwrap()
                        .model
                        .is_none(),
                    "prompt retains responsibility for global/Auto resolution"
                );
                inherited.model = Some("model-recovery/selected".into());
                assert_eq!(
                    prepare_gateway_agent_request::<H>(&mut inherited, None)
                        .await
                        .unwrap()
                        .model
                        .as_deref(),
                    Some("model-recovery/selected")
                );
                let mut forced = GatewayChatStreamRequest {
                    agent: Some("custom-forced".into()),
                    ..Default::default()
                };
                assert_eq!(
                    prepare_gateway_agent_request::<H>(&mut forced, None)
                        .await
                        .unwrap()
                        .model
                        .as_deref(),
                    Some("model-recovery/original")
                );
                forced.model = Some("  AuTo  ".into());
                assert_eq!(
                    prepare_gateway_agent_request::<H>(&mut forced, None)
                        .await
                        .unwrap()
                        .model
                        .as_deref(),
                    Some("model-recovery/original")
                );
                forced.model = Some("model-recovery/conflict".into());
                assert!(prepare_gateway_agent_request::<H>(&mut forced, None).await.is_err());
                Ok(())
            })
        },
    )
    .await
    .unwrap();
}

pub async fn gateway_first_response_replays_the_same_session_and_rejects_other_directories<H: crate::stream_host::StreamHost>() {
    let workspace = make_docs_workspace();
    let unrelated = make_docs_workspace();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    let query = || Query(InstanceQuery { directory: Some(directory.clone()) });
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut headers = HeaderMap::new();
    headers.insert(STREAM_REQUEST_ID_HEADER, request_id.parse().unwrap());
    let mut request = docs_stream_request(None);
    request.model = Some("model-recovery/original".into());
    let response =
        chat_stream::<H>(query(), headers.clone(), Json(request.clone())).await.unwrap().into_response();
    let session_id = response.headers()["x-vw-session-id"].to_str().unwrap().to_string();
    // Drop the response before consuming Done. The resumable producer owns the turn.
    drop(response);

    let unrelated =
        Query(InstanceQuery { directory: Some(unrelated.path().to_string_lossy().into_owned()) });
    let conflict = chat_stream::<H>(unrelated, headers.clone(), Json(request.clone())).await;
    assert_eq!(conflict.err().unwrap().status, StatusCode::CONFLICT);

    let response =
        chat_stream::<H>(query(), headers.clone(), Json(request.clone())).await.unwrap().into_response();
    assert_eq!(response.headers()["x-vw-session-id"].to_str().unwrap(), session_id);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();
    let done: Vec<Value> = body
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .filter(|event| event["type"] == "chat.done")
        .collect();
    assert_eq!(done.len(), 1, "{body}");
    assert_eq!(done[0]["session_id"], session_id);

    let response = chat_stream::<H>(query(), headers, Json(request)).await.unwrap().into_response();
    let replay = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(replay, bytes);

    let tenant = ui_store::current_tenant_scope();
    let scope =
        ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None).unwrap();
    let snapshot =
        ui_store::load_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert_eq!(snapshot.messages.len(), 2);
    let previews = ui_store::load_sessions_scoped_for_tenant(&tenant, Some(&scope));
    let matching: Vec<_> = previews.iter().filter(|preview| preview.id == session_id).collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].last_content.as_deref(), Some("请列出 docs 文件列表"));
}

pub async fn gateway_first_response_missing_replay_does_not_start_a_new_turn<H: crate::stream_host::StreamHost>() {
    let workspace = make_docs_workspace();
    let mut headers = HeaderMap::new();
    headers.insert(STREAM_REQUEST_ID_HEADER, uuid::Uuid::new_v4().to_string().parse().unwrap());
    headers.insert("last-event-id", "0".parse().unwrap());
    let result = chat_stream::<H>(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        headers,
        Json(docs_stream_request(None)),
    )
    .await;
    assert_eq!(result.err().unwrap().status, StatusCode::GONE);
}

pub async fn task_pool_agent_rejects_unknown_or_disabled_definition<H: crate::stream_host::StreamHost>() {
    let workspace = tempfile::tempdir().unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
    H::with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), move || {
        Box::pin(async move {
            for agent in ["main", "task-pool-unknown"] {
                let mut request =
                    GatewayChatStreamRequest { agent: Some(agent.into()), ..Default::default() };
                assert!(prepare_gateway_agent_request::<H>(&mut request, None).await.is_err());
            }
            Ok(())
        })
    })
    .await
    .unwrap();
}
