//! Managed checks return revision-bound results while diagnostics keep handshake reports.
use super::{
    catalog_create_tests::{http_draft, with_database},
    network_probe::{self, McpHandshakeStartError},
    network_probe_tests::{Fixture, TOKEN},
};
use crate::global_database::run_sync_database;
use axum::{
    Json, Router,
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vw_api_types::{
    agent_apps_diagnostics::{DiagnosticJobState, DiagnosticResult},
    mcp_management::{McpConnectionDefinition, McpConnectionOutcome, McpFailureCode},
};

#[test]
fn cc_mcp_t39_revision_results_reuse_connection_checks_and_preserve_diagnostics()
-> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database.clone())?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let tools = Arc::new(AtomicUsize::new(0));
            let observed = tools.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let base = format!("http://{}/mcp", listener.local_addr()?);
            let server = tokio::spawn(async move {
                axum::serve(listener, Router::new().route("/mcp", post(move |Json(request): Json<Value>| {
                    let observed = observed.clone();
                    async move {
                        let result = match request["method"].as_str() {
                            Some("initialize") => json!({"protocolVersion":"2025-03-26", "capabilities":{"tools":{}}, "serverInfo":{"name":"fixture","version":"1"}}),
                            Some("tools/list") => {
                                observed.fetch_add(1, Ordering::SeqCst);
                                json!({"tools":[{"name":"fixture","inputSchema":{"type":"object"}}]})
                            }
                            _ => return axum::http::StatusCode::ACCEPTED.into_response(),
                        };
                        Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
                    }
                }))).await.unwrap();
            });
            let scope = fixture.catalog_scope();
            let record = run_sync_database(|| Ok(fixture.catalog.create(&scope, http_draft("managed", &base))))?.unwrap();
            let url = format!("{base}?revision=current");
            let record = run_sync_database(|| Ok(fixture.catalog.update_connection(&scope, &record.record,
                McpConnectionDefinition::StreamableHttp { url: url.clone(), headers: Default::default() })))?.unwrap();
            assert!(record.connection_revision > 1, "fixture must not mask a fabricated revision=1");
            fixture.policy(&[json!({"project_id":"project","app_id":"claude","record_id":"managed",
                "connection_revision":record.connection_revision,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))})])?;
            let headers = HeaderMap::from_iter([("x-agent-apps-token".parse().unwrap(), HeaderValue::from_static(TOKEN))]);
            let start = |generation| network_probe::start_connection_checks(
                &fixture.jobs, database.clone(), Some(fixture.root.path().join("policy.json")),
                &headers, fixture.owner.scope.clone(), vec![record.record.id.clone()], generation,
            );
            let job = start(42).unwrap();
            let snapshot = fixture.finish(&job).await?;
            assert_eq!(snapshot.state, DiagnosticJobState::Succeeded {});
            let [DiagnosticResult::McpConnectionCheck {scope: result_scope, generation, item}] = snapshot.results.as_deref().unwrap() else { panic!("managed checks must publish the formal revision-bound result") };
            assert_eq!(result_scope, &scope);
            assert_eq!(*generation, 42);
            assert_eq!(item.id, record.record.id);
            assert_eq!(item.connection_revision, record.connection_revision);
            assert_eq!(item.outcome, McpConnectionOutcome::Connected {tool_count: Some(1)});
            assert_eq!(tools.load(Ordering::SeqCst), 1, "tool count must come from real discovery");
            assert!(fixture.catalog.current_connection_check_items(&scope, &snapshot, &job.task_id, 41).unwrap().is_empty());
            assert_eq!(fixture.catalog.current_connection_check_items(&scope, &snapshot, &job.task_id, 42).unwrap(), vec![item.clone()]);
            // The existing diagnostic entry point still emits its handshake-only contract.
            let diagnostic = fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]).unwrap();
            let old = fixture.finish(&diagnostic).await?;
            assert!(matches!(old.results.as_deref().unwrap(), [DiagnosticResult::Mcp { .. }]));
            assert_eq!(tools.load(Ordering::SeqCst), 1, "diagnostics must not acquire tool-discovery behavior");
            let replacement = run_sync_database(|| Ok(fixture.catalog.update_connection(&scope, &record.record,
                McpConnectionDefinition::StreamableHttp { url: base, headers: Default::default() })))?.unwrap();
            assert!(replacement.connection_revision > record.connection_revision);
            assert!(fixture.catalog.current_connection_check_items(&scope, &snapshot, &job.task_id, 42).unwrap().is_empty());
            assert_eq!(start(43), Err(McpHandshakeStartError::Record(McpFailureCode::StaleRevision)));
            fixture.policy(&[])?;
            assert_eq!(start(44), Err(McpHandshakeStartError::Access(axum::http::StatusCode::FORBIDDEN)));
            server.abort();
            let _ = server.await;
            Ok(())
        })
    })
}

#[test]
fn cc_mcp_t39_revision_checks_reject_revoked_success_and_cancel_owned_work() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database.clone())?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let started = Arc::new(tokio::sync::Notify::new());
            let release = Arc::new(tokio::sync::Semaphore::new(0));
            let ready = started.clone();
            let gate = release.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            let server = tokio::spawn(async move {
                axum::serve(listener, Router::new().route("/mcp", post(move |Json(request): Json<Value>| {
                    let ready = ready.clone(); let gate = gate.clone();
                    async move {
                        if request["method"] == "initialize" {
                            ready.notify_one();
                            gate.acquire().await.unwrap().forget();
                            return Json(json!({"jsonrpc":"2.0","id":request["id"],"result":{
                                "protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"fixture","version":"1"}
                            }})).into_response();
                        }
                        axum::http::StatusCode::ACCEPTED.into_response()
                    }
                }))).await.unwrap();
            });
            let scope = fixture.catalog_scope();
            let record = run_sync_database(|| Ok(fixture.catalog.create(&scope, http_draft("held", &url))))?.unwrap();
            let permit = json!({"project_id":"project","app_id":"claude","record_id":"held",
                "connection_revision":record.connection_revision,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))});
            fixture.policy(&[permit.clone()])?;
            let headers = HeaderMap::from_iter([("x-agent-apps-token".parse().unwrap(), HeaderValue::from_static(TOKEN))]);
            let start = |generation| network_probe::start_connection_checks(
                &fixture.jobs, database.clone(), Some(fixture.root.path().join("policy.json")),
                &headers, fixture.owner.scope.clone(), vec![record.record.id.clone()], generation,
            );
            let job = start(9).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(10), started.notified()).await?;
            fixture.policy(&[])?;
            release.add_permits(1);
            let snapshot = fixture.finish(&job).await?;
            let [DiagnosticResult::McpConnectionCheck {item, ..}] = snapshot.results.as_deref().unwrap() else {panic!("revision-bound result required")};
            assert_eq!(item.outcome, McpConnectionOutcome::Failed {code: McpFailureCode::PermissionDenied});
            fixture.policy(&[permit])?;
            let cancelled = start(10).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(10), started.notified()).await?;
            fixture.jobs.cancel(&fixture.owner, &cancelled.task_id).unwrap();
            let snapshot = fixture.finish(&cancelled).await?;
            assert_eq!(snapshot.state, DiagnosticJobState::Cancelled {});
            assert_eq!(snapshot.results.iter().flatten().count(), 0);
            assert_eq!(fixture.jobs.snapshot(&fixture.owner, &job.task_id).unwrap().job.task_id, job.task_id);
            server.abort(); let _ = server.await;
            Ok(())
        })
    })
}
