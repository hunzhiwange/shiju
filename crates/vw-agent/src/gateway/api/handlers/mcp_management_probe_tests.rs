use crate::{
    agent_apps::mcp::McpCatalogStore, gateway::agent_apps_diagnostic_jobs::DiagnosticJobs,
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::IntoResponse,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tower::ServiceExt;
use vw_api_types::{
    ProjectScope, TenantScope,
    agent_apps_diagnostics::{
        DiagnosticJobIdentity, DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticResult,
    },
    mcp_management::{McpCatalogScope, McpConnectionDefinition, McpConnectionOutcome},
};

const TOKEN: &str = "fixture-mcp39-member-token-with-no-secrets";
const OTHER: &str = "fixture-mcp39-other-member-token";
const AUTH: &str = "Bearer dynamic-acceptance-skey";
const ENV: &str = "SHIJU_MCP_TASK39_TEST_ROOT";
const QUERY: &str = "projectId=project&targetHostId=host&appId=claude";

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    company: &str,
    token: &str,
    authenticated: bool,
    payload: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .header("x-agent-apps-token", token);
    if authenticated {
        request = request.header("authorization", AUTH);
    }
    let response =
        app.clone().oneshot(request.body(Body::from(payload.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cc_mcp_t39_checks_reuse_scoped_jobs_and_cancel_only_the_owned_batch() {
    let Some(root) = std::env::var_os(ENV) else {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::cc_mcp_t39_checks_reuse_scoped_jobs_and_cancel_only_the_owned_batch"
                )
                .split_once("::")
                .unwrap()
                .1,
                "--nocapture",
            ])
            .env(ENV, root.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .status()
            .unwrap();
        assert!(status.success(), "MCP checks router fixture failed");
        assert!(root.path().join("verified").exists(), "isolated behavior test must run");
        return;
    };
    let root = Path::new(&root);
    let observed = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests = observed.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, Router::new().fallback(move |request: Request<Body>| {
            let requests = requests.clone();
            async move {
                if request.method() != axum::http::Method::POST {
                    return StatusCode::METHOD_NOT_ALLOWED.into_response();
                }
                let slow = request.uri().path() == "/slow";
                let body = to_bytes(request.into_body(), 32768).await.unwrap();
                let payload: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                if payload["method"] == "initialize" {
                    requests.fetch_add(1, Ordering::SeqCst);
                    if slow { std::future::pending::<()>().await; }
                    return Json(json!({"jsonrpc":"2.0","id":payload["id"],"result":{
                        "protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"fixture","version":"1"}
                    }})).into_response();
                }
                StatusCode::ACCEPTED.into_response()
            }
        })).await.unwrap();
    });
    let database = GlobalDatabase::new(root.join("catalog.sqlite3")).unwrap();
    let catalog = McpCatalogStore::new(database.clone()).unwrap();
    let scope = McpCatalogScope::Project {
        scope: ProjectScope::new(TenantScope::new("p", "a").unwrap(), "project".into()).unwrap(),
    };
    let mut permits = Vec::new();
    for id in ["fast", "slow"] {
        let url = format!("{base}/{id}");
        let record = catalog
            .create(
                &scope,
                vw_agent_apps::mcp::catalog::McpCatalogDraft {
                    id: id.into(),
                    name: None,
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection: McpConnectionDefinition::StreamableHttp {
                        url: url.clone(),
                        headers: Default::default(),
                    },
                },
            )
            .unwrap();
        permits.push(json!({"project_id":"project","app_id":"claude","record_id":id,"connection_revision":record.connection_revision,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))}));
    }
    let grant = json!({"platform_id":"p","company_id":"a","host_id":"host","mcp_probe":permits});
    let mut foreign = grant.clone();
    foreign["company_id"] = json!("b");
    let policy = json!({"principals":[
        {"id":"member","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[grant.clone(),foreign]},
        {"id":"other","token_sha256":hex::encode(Sha256::digest(OTHER)),"grants":[grant]}
    ],"hosts":[{"id":"host","binding":"local","online":true,"home":root,"applications":root,"search_paths":[]}]});
    fs::write(root.join("access.json"), policy.to_string()).unwrap();
    fs::set_permissions(root.join("access.json"), fs::Permissions::from_mode(0o600)).unwrap();
    let expected_catalog = catalog.list(&scope).unwrap();
    let jobs = DiagnosticJobs::open(root).unwrap();
    let app = crate::gateway::dynamic_tools_acceptance_router()
        .layer(Extension(database))
        .layer(Extension(jobs));
    let uri = format!("/v1/agent-apps/mcp/checks?{QUERY}");
    let payload = json!({"serverIds":["fast","slow"],"generation":17});
    assert_eq!(
        send(&app, "POST", &uri, "a", TOKEN, false, &payload).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(send(&app, "POST", &uri, "c", TOKEN, true, &payload).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        send(
            &app,
            "POST",
            &uri,
            "a",
            TOKEN,
            true,
            &json!({"serverIds":["fast"], "padding":"x".repeat(33000)})
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Tenant b has matching grants but no matching catalog records. No probe may run.
    assert_eq!(
        send(&app, "POST", &uri, "b", TOKEN, true, &payload).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(observed.load(Ordering::SeqCst), 0);
    for invalid in [
        json!({"serverIds":["fast","fast"]}),
        json!({"serverIds":[]}),
        json!({"serverIds":["fast"],"url":"http://untrusted"}),
    ] {
        assert_eq!(
            send(&app, "POST", &uri, "a", TOKEN, true, &invalid).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let denied = uri.replace("projectId=project", "projectId=foreign");
    assert_eq!(
        send(&app, "POST", &denied, "a", TOKEN, true, &payload).await.0,
        StatusCode::FORBIDDEN
    );
    let (status, submitted) = send(&app, "POST", &uri, "a", TOKEN, true, &payload).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{submitted}");
    assert_eq!(submitted["kind"], "mcp");
    assert!(submitted["correlationId"].as_str().is_some_and(|id| !id.is_empty()));
    let identity: DiagnosticJobIdentity = serde_json::from_value(submitted.clone()).unwrap();
    let id = identity.task_id.as_ref();
    let read = format!("/v1/agent-apps/diagnostics/jobs/{id}?{QUERY}");
    let cancel = format!("/v1/agent-apps/diagnostics/jobs/{id}/cancel?{QUERY}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        assert!(tokio::time::Instant::now() < deadline, "partial result deadline");
        let (status, body) = send(&app, "GET", &read, "a", TOKEN, true, &Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["progress"]["completed"] == 1 && observed.load(Ordering::SeqCst) == 2 {
            let snapshot: DiagnosticJobSnapshot = serde_json::from_value(body).unwrap();
            assert_eq!(snapshot.job, identity);
            let [DiagnosticResult::McpConnectionCheck { scope: result_scope, generation, item }] =
                snapshot.results.as_deref().unwrap()
            else {
                panic!("management API must return the formal revision-bound result");
            };
            assert_eq!(result_scope, &scope);
            assert_eq!(*generation, 17);
            assert_eq!(item.id.0, "fast");
            assert_eq!(item.connection_revision, expected_catalog[0].connection_revision);
            assert_eq!(item.outcome, McpConnectionOutcome::Connected { tool_count: Some(0) });
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for (method, endpoint) in [("GET", &read), ("POST", &cancel)] {
        for (company, token) in [("b", TOKEN), ("a", OTHER)] {
            assert_eq!(
                send(&app, method, endpoint, company, token, true, &Value::Null).await.0,
                StatusCode::NOT_FOUND
            );
        }
        assert_eq!(
            send(&app, method, endpoint, "a", TOKEN, false, &Value::Null).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let (status, independent) =
        send(&app, "POST", &uri, "a", TOKEN, true, &json!({"serverIds":["fast"]})).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_ne!(independent["taskId"], submitted["taskId"]);
    assert_eq!(send(&app, "POST", &cancel, "a", TOKEN, true, &Value::Null).await.0, StatusCode::OK);
    let final_state = loop {
        assert!(tokio::time::Instant::now() < deadline, "cancellation cleanup deadline");
        let (_, body) = send(&app, "GET", &read, "a", TOKEN, true, &Value::Null).await;
        if body["state"]["status"] == "cancelled" {
            break body;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let cancelled_snapshot: DiagnosticJobSnapshot =
        serde_json::from_value(final_state.clone()).unwrap();
    assert_eq!(cancelled_snapshot.state, DiagnosticJobState::Cancelled {});
    assert_eq!(cancelled_snapshot.job, identity);
    assert_eq!(final_state["progress"]["completed"], 1);
    assert_eq!(final_state["results"].as_array().unwrap().len(), 1);
    assert_eq!(send(&app, "POST", &cancel, "a", TOKEN, true, &Value::Null).await.1, final_state);
    let independent_read = format!(
        "/v1/agent-apps/diagnostics/jobs/{}?{QUERY}",
        independent["taskId"].as_str().unwrap()
    );
    loop {
        assert!(tokio::time::Instant::now() < deadline, "independent job deadline");
        let (status, body) =
            send(&app, "GET", &independent_read, "a", TOKEN, true, &Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        if body["state"]["status"] == "succeeded" {
            let snapshot: DiagnosticJobSnapshot = serde_json::from_value(body).unwrap();
            let [DiagnosticResult::McpConnectionCheck { generation, item, .. }] =
                snapshot.results.as_deref().unwrap()
            else {
                panic!("independent batch must keep the same result contract");
            };
            assert_eq!(*generation, 0, "omitted request generation defaults to zero");
            assert_eq!(item.connection_revision, expected_catalog[0].connection_revision);
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(observed.load(Ordering::SeqCst), 3, "poll/cancel must not replay probes");
    // The original diagnostic HTTP entry point retains its handshake report contract.
    let (status, legacy) = send(
        &app,
        "POST",
        &format!("/v1/agent-apps/diagnostics/jobs?{QUERY}"),
        "a",
        TOKEN,
        true,
        &json!({"kind":"mcp", "serverIds":["fast"]}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let legacy: DiagnosticJobIdentity = serde_json::from_value(legacy).unwrap();
    let legacy_read =
        format!("/v1/agent-apps/diagnostics/jobs/{}?{QUERY}", legacy.task_id.as_ref());
    loop {
        assert!(tokio::time::Instant::now() < deadline, "legacy diagnostic deadline");
        let (status, body) = send(&app, "GET", &legacy_read, "a", TOKEN, true, &Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        let snapshot: DiagnosticJobSnapshot = serde_json::from_value(body).unwrap();
        if snapshot.state == (DiagnosticJobState::Succeeded {}) {
            let [DiagnosticResult::Mcp { server_id, report }] =
                snapshot.results.as_deref().unwrap()
            else {
                panic!("legacy diagnostics must keep the handshake report contract");
            };
            assert_eq!(server_id.0, "fast");
            assert!(matches!(
                report.outcome,
                vw_api_types::agent_apps_diagnostics::DiagnosticOutcome::Succeeded { .. }
            ));
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(observed.load(Ordering::SeqCst), 4);
    assert_eq!(catalog.list(&scope).unwrap(), expected_catalog);
    let persisted = fs::read_to_string(root.join("diagnostics.json")).unwrap();
    assert!(persisted.contains(id));
    assert!(!final_state.to_string().contains(&base));
    server.abort();
    let _ = server.await;
    fs::write(root.join("verified"), b"").unwrap();
}
