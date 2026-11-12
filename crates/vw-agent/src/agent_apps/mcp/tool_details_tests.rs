//! Real SQLite and loopback discovery through the production authorized job executor.
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
use vw_api_types::{agent_apps_diagnostics::DiagnosticResult, mcp_management::*};

fn details_policy(fixture: &Fixture, permits: &[Value]) -> anyhow::Result<()> {
    fixture.policy(permits)?;
    let path = fixture.root.path().join("policy.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    policy["principals"][0]["grants"][0]["provider_config_read"] = json!(true);
    std::fs::write(path, serde_json::to_vec(&policy)?)?;
    Ok(())
}

#[test]
fn cc_mcp_t50_details_discover_real_specs_and_read_only_the_authorized_selection()
-> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database.clone())?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            let fail = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let failing = fail.clone();
            let server = tokio::spawn(async move {
                axum::serve(listener, Router::new().route("/mcp", post(move |Json(request): Json<Value>| {
                    let failing = failing.clone();
                    async move {
                    if request["method"] == "tools/list" && failing.load(std::sync::atomic::Ordering::SeqCst) {
                        return Json(json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32603,"message":"fixture-private-error-must-not-leak"}})).into_response();
                    }
                    let result = match request["method"].as_str() {
                        Some("initialize") => json!({"protocolVersion":"2025-03-26", "capabilities":{"tools":{}}, "serverInfo":{"name":"fixture","version":"1"}}),
                        Some("tools/list") => json!({"tools":[{"name":"search_docs", "title":"搜索文档🧪", "description":"真实发现🧪", "inputSchema":{"type":"object"}, "annotations":{"readOnlyHint":true}}]}),
                        _ => return axum::http::StatusCode::ACCEPTED.into_response(),
                    };
                    Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
                }}))).await.unwrap();
            });
            let scope = fixture.catalog_scope();
            let record = run_sync_database(|| Ok(fixture.catalog.create(&scope, http_draft("docs", &url))))?.unwrap();
            details_policy(&fixture,&[json!({"project_id":"project","app_id":"claude","record_id":"docs",
                "connection_revision":record.connection_revision,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))})])?;
            let headers = HeaderMap::from_iter([("x-agent-apps-token".parse().unwrap(), HeaderValue::from_static(TOKEN))]);
            let request = McpToolDetailsRequest { scope: scope.clone(), record: record.record.clone(), connection_revision: record.connection_revision, generation: 5 };
            let start = |request| network_probe::start_details(&fixture.jobs, database.clone(), Some(fixture.root.path().join("policy.json")), &headers, fixture.owner.scope.clone(), request);
            // A probe grant alone cannot expose persisted project selections.
            let policy_path = fixture.root.path().join("policy.json");
            let granted = std::fs::read(&policy_path)?;
            let mut probe_only: Value = serde_json::from_slice(&granted)?;
            probe_only["principals"][0]["grants"][0]["provider_config_read"] = json!(false);
            std::fs::write(&policy_path, serde_json::to_vec(&probe_only)?)?;
            assert_eq!(start(request.clone()), Err(McpHandshakeStartError::Access(axum::http::StatusCode::FORBIDDEN)));
            std::fs::write(&policy_path, granted)?;
            let job = start(request.clone()).unwrap();
            let snapshot = fixture.finish(&job).await?;
            let [DiagnosticResult::McpToolDetails { details }] = snapshot.results.as_deref().unwrap() else { panic!("real tool details required") };
            assert_eq!(details.request, request);
            assert_eq!(details.connection_error, None);
            assert_eq!(details.tools_error, None);
            assert_eq!(details.tools.len(), 1);
            assert_eq!(details.tools[0].identity.name, "search_docs");
            assert!(details.tools[0].read_only);
            assert_eq!(details.runtime, Some(McpRuntimeUsage::NotSelected));
            let selection = McpRuntimeSelection { project: fixture.owner.scope.project.clone(), record: record.record.clone() };
            run_sync_database(|| Ok(fixture.catalog.select_for_runtime(&selection.project, &scope, &selection)))?.unwrap();
            let job = start(request.clone()).unwrap();
            let snapshot = fixture.finish(&job).await?;
            let [DiagnosticResult::McpToolDetails { details }] = snapshot.results.as_deref().unwrap() else { panic!("details required") };
            assert_eq!(details.runtime, Some(McpRuntimeUsage::Selected { record: record.record.clone(), disabled:false, generation:0, current:true }));
            run_sync_database(|| Ok(fixture.catalog.disable_for_runtime(&selection.project, &scope, &selection)))?.unwrap();
            let job = start(request.clone()).unwrap();
            let snapshot = fixture.finish(&job).await?;
            let [DiagnosticResult::McpToolDetails { details }] = snapshot.results.as_deref().unwrap() else { panic!("details required") };
            assert_eq!(details.runtime, Some(McpRuntimeUsage::Selected { record: record.record.clone(), disabled:true, generation:1, current:true }));
            fail.store(true, std::sync::atomic::Ordering::SeqCst);
            let job = start(request.clone()).unwrap();
            let snapshot = fixture.finish(&job).await?;
            let [DiagnosticResult::McpToolDetails { details }] = snapshot.results.as_deref().unwrap() else { panic!("details required") };
            assert_eq!(details.connection_error, None);
            assert_eq!(details.tools_error, Some(McpFailureCode::ToolDiscoveryFailed));
            assert!(details.tools.is_empty());
            assert!(matches!(details.runtime, Some(McpRuntimeUsage::Selected { disabled:true,.. })));
            assert!(!serde_json::to_string(details)?.contains("fixture-private-error-must-not-leak"));
            let mut stale = request.clone(); stale.record.revision += 1;
            assert_eq!(start(stale), Err(McpHandshakeStartError::Record(McpFailureCode::StaleRevision)));
            let mut foreign = request.clone(); foreign.scope = McpCatalogScope::Tenant { scope: fixture.owner.scope.project.tenant_scope().clone() };
            assert_eq!(start(foreign), Err(McpHandshakeStartError::Record(McpFailureCode::PermissionDenied)));
            details_policy(&fixture,&[])?;
            assert!(matches!(start(request), Err(McpHandshakeStartError::Access(axum::http::StatusCode::FORBIDDEN))));
            server.abort(); let _ = server.await;
            Ok(())
        })
    })
}
#[test]
fn cc_mcp_t50_discovery_rechecks_revocation_and_cancel_releases_the_owned_job() -> anyhow::Result<()>
{
    with_database(|database| {
        let fixture = Fixture::new(database.clone())?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let started=std::sync::Arc::new(tokio::sync::Notify::new());
            let release=std::sync::Arc::new(tokio::sync::Semaphore::new(0));
            let ready=started.clone();let gate=release.clone();
            let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let url=format!("http://{}/mcp",listener.local_addr()?);
            let server=tokio::spawn(async move {
                axum::serve(listener,Router::new().route("/mcp",post(move |Json(request):Json<Value>|{
                    let ready=ready.clone();let gate=gate.clone();
                    async move {
                        let result=match request["method"].as_str(){
                            Some("initialize")=>json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}),
                            Some("tools/list")=>{ready.notify_one();gate.acquire().await.unwrap().forget();json!({"tools":[]})},
                            _=>return axum::http::StatusCode::ACCEPTED.into_response(),
                        };
                        Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
                    }
                }))).await.unwrap();
            });
            let scope=fixture.catalog_scope();
            let record=run_sync_database(||Ok(fixture.catalog.create(&scope,http_draft("held",&url))))?.unwrap();
            let permit=json!({"project_id":"project","app_id":"claude","record_id":"held","connection_revision":record.connection_revision,"endpoint_sha256":hex::encode(Sha256::digest(url.as_bytes()))});
            details_policy(&fixture,&[permit.clone()])?;
            let headers=HeaderMap::from_iter([("x-agent-apps-token".parse().unwrap(),HeaderValue::from_static(TOKEN))]);
            let start=||network_probe::start_details(&fixture.jobs,database.clone(),Some(fixture.root.path().join("policy.json")),&headers,fixture.owner.scope.clone(),McpToolDetailsRequest {
                scope:scope.clone(),record:record.record.clone(),connection_revision:record.connection_revision,generation:1,
            }).unwrap();
            let job=start();
            tokio::time::timeout(std::time::Duration::from_secs(10),started.notified()).await?;
            // Revoke only metadata visibility while keeping the MCP probe permit intact.
            let policy_path = fixture.root.path().join("policy.json");
            let mut policy: Value = serde_json::from_slice(&std::fs::read(&policy_path)?)?;
            policy["principals"][0]["grants"][0]["provider_config_read"] = json!(false);
            std::fs::write(&policy_path, serde_json::to_vec(&policy)?)?;
            release.add_permits(1);
            let snapshot=fixture.finish(&job).await?;
            let [DiagnosticResult::McpToolDetails {details}]=snapshot.results.as_deref().unwrap() else {panic!("details required")};
            assert_eq!(details.connection_error,Some(McpFailureCode::PermissionDenied));assert!(details.tools.is_empty());assert!(details.runtime.is_none());
            details_policy(&fixture,&[permit])?;
            let job=start();tokio::time::timeout(std::time::Duration::from_secs(10),started.notified()).await?;
            fixture.jobs.cancel(&fixture.owner,&job.task_id).unwrap();
            let snapshot=fixture.finish(&job).await?;
            assert_eq!(snapshot.state,vw_api_types::agent_apps_diagnostics::DiagnosticJobState::Cancelled {});
            assert_eq!(snapshot.results.iter().flatten().count(),0);
            server.abort();let _=server.await;Ok(())
        })
    })
}
