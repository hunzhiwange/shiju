//! 通过真实网关流入口验证配置落盘、身份固化和能力撤销。
use gateway_instance::host::InstanceQuery;
use super::profile_acceptance_host::ProfileAcceptanceHost;
use axum::{Json, body::to_bytes, extract::Query, http::HeaderMap, response::IntoResponse};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use vw_api_chat::chat::GatewayChatStreamRequest;

async fn send<H: ProfileAcceptanceHost>(workspace: &std::path::Path, session: &str, agent: &str) -> String {
    let response = H::chat(
        Query(InstanceQuery { directory: Some(workspace.to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(session.to_owned().into()),
            agent: Some(agent.into()),
            model: (agent == "acp-amp-acp").then(|| "model-recovery/original".into()),
            messages: vec![json!({"role":"user","content":"验收运行"})],
            options: Some(if agent == "main" { json!({"full_access":false}) } else { json!({"full_access":false,"cwd":workspace,"acp_model_source":if agent == "acp-amp-acp" { "session_model" } else { "cli_local" },"acp_close_client_after_request":agent != "acp-deepagents"}) }),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_response();
    let bytes = if agent == "acp-amp-acp" {
        use futures_util::StreamExt;
        let mut stream = response.into_body().into_data_stream();
        let mut bytes = Vec::new();
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(chunk) = stream.next().await {
                bytes.extend_from_slice(&chunk.unwrap());
                assert!(bytes.len() < 2_000_000);
            }
        })
        .await;
        assert!(result.is_ok(), "Amp gateway timed out: {}", String::from_utf8_lossy(&bytes));
        bytes
    } else {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            to_bytes(response.into_body(), 2_000_000),
        )
        .await
        .unwrap()
        .unwrap()
        .to_vec()
    };
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn assert_completed_turn<H: ProfileAcceptanceHost>(
    runtime: &tokio::runtime::Runtime,
    workspace: &std::path::Path,
    session: &str,
) {
    let events = runtime.block_on(send::<H>(workspace, session, "main"));
    assert!(events.contains("chat.done"), "{events}");
    assert!(events.contains("验收回复"), "{events}");
    assert!(!events.contains("chat.error"), "{events}");
}

pub fn agent_profile_acceptance_runs_selected_profile_end_to_end<H: ProfileAcceptanceHost>() {
    agent_env::provide_with(
        std::collections::HashMap::from([(
            "VW_MODEL_RECOVERY_TEST_KEY".into(),
            "fixture-key".into(),
        )]),
        run_profile_acceptance::<H>,
    );
}

fn run_profile_acceptance<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    struct HttpMode(Option<std::ffi::OsString>);
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
    let _http = HttpMode(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    // Reuse the existing test-only real transport switch and restore it on panic.
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    if let Ok(workspace) = std::env::var("SHIJU_ACCEPTANCE_RESTART_WORKSPACE") {
        let session = std::env::var("SHIJU_ACCEPTANCE_RESTART_SESSION").unwrap();
        assert_completed_turn::<H>(&runtime, std::path::Path::new(&workspace), &session);
        return;
    }
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = Arc::clone(&requests);
    let router = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move |Json(body): Json<Value>| {
        let captured = Arc::clone(&captured);
        async move {
            captured.lock().unwrap().push(body);
            let chunk = json!({"id":"acceptance","object":"chat.completion.chunk","created":0,"model":"original","choices":[{"index":0,"delta":{"content":"验收回复"},"finish_reason":"stop"}]});
            ([("content-type", "text/event-stream")], format!("data: {chunk}\n\ndata: [DONE]\n\n"))
        }
    }));
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let server = runtime.spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let _fixture =
        runtime.block_on(H::recovery(
            &format!("http://{address}/v1"),
            Some("fixture-key"),
        ));
    let workspace = tempfile::tempdir().unwrap();
    let session = format!("ses_acceptance_{}", uuid::Uuid::new_v4());
    for rule in ["ACCEPTANCE_ORIGINAL_RULE", "ACCEPTANCE_EDITED_RULE"] {
        runtime.block_on(agent_config::runtime::update(json!({"agents":{"main":{
            "label":"中文验收助手", "description":"持久化验收", "recommended_prompts":["检查项目"],
            "system_prompt":rule,
            "defaults":{
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":[]}, "mcp":{"mode":"fixed","value":[]}
            }
        }}}))).unwrap();
        assert_completed_turn::<H>(&runtime, workspace.path(), &session);
    }
    // A fresh process cannot reuse an in-memory binding or config cache.
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "gateway::api::handlers::session::agent_profile_acceptance_tests::agent_profile_acceptance_runs_selected_profile_end_to_end", "--nocapture"])
        .env("SHIJU_ACCEPTANCE_RESTART_WORKSPACE", workspace.path())
        .env("SHIJU_ACCEPTANCE_RESTART_SESSION", &session)
        .output().unwrap();
    assert!(
        child.status.success(),
        "restart failed: {}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert_acp_profile::<H>(&runtime, workspace.path(), "codex");
    server.abort();
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 3, "two live turns plus one fresh-process turn");
    for request in captured.iter() {
        assert_eq!(request["model"], "original");
        let body = request["messages"].to_string();
        assert!(body.contains("ACCEPTANCE_ORIGINAL_RULE"));
        assert!(!body.contains("ACCEPTANCE_EDITED_RULE"));
    }
}

