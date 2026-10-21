//! 桌面端网关适配测试，验证租户项目作用域、异步桥接和错误原样传播。
//! 测试使用无网络客户端边界，确保无效项目标识在发出请求前被拒绝。

#[cfg(not(target_arch = "wasm32"))]
async fn stop_stub(
    status: &'static str,
    body: &'static str,
    tenant: vw_gateway_client::vw_api_types::id::TenantScope,
) -> (vw_gateway_client::GatewayClient, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("stop stub should bind");
    let port = listener.local_addr().expect("stop stub address").port();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("stop request should connect");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).await.expect("stop request should be readable");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.expect("stop response should be written");
        String::from_utf8(request).expect("stop request should be utf8")
    });
    let endpoint =
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant);
    let client = vw_gateway_client::GatewayClient::new(endpoint).expect("stop client should build");
    (client, handle)
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn session_open_requests_summary_and_latest_page_only() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).await.unwrap();
                bytes.extend_from_slice(&buffer[..read]);
                if read == 0 || bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(bytes).unwrap();
            let body = if request.contains("/ui/summary") {
                r#"{"id":"session-page","title":"Paged","updated_ms":42,"message_count":10000,"call_count":0}"#
            } else {
                r#"{"items":[],"total_count":10000}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            requests.push(request);
        }
        requests
    });
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        port,
    ))
    .unwrap();

    let (summary, page) = super::gateway_bounded_session_open_with_client(
        &client,
        "session-page",
        "/workspace/project",
    )
    .await
    .unwrap();
    let requests = server.await.unwrap();

    assert_eq!(summary.message_count, 10_000);
    assert!(page.items.is_empty());
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .any(|request| request.starts_with("GET /v1/session/session-page/ui/summary?"))
    );
    let page_request = requests
        .iter()
        .find(|request| request.starts_with("GET /v1/session/session-page/ui/messages?"))
        .expect("latest page request");
    assert!(page_request.contains("limit=64"));
    assert!(!requests.iter().any(|request| request.contains("/ui/without-raw")));
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("session_gateway_tests"));
}

#[test]
fn gateway_client_binds_configured_endpoint_without_network_access() {
    let client =
        super::gateway_client().expect("configured gateway endpoint should build a client");
    let configured = crate::app::config::gateway_client_endpoint();

    assert_eq!(client.endpoint().host, configured.host);
    assert_eq!(client.endpoint().port, configured.port);
    assert_eq!(client.endpoint().tenant_scope(), configured.tenant_scope());
}