pub async fn agent_profile_acceptance_rejects_revoked_capabilities<H: ProfileAcceptanceHost>() {
    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let mcp = workspace.path().join(".vwacprc.json");
    std::fs::write(
        &mcp,
        serde_json::to_vec(
            &json!({"mcpServers":[{"name":"revoked","command":"missing-command","enabled":true}]}),
        )
        .unwrap(),
    )
    .unwrap();
    agent_config::runtime::update(
        json!({"agents":{"main":{"defaults":{"mcp":{"mode":"fixed","value":["revoked"]}}}}}),
    )
    .await
    .unwrap();
    let session = format!("ses_revoked_{}", uuid::Uuid::new_v4());
    let selected_session = session.clone();
    let directory = workspace.path().to_string_lossy().into_owned();
    H::with_tenant_instance(
        H::current_tenant_scope(),
        directory.clone(),
        move || {
            Box::pin(async move {
                let (binding, _) = session_engine::admission::prepare::<H::Admission>(
                    &selected_session,
                    Some("main"),
                    None,
                    None,
                    std::path::Path::new(&directory),
                )
                .await
                .unwrap();
                assert_eq!(
                    serde_json::to_value(binding.defaults.mcp).unwrap(),
                    json!({"mode":"fixed","value":["revoked"]})
                );
                Ok(())
            })
        },
    )
    .await
    .unwrap();
    std::fs::write(
        &mcp,
        serde_json::to_vec(
            &json!({"mcpServers":[{"name":"revoked","command":"missing-command","enabled":false}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let result = H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(session.into()),
            agent: Some("main".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    )
    .await;
    assert!(result.is_err(), "revoked MCP must reject before model or command execution");
}

pub async fn agent_profile_acceptance_catalog_uses_request_tenant<H: ProfileAcceptanceHost>() {
    use axum::{body::Body, extract::Extension, http::Request};
    use tower::ServiceExt;
    use vw_api_transport::id::TenantScope;

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    agent_config::runtime::update_global_for_tenant(
        &TenantScope::default(),
        json!({
            "agents":{"main":{"label":"DEFAULT_CATALOG_SENTINEL"}}
        }),
    )
    .await
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    for platform in ["acceptance-one", "acceptance-two"] {
        let tenant = TenantScope::new(platform, "validation").unwrap();
        agent_config::runtime::update_global_for_tenant(
            &tenant,
            json!({
                "agents":{"main":{"label":platform}}
            }),
        )
        .await
        .unwrap();
        let response = H::catalog_router()
            .layer(Extension(tenant))
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/agent?include_disabled=true&directory={}",
                        directory.path().display()
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), 2_000_000).await.unwrap();
        let agents: Vec<Value> = serde_json::from_slice(&body).unwrap();
        let main = agents.iter().find(|entry| entry["key"] == "main").unwrap();
        assert_eq!(
            main["label"], platform,
            "catalog must use the request tenant, not the ambient default"
        );
    }
}

pub async fn agent_config_cleanup_global_profile_roundtrips_to_new_session<H: ProfileAcceptanceHost>() {
    use axum::{body::Body, extract::Extension, http::Request};
    use tower::ServiceExt;
    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for (key, runtime) in
        [("main", json!({"kind":"native"})), ("acp-codex", json!({"kind":"acp","backend":"codex"}))]
    {
        let patch = json!({"agents":{key:{
            "enabled":true,"runtime":runtime,"description":"中文描述\n第二行",
            "system_prompt":"GLOBAL_PROFILE_RULE",
            "defaults":{"full_access":true,"permission":{"mode":"fixed","value":{"*":"deny"}},
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":[]},"mcp":{"mode":"fixed","value":[]}}
        }}});
        let response = H::config_router()
            .layer(Extension(H::current_tenant_scope()))
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/config")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let saved: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap())
                .unwrap();
        assert_eq!(saved["agents"][key]["defaults"]["full_access"], true);
        for workspace in [first.path(), second.path()] {
            let runtime = runtime.clone();
            let directory = workspace.to_string_lossy().into_owned();
            H::with_tenant_instance(
                H::current_tenant_scope(),
                directory.clone(),
                move || {
                    Box::pin(async move {
                        let config = H::config().await.unwrap();
                        let profile = &config.agents[key];
                        assert_eq!(profile.description.as_deref(), Some("中文描述\n第二行"));
                        let session = format!("ses_cleanup_{}", uuid::Uuid::new_v4());
                        let (binding, _) = session_engine::admission::prepare::<H::Admission>(
                            &session,
                            Some(key),
                            None,
                            None,
                            std::path::Path::new(&directory),
                        )
                        .await
                        .unwrap();
                        assert_eq!(binding.system_prompt.as_deref(), Some("GLOBAL_PROFILE_RULE"));
                        assert_eq!(serde_json::to_value(&binding.runtime).unwrap(), runtime);
                        assert_eq!(
                            serde_json::to_value(&binding.defaults).unwrap(),
                            serde_json::to_value(&profile.defaults).unwrap()
                        );
                        assert!(
                            H::resolve_full_access(
                                &directory,
                                &session,
                                None,
                                binding.defaults.full_access,
                            )
                            .await
                            .unwrap()
                        );
                        assert!(
                            !H::resolve_full_access(
                                &directory,
                                &session,
                                Some(false),
                                true,
                            )
                            .await
                            .unwrap()
                        );
                        assert!(
                            !H::resolve_full_access(
                                &directory, &session, None, true,
                            )
                            .await
                            .unwrap()
                        );
                        Ok(())
                    })
                },
            )
            .await
            .unwrap();
        }
    }
}


mod onboarding;
pub use onboarding::*;
mod peer;
use peer::assert_acp_profile;