#[test]
fn desktop_project_calls_use_active_tenant_scope() {
    let scope = super::active_tenant_scope();
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(&scope, endpoint.tenant_scope());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn task_plan_run_stop_gateway_boundary_preserves_task_and_tenant_scope() {
    use vw_gateway_client::vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

    let tenant = TenantScope::new("platform-nested", "company-nested").expect("tenant scope");
    let (client, request) = stop_stub(
        "200 OK",
        r#"{"task_id":"task-nested","status":"cancelled","current_session_id":"task-nested-1"}"#,
        tenant,
    )
    .await;

    let status = super::gateway_task_plan_run_stop_with_client(&client, " task-nested ")
        .await
        .expect("stop response");
    let request = request.await.expect("stop stub should finish");
    let request_lower = request.to_ascii_lowercase();

    assert_eq!(status.task_id, "task-nested");
    assert_eq!(status.status, vw_gateway_client::vw_api_types::task::TaskPlanRunStatus::Cancelled);
    assert!(request.starts_with("POST /v1/task-plan-run/task-nested/stop HTTP/1.1"));
    assert!(
        request_lower
            .contains(&format!("{}: platform-nested", PLATFORM_ID_HEADER.to_ascii_lowercase()))
    );
    assert!(
        request_lower
            .contains(&format!("{}: company-nested", COMPANY_ID_HEADER.to_ascii_lowercase()))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn task_plan_run_stop_gateway_boundary_surfaces_unknown_and_cross_tenant_errors() {
    use vw_gateway_client::vw_api_types::id::TenantScope;

    for (status, body) in [
        ("404 Not Found", r#"{"error":"task-plan-run 不存在"}"#),
        ("403 Forbidden", r#"{"error":"当前租户不可见"}"#),
    ] {
        let tenant = TenantScope::new("platform-nested", "company-nested").expect("tenant scope");
        let (client, request) = stop_stub(status, body, tenant).await;
        let result = super::gateway_task_plan_run_stop_with_client(&client, "task-nested").await;
        request.await.expect("error stop stub should finish");
        assert!(result.is_err(), "{status} must remain an explicit gateway error");
    }

    let unused_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("unused stop port should bind");
    let unused_port = unused_listener.local_addr().expect("unused stop address").port();
    drop(unused_listener);
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        unused_port,
    ))
    .expect("unreachable client should build");
    assert!(
        super::gateway_task_plan_run_stop_with_client(&client, "task-request-failure")
            .await
            .is_err()
    );
}

#[test]
fn active_project_scope_preserves_active_tenant_and_project_id() {
    let scope = super::active_project_scope("project-stable".into())
        .expect("non-empty project id should build a project scope");
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(scope.project_id().as_ref(), "project-stable");
}

#[test]
fn active_project_scope_rejects_empty_project_id() {
    let error =
        super::active_project_scope("".into()).expect_err("empty project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn active_project_scope_rejects_whitespace_project_id() {
    let error = super::active_project_scope(" \t ".into())
        .expect_err("whitespace-only project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn desktop_agent_scheduler_calls_use_resolved_active_scope() {
    let scope = super::active_agent_scheduler_scope("project-stable".into())
        .expect("resolved project id should build a scheduler scope");
    let project_scope = scope.project_scope();
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(project_scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(project_scope.project_id().as_ref(), "project-stable");
    assert_eq!(
        vw_gateway_client::GatewayClient::agent_scheduler_events_stream_path(&scope, None),
        "/v1/agent-scheduler/projects/project-stable/events/stream"
    );
}

#[test]
fn active_agent_scheduler_scope_propagates_invalid_project_id() {
    let error = super::active_agent_scheduler_scope(" ".into())
        .expect_err("invalid project id should be rejected before scheduler scope construction");

    assert_eq!(error, "project id must not be empty");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_runs_future_without_existing_runtime() {
    let result = super::block_on_gateway(async { Ok::<_, String>("completed".to_string()) });

    assert_eq!(result.as_deref(), Ok("completed"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_propagates_future_error() {
    let result = super::block_on_gateway(async { Err::<String, _>("gateway failed".to_string()) });

    assert_eq!(result, Err("gateway failed".to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_save_session_result_preserves_original_gateway_error() {
    let result = super::gateway_save_session_result_from_future(async {
        Err::<Option<std::path::PathBuf>, _>("original gateway save failure".to_string())
    });

    assert_eq!(result, Err("original gateway save failure".to_string()));
}

#[test]
fn desktop_ui_snapshot_omits_runtime_trajectory() {
    let session = vw_shared::session::ui_types::ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: vec![serde_json::json!({"tool": "shell"})],
        steps: vec![vw_shared::session::ui_types::ChatSessionStep {
            index: 1,
            turn_id: Some("turn-1".to_string()),
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: vw_shared::session::ui_types::TokenUsage::default(),
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("provider/model".to_string()),
            tool_timings: Vec::new(),
        }],
        created_ms: 1,
        updated_ms: 2,
    };

    let snapshot = super::session_ui_content_snapshot(&session);

    assert!(snapshot.calls.is_empty());
    assert!(snapshot.steps.is_empty());
    assert_eq!(snapshot.id, session.id);
    assert_eq!(snapshot.updated_ms, session.updated_ms);
}

#[test]
fn gateway_session_message_delta_payload_is_bounded() {
    use vw_shared::session::ui_types::{
        ChatMessage, ChatRole, SessionMessageDelta, SessionMessageRecord,
    };

    let delta = SessionMessageDelta {
        session_id: "session-bounded".to_string(),
        expected_updated_ms: 10,
        updated_ms: 11,
        upserts: vec![SessionMessageRecord {
            seq_no: 10_000,
            message_id: Some("assistant-active".to_string()),
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: "delta".repeat(128),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        }],
        truncate_after: None,
    };
    let bytes = serde_json::to_vec(&delta).unwrap();
    let full_session = vw_shared::session::ui_types::ChatSession {
        id: delta.session_id.clone(),
        title: "baseline".to_string(),
        messages: (0..10_000).map(|_| delta.upserts[0].message.clone()).collect(),
        message_ids: vec![None; 10_000],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 11,
    };
    let full_bytes = serde_json::to_vec(&full_session).unwrap().len();

    assert_eq!(delta.upserts.len(), 1);
    assert!(bytes.len() < 1_024, "single-entry payload was {} bytes", bytes.len());
    assert!(full_bytes > bytes.len() * 1_000);
    eprintln!("full_session_bytes={full_bytes} delta_bytes={}", bytes.len());
}

#[test]
fn permanent_session_save_error_only_classifies_payload_too_large() {
    assert!(super::is_permanent_session_save_error(
        "gateway request failed: 413 Payload Too Large"
    ));
    assert!(!super::is_permanent_session_save_error(
        "gateway request failed: 503 Service Unavailable"
    ));
    assert!(!super::is_permanent_session_save_error("database is locked"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_runs_future_inside_multithread_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("multi-thread runtime should build");

    let result = runtime.block_on(async {
        super::block_on_gateway(async { Ok::<_, String>("inside-runtime".to_string()) })
    });

    assert_eq!(result.as_deref(), Ok("inside-runtime"));
}

#[test]
fn desktop_session_persistence_surface_is_gateway_backed() {
    let _ = super::gateway_load_session_async;
    let _ = super::gateway_load_session_any_async;
    let _ = super::gateway_load_sessions_scoped_async;
    let _ = super::gateway_save_session_async;
    let _ = super::gateway_save_agent_session_scoped_async;
    let _ = super::gateway_session_file_path_async;
    let _ = super::gateway_session_todo_list_async;
    let _ = super::gateway_task_plan_run_stop_async;
}

#[test]
fn desktop_gateway_async_surfaces_are_exposed_without_network_access() {
    let _ = super::gateway_load_archived_session_ids_async;
    let _ = super::gateway_session_preview_meta_async;
    let _ = super::gateway_session_step_llm_raw_async;
    let _ = super::gateway_resolve_session_scope_id_async;
    let _ = super::gateway_permission_list_owned_async;
    let _ = super::gateway_set_session_scope_async;
    let _ = super::gateway_current_session_scope_async;
    let _ = super::gateway_question_list_async;
    let _ = super::gateway_question_reply_async;
    let _ = super::gateway_question_reject_async;
    let _ = super::gateway_permission_list_async;
    let _ = super::gateway_permission_reply_async;
    let _ = super::gateway_external_apps_async;
    let _ = super::gateway_external_open_async;
    let _ = super::gateway_external_reveal_async;
    let _ = super::gateway_skills_async;
    let _ = super::gateway_skill_detail_async;
    let _ = super::gateway_skill_create_async;
    let _ = super::gateway_skill_install_builtin_async;
    let _ = super::gateway_skill_set_enabled_async;
    let _ = super::gateway_skill_delete_async;
    let _ = super::gateway_plugins_async;
    let _ = super::gateway_plugin_set_enabled_async;
    let _ = super::gateway_plugin_delete_async;
    let _ = super::gateway_project_change_records_async;
    let _ = super::gateway_skill_market_sources_async;
    let _ = super::gateway_skill_market_recommended_async;
    let _ = super::gateway_skill_market_skills_async;
    let _ = super::gateway_skill_market_categories_async;
    let _ = super::gateway_skill_market_skill_detail_async;
    let _ = super::gateway_skill_market_skill_file_async;
    let _ = super::gateway_skill_market_skill_sets_async;
    let _ = super::gateway_skill_market_skill_set_detail_async;
    let _ = super::gateway_shiju_account_login_async;
    let _ = super::gateway_shiju_account_login_poll_async;
    let _ = super::gateway_shiju_account_profile_async;
    let _ = super::gateway_shiju_app_catalog_async;
    let _ = super::gateway_shiju_apps_async;
    let _ = super::gateway_shiju_app_api_key_binding_async;
    let _ = super::gateway_shiju_app_api_keys_async;
    let _ = super::gateway_shiju_app_save_api_key_binding_async;
    let _ = super::gateway_shiju_app_prepare_launch_async;
    let _ = super::gateway_shiju_skill_search_async;
    let _ = super::gateway_shiju_skill_installed_async;
    let _ = super::gateway_shiju_system_skills_async;
    let _ = super::gateway_shiju_maintained_skills_async;
    let _ = super::gateway_shiju_skill_tags_async;
    let _ = super::gateway_shiju_tenant_skill_tags_async;
    let _ = super::gateway_shiju_skill_detail_async;
    let _ = super::gateway_shiju_skill_readme_async;
    let _ = super::gateway_shiju_skill_versions_async;
    let _ = super::gateway_shiju_skill_remote_install_async;
    let _ = super::gateway_shiju_skill_remote_uninstall_async;
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_gateway_blocking_surfaces_are_exposed_without_invoking_gateway() {
    let _ = super::gateway_load_session;
    let _ = super::gateway_load_session_any;
    let _ = super::gateway_load_sessions_scoped;
    let _ = super::gateway_load_archived_session_ids;
    let _ = super::gateway_session_preview_meta;
    let _ = super::gateway_session_file_path;
    let _ = super::gateway_resolve_session_scope_id;
    let _ = super::gateway_save_session_result;
    let _ = super::gateway_save_session;
    let _ = super::gateway_set_session_scope;
    let _ = super::gateway_current_session_scope;
    let _ = super::gateway_question_list;
    let _ = super::gateway_question_reply;
    let _ = super::gateway_question_reject;
    let _ = super::gateway_permission_list;
    let _ = super::gateway_permission_reply;
    let _ = super::gateway_save_agent_session_scoped;
    let _ = super::gateway_project_change_records;
}
